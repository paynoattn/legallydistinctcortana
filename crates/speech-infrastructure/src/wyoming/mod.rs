pub(crate) mod codec;

use std::io::{self, BufReader, Read};
use std::net::{TcpStream, ToSocketAddrs};
use std::time::{Duration, Instant};

use serde_json::{Map, Value};
use speech_application::ports::{SpeechSynthesizer, SynthesisError};
use speech_domain::AudioClip;

use crate::WyomingConfig;
use codec::{CodecError, Event, read_event, write_event};

const PCM16_WIDTH: u64 = 2;

/// Most audio accepted for one sentence, whatever the format.
const MAX_AUDIO_BYTES: usize = 16 * 1024 * 1024;

/// Most audio accepted for one sentence, in seconds of playback. A 300-character sentence
/// takes well under a minute even at the slowest mood.
const MAX_SENTENCE_SECONDS: u64 = 120;

/// Sample rates a voice server may use. Anything else is refused before any audio is
/// decoded, because resampling it would allocate without bound.
const SAMPLE_RATES: std::ops::RangeInclusive<u32> = 8_000..=192_000;

/// Synthesizes speech with a Wyoming text-to-speech server such as `wyoming-piper`.
///
/// One TCP connection per sentence: `synthesize` out, `audio-start`, `audio-chunk`s, and
/// `audio-stop` back (16-bit PCM).
#[derive(Debug, Clone)]
pub struct WyomingSynthesizer {
    config: WyomingConfig,
}

impl WyomingSynthesizer {
    /// A synthesizer for the server in `config`.
    #[must_use]
    pub const fn new(config: WyomingConfig) -> Self {
        Self { config }
    }

    fn connect(&self) -> Result<TcpStream, SynthesisError> {
        let addresses = (self.config.host.as_str(), self.config.port)
            .to_socket_addrs()
            .map_err(|error| self.unreachable(&error))?;
        let mut last = io::Error::new(io::ErrorKind::NotFound, "the host name has no addresses");
        for address in addresses {
            match TcpStream::connect_timeout(&address, self.config.connect_timeout) {
                Ok(stream) => return self.with_timeouts(stream),
                Err(error) => last = error,
            }
        }
        Err(self.unreachable(&last))
    }

    fn with_timeouts(&self, stream: TcpStream) -> Result<TcpStream, SynthesisError> {
        let timeout = Some(self.config.io_timeout);
        stream
            .set_read_timeout(timeout)
            .and_then(|()| stream.set_write_timeout(timeout))
            .map_err(|error| self.unreachable(&error))?;
        Ok(stream)
    }

    fn request(&self, sentence: &str) -> Map<String, Value> {
        let mut data = Map::new();
        data.insert("text".into(), Value::from(sentence));
        if let Some(voice) = &self.config.voice {
            let mut name = Map::new();
            name.insert("name".into(), Value::from(voice.as_str()));
            data.insert("voice".into(), Value::Object(name));
        }
        data
    }

    fn unreachable(&self, error: &io::Error) -> SynthesisError {
        SynthesisError::Unreachable {
            address: self.config.address(),
            reason: error.to_string(),
        }
    }

    fn codec_error(&self, error: CodecError) -> SynthesisError {
        match error {
            CodecError::Io(io) => self.unreachable(&io),
            other => SynthesisError::Protocol(other.to_string()),
        }
    }
}

impl SpeechSynthesizer for WyomingSynthesizer {
    fn synthesize(&self, sentence: &str) -> Result<AudioClip, SynthesisError> {
        let stream = self.connect()?;
        let mut writer = stream
            .try_clone()
            .map_err(|error| self.unreachable(&error))?;
        let request = Event {
            kind: "synthesize".into(),
            data: self.request(sentence),
            payload: Vec::new(),
        };
        write_event(&mut writer, &request).map_err(|error| self.unreachable(&error))?;
        let mut reader = BufReader::new(DeadlineReader {
            stream,
            deadline: Instant::now() + self.config.io_timeout,
            limit: self.config.io_timeout,
        });
        let mut audio = AudioCollector::default();
        while let Some(event) = read_event(&mut reader).map_err(|error| self.codec_error(error))? {
            if audio.accept(event)? == Flow::Done {
                return audio.finish();
            }
        }
        Err(SynthesisError::Protocol(
            "the server closed the connection before audio-stop".into(),
        ))
    }
}

/// Reads from the socket, but never past `deadline`: every read gets only the time that is
/// left, so a server that drips bytes cannot stretch one sentence beyond `limit`.
struct DeadlineReader {
    stream: TcpStream,
    deadline: Instant,
    limit: Duration,
}

