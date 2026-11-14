use std::io;

use cpal::Sample;
use serde_json::{Map, Value};
use speech_application::ports::{SpeechToText, TranscriptionError};
use speech_domain::AudioClip;

use super::codec::{CodecError, Event, read_event};
use super::connection::{self, Server};
use crate::WyomingSttConfig;

/// Sample rate Whisper listens at.
const WHISPER_RATE: u32 = 16_000;

/// Bytes of 16-bit mono audio per `audio-chunk`: 100 ms, as `voice.py` sends.
const CHUNK_BYTES: usize = 3_200;

/// Transcribes speech with a Wyoming speech-to-text server such as `wyoming-faster-whisper`.
///
/// One TCP connection per utterance: `transcribe`, `audio-start`, `audio-chunk`s of 16 kHz
/// 16-bit mono PCM, and `audio-stop` out; `transcript` back.
#[derive(Debug, Clone)]
pub struct WyomingTranscriber {
    config: WyomingSttConfig,
}

impl WyomingTranscriber {
    /// A transcriber for the server in `config`.
    #[must_use]
    pub const fn new(config: WyomingSttConfig) -> Self {
        Self { config }
    }

    fn requests(&self, speech: &AudioClip) -> Vec<Event> {
        let pcm = pcm16(&for_whisper(speech));
        let mut requests = vec![
            event("transcribe", self.transcribe_data()),
            event("audio-start", format()),
        ];
        requests.extend(pcm.chunks(CHUNK_BYTES).map(|chunk| Event {
            kind: "audio-chunk".into(),
            data: format(),
            payload: chunk.to_vec(),
        }));
        requests.push(event("audio-stop", Map::new()));
        requests
    }

    fn transcribe_data(&self) -> Map<String, Value> {
        let mut data = Map::new();
        if let Some(language) = &self.config.language {
            data.insert("language".into(), Value::from(language.as_str()));
        }
        data
    }

    fn unreachable(&self, error: &io::Error) -> TranscriptionError {
        TranscriptionError::Unreachable {
            address: self.config.address(),
            reason: error.to_string(),
        }
    }

    fn codec_error(&self, error: CodecError) -> TranscriptionError {
        match error {
            CodecError::Io(io) => self.unreachable(&io),
            other => TranscriptionError::Protocol(other.to_string()),
        }
    }
}

impl SpeechToText for WyomingTranscriber {
    fn transcribe(&self, speech: &AudioClip) -> Result<String, TranscriptionError> {
        let server = Server {
            host: &self.config.host,
            port: self.config.port,
            connect_timeout: self.config.connect_timeout,
            io_timeout: self.config.io_timeout,
        };
        let mut reader = connection::exchange(&server, self.requests(speech))
            .map_err(|error| self.unreachable(&error))?;
        while let Some(reply) = read_event(&mut reader).map_err(|error| self.codec_error(error))? {
            match reply.kind.as_str() {
                "transcript" => return Ok(text(&reply.data).unwrap_or_default().to_owned()),
                "error" => {
                    return Err(TranscriptionError::Server(
                        text(&reply.data).unwrap_or("unknown error").to_owned(),
                    ));
                }
                _ => {}
            }
        }
        Err(TranscriptionError::Protocol(
            "the server closed the connection before sending a transcript".into(),
        ))
    }
}

/// `speech` at 16 kHz. Faster recordings are smoothed first so that sound Whisper cannot hear
/// does not fold back into the speech band as noise.
fn for_whisper(speech: &AudioClip) -> AudioClip {
    let ratio = speech.sample_rate().div_ceil(WHISPER_RATE);
    speech
        .smoothed(u16::try_from(ratio).unwrap_or(u16::MAX))
        .resampled(WHISPER_RATE)
}

fn pcm16(clip: &AudioClip) -> Vec<u8> {
    clip.samples()
        .iter()
        .flat_map(|sample| i16::from_sample(*sample).to_le_bytes())
        .collect()
}

fn format() -> Map<String, Value> {
    let mut data = Map::new();
    data.insert("rate".into(), Value::from(WHISPER_RATE));
    data.insert("width".into(), Value::from(2));
    data.insert("channels".into(), Value::from(1));
    data
}

