use std::io::BufReader;
use std::net::TcpListener;
use std::sync::{Arc, Mutex, mpsc};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use platform_ports::{InsertError, TextInserter};
use serde_json::{Map, json};
use speech_application::ports::{AudioInput, CaptureError, TranscriptionError};
use speech_application::{DictationError, DictationOutcome, DictationService, PressOutcome};
use speech_domain::AudioClip;

use crate::wyoming::codec::{Event, read_event, write_event};
use crate::{WyomingSttConfig, WyomingTranscriber};

struct Microphone {
    rate: u32,
    samples: usize,
}

impl AudioInput for Microphone {
    fn start(&self) -> Result<(), CaptureError> {
        Ok(())
    }

    fn stop(&self) -> Result<AudioClip, CaptureError> {
        Ok(AudioClip::from_samples(self.rate, vec![0.25; self.samples]).unwrap())
    }

    fn level(&self) -> f32 {
        0.25
    }
}

#[derive(Default)]
struct FocusedApp {
    typed: Mutex<Vec<String>>,
}

impl TextInserter for FocusedApp {
    fn insert(&self, text: &str) -> Result<(), InsertError> {
        self.typed.lock().unwrap().push(text.to_owned());
        Ok(())
    }
}

struct Heard {
    language: Option<String>,
    rate: u64,
    audio_bytes: usize,
}

impl Heard {
    fn note(&mut self, event: &Event) {
        let data = |key: &str| event.data.get(key).cloned().unwrap_or_default();
        match event.kind.as_str() {
            "transcribe" => self.language = data("language").as_str().map(str::to_owned),
            "audio-start" => self.rate = data("rate").as_u64().unwrap_or_default(),
            _ => self.audio_bytes += event.payload.len(),
        }
    }
}

fn joined<T: Send + 'static>(server: JoinHandle<T>) -> T {
    let (done, finished) = mpsc::channel();
    thread::spawn(move || done.send(server.join().unwrap()));
    finished
        .recv_timeout(Duration::from_secs(5))
        .expect("the client never finished talking to the server")
}

fn whisper(transcript: &'static str) -> (u16, JoinHandle<Heard>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let server = thread::spawn(move || {
        let (stream, _) = listener.accept().unwrap();
        let mut reader = BufReader::new(stream.try_clone().unwrap());
        let mut heard = Heard {
            language: None,
            rate: 0,
            audio_bytes: 0,
        };
        let events = std::iter::from_fn(|| read_event(&mut reader).unwrap())
            .take_while(|event| event.kind != "audio-stop");
        for event in events {
            heard.note(&event);
        }
        let reply = Event {
            kind: "transcript".into(),
            data: json!({"text": transcript})
                .as_object()
                .cloned()
                .unwrap_or_else(Map::new),
            payload: Vec::new(),
        };
        write_event(&mut &stream, &reply).unwrap();
        heard
    });
    (port, server)
}

fn service(port: u16, microphone: Microphone, app: &Arc<FocusedApp>) -> DictationService {
    let config = WyomingSttConfig {
        host: "127.0.0.1".into(),
        port,
        language: Some("en".into()),
        connect_timeout: Duration::from_secs(2),
        io_timeout: Duration::from_secs(2),
    };
    let inserter: Arc<dyn TextInserter> = Arc::<FocusedApp>::clone(app);
    DictationService::new(
        Arc::new(microphone),
        Arc::new(WyomingTranscriber::new(config)),
        inserter,
    )
}

#[test]
fn hold_speak_release_types_what_whisper_heard_into_the_focused_app() {
    let (port, server) = whisper(" Schedule the review for Friday. ");
    let app = Arc::new(FocusedApp::default());
    let microphone = Microphone {
        rate: 48_000,
        samples: 48_000,
    };
    let service = service(port, microphone, &app);
    assert_eq!(service.press(), Ok(PressOutcome::Listening));
    let utterance = service.release().unwrap();
    assert_eq!(
        service.finish(utterance),
        Ok(DictationOutcome::Inserted { chars: 31 })
    );
    assert_eq!(
        *app.typed.lock().unwrap(),
        ["Schedule the review for Friday. "]
    );
    assert_one_second_in_english(&joined(server));
}

fn assert_one_second_in_english(heard: &Heard) {
    assert_eq!(heard.language.as_deref(), Some("en"));
    assert_eq!(heard.rate, 16_000);
    assert_eq!(heard.audio_bytes, 16_000 * 2);
}

#[test]
fn an_offline_whisper_is_reported_with_its_address_and_nothing_is_typed() {
    let port = TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port();
    let app = Arc::new(FocusedApp::default());
    let microphone = Microphone {
        rate: 16_000,
        samples: 16_000,
    };
    let service = service(port, microphone, &app);
    service.press().unwrap();
    let error = service.finish(service.release().unwrap()).unwrap_err();
    assert!(
        matches!(
            &error,
            DictationError::Transcription(TranscriptionError::Unreachable { address, .. })
                if *address == format!("127.0.0.1:{port}")
        ),
        "{error:?}"
    );
    assert!(app.typed.lock().unwrap().is_empty(), "nothing was typed");
}
