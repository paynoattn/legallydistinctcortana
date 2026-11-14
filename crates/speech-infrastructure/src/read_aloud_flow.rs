use std::io::BufReader;
use std::net::TcpListener;
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use kernel::Mood;
use platform_ports::{SelectionError, SelectionReader};
use serde_json::{Map, Value, json};
use speech_application::ports::{AudioOutput, PlaybackError, SynthesisError};
use speech_application::{CancelFlag, ReadAloudError, ReadAloudService, ReadOutcome};
use speech_domain::AudioClip;

use crate::wyoming::codec::{Event, read_event, write_event};
use crate::{WyomingConfig, WyomingSynthesizer};

struct Selected(&'static str);

impl SelectionReader for Selected {
    fn read_selection(&self) -> Result<String, SelectionError> {
        Ok(self.0.to_owned())
    }
}

#[derive(Default)]
struct Speakers {
    clips: Mutex<Vec<(u32, usize)>>,
}

impl AudioOutput for Speakers {
    fn enqueue(&self, clip: AudioClip) -> Result<(), PlaybackError> {
        self.clips
            .lock()
            .unwrap()
            .push((clip.sample_rate(), clip.samples().len()));
        Ok(())
    }

    fn wait_until_idle(&self, _cancel: &CancelFlag) {}

    fn stop(&self) {}
}

fn event(kind: &str, data: &Value, payload: Vec<u8>) -> Event {
    Event {
        kind: kind.into(),
        data: data.as_object().cloned().unwrap_or_else(Map::new),
        payload,
    }
}

fn piper(connections: usize) -> (u16, JoinHandle<Vec<String>>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let server = thread::spawn(move || {
        let format = json!({"rate": 22050, "width": 2, "channels": 1});
        (0..connections)
            .map(|_| {
                let (stream, _) = listener.accept().unwrap();
                let request = read_event(&mut BufReader::new(stream.try_clone().unwrap()))
                    .unwrap()
                    .unwrap();
                let mut writer = stream;
                write_event(&mut writer, &event("audio-start", &format, vec![])).unwrap();
                write_event(
                    &mut writer,
                    &event("audio-chunk", &format, vec![0; 2_205 * 2]),
                )
                .unwrap();
                write_event(&mut writer, &event("audio-stop", &json!({}), vec![])).unwrap();
                request.data["text"].as_str().unwrap().to_owned()
            })
            .collect()
    });
    (port, server)
}

fn service(port: u16, selection: &'static str, speakers: &Arc<Speakers>) -> ReadAloudService {
    let config = WyomingConfig {
        host: "127.0.0.1".into(),
        port,
        voice: None,
        connect_timeout: Duration::from_secs(2),
        io_timeout: Duration::from_secs(2),
    };
    let output: Arc<dyn AudioOutput> = Arc::<Speakers>::clone(speakers);
    ReadAloudService::new(
        Arc::new(Selected(selection)),
        Arc::new(WyomingSynthesizer::new(config)),
        output,
    )
}

#[test]
fn reads_a_selection_sentence_by_sentence_through_a_wyoming_server() {
    let (port, server) = piper(2);
    let speakers = Arc::new(Speakers::default());
    let service = service(port, "Good morning.\nTwo emails need you.", &speakers);
    assert_eq!(service.capture_selection().unwrap().char_count, 34);
    assert_eq!(
        service.read_captured(Mood::Alert),
        Ok(ReadOutcome::Finished)
    );
    assert_eq!(
        server.join().unwrap(),
        ["Good morning.", "Two emails need you."]
    );
    let alert_pause = 22_050 * 160 / 1_000;
    assert_eq!(
        *speakers.clips.lock().unwrap(),
        [(22_050, 2_004 + alert_pause), (22_050, 2_004 + alert_pause)]
    );
}

#[test]
fn an_offline_voice_server_is_reported_with_its_address() {
    let port = TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port();
    let speakers = Arc::new(Speakers::default());
    let service = service(port, "Hello.", &speakers);
    service.capture_selection().unwrap();
    let error = service.read_captured(Mood::Neutral).unwrap_err();
    assert!(
        matches!(
            &error,
            ReadAloudError::Synthesis(SynthesisError::Unreachable { address, .. })
                if *address == format!("127.0.0.1:{port}")
        ),
        "{error:?}"
    );
    assert!(
        error
            .to_string()
            .starts_with(&format!("can't reach the voice server at 127.0.0.1:{port}")),
        "{error}"
    );
}