fn event(kind: &str, data: Map<String, Value>) -> Event {
    Event {
        kind: kind.into(),
        data,
        payload: Vec::new(),
    }
}

fn text(data: &Map<String, Value>) -> Option<&str> {
    data.get("text").and_then(Value::as_str)
}

#[cfg(test)]
mod tests {
    use std::io::BufReader;
    use std::net::TcpListener;
    use std::sync::mpsc;
    use std::thread::{self, JoinHandle};
    use std::time::Duration;

    use serde_json::json;

    use super::super::codec::write_event;
    use super::*;

    fn config(port: u16, language: Option<&str>) -> WyomingSttConfig {
        WyomingSttConfig {
            host: "127.0.0.1".into(),
            port,
            language: language.map(str::to_owned),
            connect_timeout: Duration::from_secs(2),
            io_timeout: Duration::from_secs(2),
        }
    }

    fn reply(kind: &str, data: &Value) -> Event {
        Event {
            kind: kind.into(),
            data: data.as_object().cloned().unwrap(),
            payload: Vec::new(),
        }
    }

    fn joined<T: Send + 'static>(server: JoinHandle<T>) -> T {
        let (done, finished) = mpsc::channel();
        thread::spawn(move || done.send(server.join().unwrap()));
        finished
            .recv_timeout(Duration::from_secs(5))
            .expect("the client never finished talking to the server")
    }

    fn whisper(replies: Vec<Event>) -> (u16, JoinHandle<Vec<Event>>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let server = thread::spawn(move || {
            let (stream, _) = listener.accept().unwrap();
            let mut reader = BufReader::new(stream.try_clone().unwrap());
            let mut received = Vec::new();
            while let Some(event) = read_event(&mut reader).unwrap() {
                let stop = event.kind == "audio-stop";
                received.push(event);
                if stop {
                    break;
                }
            }
            let mut writer = stream;
            for event in &replies {
                write_event(&mut writer, event).unwrap();
            }
            received
        });
        (port, server)
    }

    fn clip(rate: u32, samples: Vec<f32>) -> AudioClip {
        AudioClip::from_samples(rate, samples).unwrap()
    }

    fn transcribe(
        language: Option<&str>,
        speech: &AudioClip,
        replies: Vec<Event>,
    ) -> (Result<String, TranscriptionError>, Vec<Event>) {
        let (port, server) = whisper(replies);
        let result = WyomingTranscriber::new(config(port, language)).transcribe(speech);
        (result, joined(server))
    }

    fn kinds(events: &[Event]) -> Vec<&str> {
        events.iter().map(|event| event.kind.as_str()).collect()
    }

    fn quarter_second() -> (Result<String, TranscriptionError>, Vec<Event>) {
        transcribe(
            Some("en"),
            &clip(16_000, vec![0.5; 4_000]),
            vec![reply("transcript", &json!({"text": "Hello, world."}))],
        )
    }

    #[test]
    fn empty_audio_sends_start_and_stop_without_an_audio_chunk() {
        let (result, sent) = transcribe(
            None,
            &clip(48_000, vec![]),
            vec![reply("transcript", &json!({"text": ""}))],
        );
        assert_eq!(result, Ok(String::new()));
        assert_eq!(kinds(&sent), ["transcribe", "audio-start", "audio-stop"]);
        assert!(sent.iter().all(|event| event.payload.is_empty()));
    }

    #[test]
    fn exact_chunk_boundaries_preserve_sample_order_without_an_empty_tail() {
        let samples = [vec![0.25; 1_600], vec![-0.5; 1_600]].concat();
        let (_, sent) = transcribe(
            None,
            &clip(16_000, samples),
            vec![reply("transcript", &json!({"text": "hi"}))],
        );
        assert_eq!(
            kinds(&sent),
            [
                "transcribe",
                "audio-start",
                "audio-chunk",
                "audio-chunk",
                "audio-stop"
            ]
        );
        assert_eq!(sent[2].payload, [0x00, 0x20].repeat(1_600));
        assert_eq!(sent[3].payload, [0x00, 0xC0].repeat(1_600));
        let format = json!({"rate": 16000, "width": 2, "channels": 1});
        assert_eq!(sent[2].data, *format.as_object().unwrap());
        assert_eq!(sent[3].data, *format.as_object().unwrap());
    }

    #[test]
    fn one_sample_past_a_chunk_boundary_is_kept_in_its_own_chunk() {
        let mut samples = vec![0.0; 1_600];
        samples.push(-1.0);
        let sent = WyomingTranscriber::new(config(10_300, None)).requests(&clip(16_000, samples));
        assert_eq!(
            kinds(&sent),
            [
                "transcribe",
                "audio-start",
                "audio-chunk",
                "audio-chunk",
                "audio-stop"
            ]
        );
        assert_eq!(sent[2].payload, vec![0; 3_200]);
        assert_eq!(sent[3].payload, [0x00, 0x80]);
    }

    #[test]
    fn sends_the_language_and_the_audio_then_returns_the_transcript() {
        let (result, sent) = quarter_second();
        assert_eq!(result.unwrap(), "Hello, world.");
        assert_eq!(
            kinds(&sent),
            [
                "transcribe",
                "audio-start",
                "audio-chunk",
                "audio-chunk",
                "audio-chunk",
                "audio-stop"
            ]
        );
        assert_eq!(
            sent[0].data,
            *json!({"language": "en"}).as_object().unwrap()
        );
    }

    #[test]
    fn audio_is_sent_as_16_khz_mono_in_100_ms_chunks() {
        let (_, sent) = quarter_second();
        let format = json!({"rate": 16000, "width": 2, "channels": 1});
        assert_eq!(sent[1].data, *format.as_object().unwrap());
        assert_eq!(sent[2].data, *format.as_object().unwrap());
        assert_eq!(sent[5].data, Map::new());
        let lengths: Vec<usize> = sent[2..5].iter().map(|chunk| chunk.payload.len()).collect();
        assert_eq!(lengths, [3_200, 3_200, 1_600]);
        assert_eq!(sent[2].payload[..4], [0x00, 0x40, 0x00, 0x40]);
    }

    #[test]
    fn without_a_language_the_server_detects_it() {
        let speech = clip(16_000, vec![0.0; 10]);
        let (result, sent) = transcribe(
            None,
            &speech,
            vec![reply("transcript", &json!({"text": "Hallo"}))],
        );
        assert_eq!(result.unwrap(), "Hallo");
        assert_eq!(sent[0].data, Map::new());
    }

    #[test]
    fn encodes_samples_as_little_endian_16_bit_and_clips_at_full_scale() {
        let speech = clip(16_000, vec![0.25, -0.25, 1.0, -1.0, 2.0]);
        assert_eq!(
            pcm16(&speech),
            [0x00, 0x20, 0x00, 0xE0, 0xFF, 0x7F, 0x00, 0x80, 0xFF, 0x7F]
        );
    }

    #[test]
    fn audio_from_a_48_khz_microphone_is_smoothed_and_sent_at_16_khz() {
        let tone = [1.0, -0.5, -0.5].repeat(1_600);
        let (_, sent) = transcribe(
            Some("en"),
            &clip(48_000, tone),
            vec![reply("transcript", &json!({"text": ""}))],
        );
        let audio: Vec<u8> = sent
            .iter()
            .filter(|event| event.kind == "audio-chunk")
            .flat_map(|event| event.payload.clone())
            .collect();
        assert_eq!(audio.len(), 1_600 * 2);
        let loudest = audio
            .as_chunks::<2>()
            .0
            .iter()
            .skip(1)
            .map(|pair| i16::from_le_bytes(*pair).unsigned_abs())
            .max()
            .unwrap();
        assert!(loudest < 8, "a 16 kHz tone is filtered out, peak {loudest}");
    }

    #[test]
    fn whisper_hears_at_16_khz_after_smoothing_by_the_rate_ratio() {
        let unchanged = clip(16_000, vec![0.5, -0.5]);
        assert_eq!(for_whisper(&unchanged), unchanged);
        let slow = clip(8_000, vec![0.5, -0.5]);
        assert_eq!(for_whisper(&slow), slow.resampled(16_000));
        let fast = clip(44_100, vec![0.3, 0.6, 0.9, 0.0, -0.9, 0.3]);
        assert_eq!(for_whisper(&fast), fast.smoothed(3).resampled(16_000));
    }

    #[test]
    fn skips_other_events_until_the_transcript() {
        let speech = clip(16_000, vec![0.0; 10]);
        let (result, _) = transcribe(
            Some("en"),
            &speech,
            vec![
                reply("transcript-start", &json!({})),
                reply("transcript-chunk", &json!({"text": "Hel"})),
                reply("transcript", &json!({"text": "Hello"})),
            ],
        );
        assert_eq!(result.unwrap(), "Hello");
    }

    #[test]
    fn a_transcript_without_text_is_empty() {
        let speech = clip(16_000, vec![0.0; 10]);
        let (result, _) = transcribe(Some("en"), &speech, vec![reply("transcript", &json!({}))]);
        assert_eq!(result.unwrap(), "");
    }

    #[test]
    fn reports_server_errors() {
        let speech = clip(16_000, vec![0.0; 10]);
        let (result, _) = transcribe(
            Some("en"),
            &speech,
            vec![reply("error", &json!({"text": "model not loaded"}))],
        );
        assert_eq!(
            result,
            Err(TranscriptionError::Server("model not loaded".into()))
        );
        let (untold, _) = transcribe(Some("en"), &speech, vec![reply("error", &json!({}))]);
        assert_eq!(
            untold,
            Err(TranscriptionError::Server("unknown error".into()))
        );
    }

    #[test]
    fn a_server_that_hangs_up_without_a_transcript_is_a_protocol_error() {
        let speech = clip(16_000, vec![0.0; 10]);
        let (result, _) = transcribe(Some("en"), &speech, vec![reply("info", &json!({}))]);
        assert_eq!(
            result,
            Err(TranscriptionError::Protocol(
                "the server closed the connection before sending a transcript".into()
            ))
        );
    }

    #[test]
    fn a_malformed_reply_is_a_protocol_error() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let server = thread::spawn(move || {
            let (stream, _) = listener.accept().unwrap();
            let mut reader = BufReader::new(stream.try_clone().unwrap());
            while read_event(&mut reader)
                .unwrap()
                .is_some_and(|event| event.kind != "audio-stop")
            {}
            let mut writer = stream;
            io::Write::write_all(&mut writer, b"not json\n").unwrap();
        });
        let result = WyomingTranscriber::new(config(port, None)).transcribe(&clip(16_000, vec![]));
        joined(server);
        assert!(
            matches!(&result, Err(TranscriptionError::Protocol(message)) if message.starts_with("malformed event")),
            "{result:?}"
        );
    }

    #[test]
    fn a_silent_server_times_out_as_unreachable() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let server = thread::spawn(move || {
            let (stream, _) = listener.accept().unwrap();
            thread::sleep(Duration::from_millis(1_500));
            drop(stream);
        });
        let mut settings = config(port, None);
        settings.io_timeout = Duration::from_millis(1_000);
        let result = WyomingTranscriber::new(settings).transcribe(&clip(16_000, vec![0.0; 10]));
        joined(server);
        assert_eq!(
            result,
            Err(TranscriptionError::Unreachable {
                address: format!("127.0.0.1:{port}"),
                reason: "no complete answer within 1 s".into()
            })
        );
    }

    #[test]
    fn reports_an_unreachable_server_with_its_address() {
        let port = TcpListener::bind("127.0.0.1:0")
            .unwrap()
            .local_addr()
            .unwrap()
            .port();
        let result = WyomingTranscriber::new(config(port, None)).transcribe(&clip(16_000, vec![]));
        assert!(
            matches!(&result, Err(TranscriptionError::Unreachable { address, .. }) if *address == format!("127.0.0.1:{port}")),
            "{result:?}"
        );
    }
}
