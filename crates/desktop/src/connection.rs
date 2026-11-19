//! Watches whether the voice servers answer, for the tray and the settings window.

use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::sync::{Mutex, PoisonError};
use std::time::Duration;

use serde::Serialize;
use speech_application::ConnectionStatus;
use speech_infrastructure::check_servers;
use tauri::{AppHandle, Emitter, Manager, Runtime, State};

use crate::settings::SettingsState;
use crate::support;
use crate::tray;
use crate::windows::SETTINGS;

const EVENT: &str = "connection:status";
const INTERVAL: Duration = Duration::from_secs(30);

pub(crate) struct Connection {
    status: Mutex<ConnectionStatus>,
    poke: Mutex<Sender<()>>,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ConnectionPayload {
    kind: &'static str,
    summary: String,
}

impl From<&ConnectionStatus> for ConnectionPayload {
    fn from(status: &ConnectionStatus) -> Self {
        Self {
            kind: status.kind.id(),
            summary: status.summary.clone(),
        }
    }
}

impl Connection {
    pub(crate) fn new(host: &str) -> (Self, Receiver<()>) {
        let (poke, poked) = mpsc::channel();
        let connection = Self {
            status: Mutex::new(ConnectionStatus::checking(host)),
            poke: Mutex::new(poke),
        };
        (connection, poked)
    }

    /// Checks now, then every [`INTERVAL`] or whenever poked. Call once the connection is
    /// managed by `app`.
    pub(crate) fn watch<R: Runtime>(app: &AppHandle<R>, poked: Receiver<()>) {
        let watcher = app.clone();
        support::spawn("voice-server-watch", move || {
            check(&watcher);
            while poked.recv_timeout(INTERVAL) != Err(RecvTimeoutError::Disconnected) {
                check(&watcher);
            }
        });
    }

    pub(crate) fn status(&self) -> ConnectionStatus {
        self.status
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }

    pub(crate) fn check_soon(&self) {
        let sent = self
            .poke
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .send(());
        if sent.is_err() {
            tracing::warn!("the voice server watcher has stopped");
        }
    }

    fn record(&self, status: ConnectionStatus) -> bool {
        let mut current = self.status.lock().unwrap_or_else(PoisonError::into_inner);
        let changed = *current != status;
        *current = status;
        changed
    }
}

fn check<R: Runtime>(app: &AppHandle<R>) -> ConnectionStatus {
    let settings = app.state::<SettingsState>().current();
    let status = check_servers(&settings.voice_host, settings.stt_port, settings.tts_port);
    if app.state::<Connection>().record(status.clone()) {
        tracing::info!(%status, "voice servers");
        tray::show_status(app, &status);
        drop(app.emit_to(SETTINGS, EVENT, ConnectionPayload::from(&status)));
    }
    status
}

#[tauri::command]
#[expect(
    clippy::needless_pass_by_value,
    reason = "Tauri commands receive their arguments by value"
)]
pub(crate) fn connection_status(connection: State<'_, Connection>) -> ConnectionPayload {
    ConnectionPayload::from(&connection.status())
}

#[tauri::command(async)]
#[expect(
    clippy::needless_pass_by_value,
    reason = "Tauri commands receive their arguments by value"
)]
pub(crate) fn check_connection(app: AppHandle) -> ConnectionPayload {
    ConnectionPayload::from(&check(&app))
}