impl DeadlineReader {
    fn timed_out(&self) -> io::Error {
        io::Error::new(
            io::ErrorKind::TimedOut,
            format!("no complete answer within {} s", self.limit.as_secs()),
        )
    }
}

impl Read for DeadlineReader {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let left = self.deadline.saturating_duration_since(Instant::now());
        if left.is_zero() {
            return Err(self.timed_out());
        }
        self.stream.set_read_timeout(Some(left))?;
        self.stream.read(buf).map_err(|error| {
            if matches!(
                error.kind(),
                io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut
            ) {
                self.timed_out()
            } else {
                error
            }
        })
    }
}

#[derive(Debug, PartialEq, Eq)]
enum Flow {
    More,
    Done,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct AudioFormat {
    rate: u32,
    channels: u16,
}

#[derive(Debug, Default)]
struct AudioCollector {
    format: Option<AudioFormat>,
    pcm: Vec<u8>,
}

impl AudioCollector {
    fn accept(&mut self, event: Event) -> Result<Flow, SynthesisError> {
        match event.kind.as_str() {
            "audio-start" => self.format = Some(AudioFormat::from_data(&event.data)?),
            "audio-chunk" => self.add_chunk(event)?,
            "audio-stop" => return Ok(Flow::Done),
            "error" => return Err(SynthesisError::Server(error_text(&event.data))),
            _ => {}
        }
        Ok(Flow::More)
    }

    fn add_chunk(&mut self, event: Event) -> Result<(), SynthesisError> {
        if self.format.is_none() {
            self.format = Some(AudioFormat::from_data(&event.data)?);
        }
        let limit = self.format.map_or(MAX_AUDIO_BYTES, AudioFormat::byte_limit);
        if self.pcm.len() + event.payload.len() > limit {
            return Err(SynthesisError::Protocol(format!(
                "the audio for one sentence is over the {limit}-byte limit"
            )));
        }
        self.pcm.extend(event.payload);
        Ok(())
    }

    fn finish(self) -> Result<AudioClip, SynthesisError> {
        let format = self.format.ok_or_else(|| {
            SynthesisError::Protocol("audio-stop arrived before any audio format".into())
        })?;
        AudioClip::from_pcm16(format.rate, format.channels, &self.pcm)
            .map_err(|error| SynthesisError::Protocol(error.to_string()))
    }
}

impl AudioFormat {
    fn from_data(data: &Map<String, Value>) -> Result<Self, SynthesisError> {
        let width = field(data, "width")?;
        if width != PCM16_WIDTH {
            return Err(SynthesisError::Protocol(format!(
                "only 16-bit audio is supported, got {width} bytes per sample"
            )));
        }
        let rate = narrow(field(data, "rate")?, "rate")?;
        if !SAMPLE_RATES.contains(&rate) {
            return Err(SynthesisError::Protocol(format!(
                "audio rate {rate} Hz is outside {} to {} Hz",
                SAMPLE_RATES.start(),
                SAMPLE_RATES.end()
            )));
        }
        Ok(Self {
            rate,
            channels: narrow(field(data, "channels")?, "channels")?,
        })
    }

    fn byte_limit(self) -> usize {
        let per_second = u64::from(self.rate) * u64::from(self.channels) * PCM16_WIDTH;
        usize::try_from(per_second * MAX_SENTENCE_SECONDS)
            .map_or(MAX_AUDIO_BYTES, |bytes| bytes.min(MAX_AUDIO_BYTES))
    }
}

fn field(data: &Map<String, Value>, name: &str) -> Result<u64, SynthesisError> {
    data.get(name)
        .and_then(Value::as_u64)
        .ok_or_else(|| SynthesisError::Protocol(format!("audio format has no {name}")))
}

fn narrow<T: TryFrom<u64>>(value: u64, name: &str) -> Result<T, SynthesisError> {
    T::try_from(value)
        .map_err(|_| SynthesisError::Protocol(format!("audio {name} {value} is out of range")))
}

fn error_text(data: &Map<String, Value>) -> String {
    data.get("text")
        .and_then(Value::as_str)
        .unwrap_or("unknown error")
        .to_owned()
}

#[cfg(test)]
mod tests {
    use std::net::TcpListener;
    use std::thread::{self, JoinHandle};
    use std::time::Duration;

    use serde_json::json;

    use super::*;

    type Reply = (&'static str, Value, Vec<u8>);

    fn object(value: &Value) -> Map<String, Value> {
        value.as_object().cloned().unwrap()
    }

    fn config(port: u16, voice: Option<&str>) -> WyomingConfig {
        WyomingConfig {
            host: "127.0.0.1".into(),
            port,
            voice: voice.map(str::to_owned),
            connect_timeout: Duration::from_secs(2),
            io_timeout: Duration::from_secs(2),
        }
    }

    fn serve(replies: Vec<Reply>) -> (u16, JoinHandle<Event>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let server = thread::spawn(move || {
            let (stream, _) = listener.accept().unwrap();
            let mut reader = BufReader::new(stream.try_clone().unwrap());
            let request = read_event(&mut reader).unwrap().unwrap();
            let mut writer = stream;
            for (kind, data, payload) in replies {
                let reply = Event {
                    kind: kind.into(),
                    data: object(&data),
                    payload,
                };
                write_event(&mut writer, &reply).unwrap();
            }
            request
        });
        (port, server)
    }

    fn format() -> Value {
        json!({"rate": 22050, "width": 2, "channels": 1})
    }

    fn synthesize(replies: Vec<Reply>) -> Result<AudioClip, SynthesisError> {
        let (port, server) = serve(replies);
        let result = WyomingSynthesizer::new(config(port, None)).synthesize("Hello.");
        server.join().unwrap();
        result
    }

    #[test]
    fn sends_the_sentence_and_voice_and_decodes_the_audio() {
        let (port, server) = serve(vec![
            ("audio-start", format(), vec![]),
            ("audio-chunk", format(), vec![0x00, 0x40]),
            ("audio-chunk", format(), vec![0x00, 0xC0]),
            ("audio-stop", json!({}), vec![]),
        ]);
        let synth = WyomingSynthesizer::new(config(port, Some("en_US-amy-medium")));
        let clip = synth.synthesize("Hello there.").unwrap();
        let request = server.join().unwrap();
        assert_eq!(request.kind, "synthesize");
        assert_eq!(
            request.data,
            object(&json!({"text": "Hello there.", "voice": {"name": "en_US-amy-medium"}}))
        );
        assert_eq!(clip.sample_rate(), 22_050);
        assert_eq!(clip.samples(), [0.5, -0.5]);
    }

    #[test]
    fn omits_the_voice_when_none_is_configured() {
        let (port, server) = serve(vec![
            ("audio-chunk", format(), vec![0, 0]),
            ("audio-stop", json!({}), vec![]),
        ]);
        let clip = WyomingSynthesizer::new(config(port, None))
            .synthesize("Hi.")
            .unwrap();
        assert_eq!(server.join().unwrap().data, object(&json!({"text": "Hi."})));
        assert_eq!(clip.samples(), [0.0]);
    }

    #[test]
    fn ignores_unrelated_events() {
        let clip = synthesize(vec![
            ("info", json!({}), vec![]),
            ("audio-start", format(), vec![]),
            ("audio-stop", json!({}), vec![]),
        ])
        .unwrap();
        assert!(clip.samples().is_empty(), "no chunks means no samples");
    }

    #[test]
    fn reports_server_errors() {
        assert_eq!(
            synthesize(vec![("error", json!({"text": "voice not found"}), vec![])]),
            Err(SynthesisError::Server("voice not found".into()))
        );
        assert_eq!(
            synthesize(vec![("error", json!({}), vec![])]),
            Err(SynthesisError::Server("unknown error".into()))
        );
    }

    fn start(format: &Value) -> Vec<Reply> {
        vec![("audio-start", format.clone(), vec![])]
    }

    #[test]
    fn reports_protocol_problems() {
        let cases: Vec<(Vec<Reply>, &str)> = vec![
            (vec![], "the server closed the connection before audio-stop"),
            (
                vec![("audio-stop", json!({}), vec![])],
                "audio-stop arrived before any audio format",
            ),
            (
                start(&json!({"rate": 8000, "width": 4, "channels": 1})),
                "only 16-bit audio is supported, got 4 bytes per sample",
            ),
            (
                start(&json!({"width": 2, "channels": 1})),
                "audio format has no rate",
            ),
            (
                start(&json!({"rate": 22050, "width": 2, "channels": 70000})),
                "audio channels 70000 is out of range",
            ),
            (
                vec![
                    ("audio-start", format(), vec![]),
                    ("audio-chunk", format(), vec![1]),
                    ("audio-stop", json!({}), vec![]),
                ],
                "1 bytes is not a whole number of 16-bit frames of 1 channel(s)",
            ),
        ];
        for (replies, message) in cases {
            assert_eq!(
                synthesize(replies),
                Err(SynthesisError::Protocol(message.into()))
            );
        }
    }

    #[test]
    fn malformed_replies_are_protocol_errors() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut reader = BufReader::new(stream.try_clone().unwrap());
            read_event(&mut reader).unwrap();
            io::Write::write_all(&mut stream, b"not json\n").unwrap();
        });
        let result = WyomingSynthesizer::new(config(port, None)).synthesize("Hi.");
        server.join().unwrap();
        assert!(
            matches!(&result, Err(SynthesisError::Protocol(message)) if message.starts_with("malformed event")),
            "{result:?}"
        );
    }

    const CHUNK_BYTES: usize = 2 * 1024 * 1024;

    fn chunks_of(total: usize, audio: &Value) -> Vec<Reply> {
        let mut replies = Vec::new();
        let mut left = total;
        while left > 0 {
            let size = left.min(CHUNK_BYTES);
            replies.push(("audio-chunk", audio.clone(), vec![0; size]));
            left -= size;
        }
        replies
    }

    fn audio(rate: u32, channels: u16) -> Value {
        json!({"rate": rate, "width": 2, "channels": channels})
    }

    fn over_limit(limit: usize) -> Result<AudioClip, SynthesisError> {
        Err(SynthesisError::Protocol(format!(
            "the audio for one sentence is over the {limit}-byte limit"
        )))
    }

    fn accepts_exactly(total: usize, audio: &Value) {
        let mut replies = chunks_of(total, audio);
        replies.push(("audio-stop", json!({}), vec![]));
        let clip = synthesize(replies).unwrap();
        assert_eq!(clip.samples().len() * 2 * channels_of(audio), total);
    }

    fn channels_of(audio: &Value) -> usize {
        usize::try_from(audio["channels"].as_u64().unwrap()).unwrap()
    }

    #[test]
    fn caps_a_sentence_at_two_minutes_of_audio_at_22_khz_mono() {
        let cap = 22_050 * 2 * 120;
        assert_eq!(cap, 5_292_000);
        accepts_exactly(cap, &audio(22_050, 1));
        assert_eq!(
            synthesize(chunks_of(cap + 2, &audio(22_050, 1))),
            over_limit(cap)
        );
    }

    #[test]
    fn the_cap_scales_with_the_channel_count() {
        let cap = 8_000 * 2 * 2 * 120;
        assert_eq!(cap, 3_840_000);
        accepts_exactly(cap, &audio(8_000, 2));
        assert_eq!(
            synthesize(chunks_of(cap + 4, &audio(8_000, 2))),
            over_limit(cap)
        );
    }

    #[test]
    fn the_cap_never_passes_sixteen_mebibytes() {
        let cap = 16 * 1024 * 1024;
        assert_eq!(cap, MAX_AUDIO_BYTES);
        accepts_exactly(cap, &audio(192_000, 2));
        assert_eq!(
            synthesize(chunks_of(cap + 4, &audio(192_000, 2))),
            over_limit(cap)
        );
    }

    #[test]
    fn the_cap_follows_a_format_given_by_audio_start() {
        let mut replies = start(&audio(8_000, 1));
        replies.extend(chunks_of(1_920_000 + 2, &audio(8_000, 1)));
        assert_eq!(synthesize(replies), over_limit(1_920_000));
    }

    #[test]
    fn accepts_the_lowest_and_highest_voice_rates() {
        for rate in [8_000, 192_000] {
            let clip = synthesize(vec![
                ("audio-chunk", audio(rate, 1), vec![0, 0]),
                ("audio-stop", json!({}), vec![]),
            ])
            .unwrap();
            assert_eq!(clip.sample_rate(), rate);
        }
    }

    #[test]
    fn refuses_rates_outside_the_voice_range_before_decoding_any_audio() {
        for rate in [0, 1, 7_999, 192_001, 384_000, u32::MAX] {
            assert_eq!(
                synthesize(vec![("audio-start", audio(rate, 1), vec![])]),
                Err(SynthesisError::Protocol(format!(
                    "audio rate {rate} Hz is outside 8000 to 192000 Hz"
                ))),
                "rate {rate}"
            );
        }
    }

    #[test]
    fn a_rate_a_u32_cannot_hold_is_out_of_range() {
        assert_eq!(
            synthesize(vec![(
                "audio-start",
                audio_with_rate(5_000_000_000),
                vec![]
            )]),
            Err(SynthesisError::Protocol(
                "audio rate 5000000000 is out of range".into()
            ))
        );
    }

    fn audio_with_rate(rate: u64) -> Value {
        json!({"rate": rate, "width": 2, "channels": 1})
    }

    #[test]
    fn gives_up_when_a_payload_is_dripped_one_byte_at_a_time() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let server = thread::spawn(move || {
            let (stream, _) = listener.accept().unwrap();
            let mut reader = BufReader::new(stream.try_clone().unwrap());
            read_event(&mut reader).unwrap().unwrap();
            let mut writer = stream;
            let header = format!(
                "{{\"type\":\"audio-chunk\",\"data\":{},\"payload_length\":1000}}\n",
                format()
            );
            if io::Write::write_all(&mut writer, header.as_bytes()).is_err() {
                return;
            }
            for _ in 0..100 {
                if io::Write::write_all(&mut writer, &[0]).is_err() {
                    break;
                }
                thread::sleep(Duration::from_millis(100));
            }
        });
        let mut settings = config(port, None);
        settings.io_timeout = Duration::from_millis(1_100);
        let started = Instant::now();
        let result = WyomingSynthesizer::new(settings).synthesize("Hi.");
        let elapsed = started.elapsed();
        server.join().unwrap();
        assert_eq!(
            result,
            Err(SynthesisError::Unreachable {
                address: format!("127.0.0.1:{port}"),
                reason: "no complete answer within 1 s".into(),
            })
        );
        assert!(elapsed >= Duration::from_millis(1_000), "{elapsed:?}");
        assert!(elapsed < Duration::from_millis(2_500), "{elapsed:?}");
    }

    #[test]
    fn a_silent_server_times_out_with_the_same_message() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let server = thread::spawn(move || {
            let (stream, _) = listener.accept().unwrap();
            let mut reader = BufReader::new(stream.try_clone().unwrap());
            read_event(&mut reader).unwrap().unwrap();
            thread::sleep(Duration::from_millis(1_500));
        });
        let mut settings = config(port, None);
        settings.io_timeout = Duration::from_millis(1_100);
        let result = WyomingSynthesizer::new(settings).synthesize("Hi.");
        server.join().unwrap();
        assert_eq!(
            result,
            Err(SynthesisError::Unreachable {
                address: format!("127.0.0.1:{port}"),
                reason: "no complete answer within 1 s".into(),
            })
        );
    }

    #[test]
    fn gives_up_when_audio_keeps_coming_past_the_deadline() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let server = thread::spawn(move || {
            let (stream, _) = listener.accept().unwrap();
            let mut reader = BufReader::new(stream.try_clone().unwrap());
            read_event(&mut reader).unwrap().unwrap();
            let mut writer = stream;
            let chunk = Event {
                kind: "audio-chunk".into(),
                data: object(&format()),
                payload: vec![0, 0],
            };
            for _ in 0..100 {
                if write_event(&mut writer, &chunk).is_err() {
                    break;
                }
                thread::sleep(Duration::from_millis(100));
            }
        });
        let mut settings = config(port, None);
        settings.io_timeout = Duration::from_millis(1_100);
        let started = Instant::now();
        let result = WyomingSynthesizer::new(settings).synthesize("Hi.");
        let elapsed = started.elapsed();
        server.join().unwrap();
        assert_eq!(
            result,
            Err(SynthesisError::Unreachable {
                address: format!("127.0.0.1:{port}"),
                reason: "no complete answer within 1 s".into(),
            })
        );
        assert!(elapsed >= Duration::from_millis(1_100), "{elapsed:?}");
        assert!(elapsed < Duration::from_secs(5), "{elapsed:?}");
    }

    #[test]
    fn reports_an_unreachable_server_with_its_address() {
        let port = TcpListener::bind("127.0.0.1:0")
            .unwrap()
            .local_addr()
            .unwrap()
            .port();
        let result = WyomingSynthesizer::new(config(port, None)).synthesize("Hi.");
        assert!(
            matches!(&result, Err(SynthesisError::Unreachable { address, .. }) if *address == format!("127.0.0.1:{port}")),
            "{result:?}"
        );
    }

    #[test]
    fn an_unknown_host_is_unreachable() {
        let mut settings = config(1, None);
        settings.host = "host.invalid".into();
        let result = WyomingSynthesizer::new(settings).synthesize("Hi.");
        assert!(
            matches!(&result, Err(SynthesisError::Unreachable { address, .. }) if address == "host.invalid:1"),
            "{result:?}"
        );
    }
}
