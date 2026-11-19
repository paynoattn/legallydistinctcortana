use std::thread;
use std::time::Duration;

use speech_domain::ConnectionStatus;

use super::connection::{Server, connect};

/// How long each server gets to accept a connection during a check.
pub const PROBE_TIMEOUT: Duration = Duration::from_secs(2);

/// Whether something on `host` accepts a TCP connection on `port` within `timeout`.
#[must_use]
pub fn server_answers(host: &str, port: u16, timeout: Duration) -> bool {
    connect(&Server {
        host,
        port,
        connect_timeout: timeout,
        io_timeout: timeout,
    })
    .is_ok()
}

/// Checks the speech recognition server on `stt_port` and the voice server on `tts_port`
/// of `host` at the same time, each within [`PROBE_TIMEOUT`].
#[must_use]
pub fn check_servers(host: &str, stt_port: u16, tts_port: u16) -> ConnectionStatus {
    let (recognition, voice) = thread::scope(|scope| {
        let voice = scope.spawn(|| server_answers(host, tts_port, PROBE_TIMEOUT));
        let recognition = server_answers(host, stt_port, PROBE_TIMEOUT);
        (recognition, voice.join().unwrap_or(false))
    });
    ConnectionStatus::checked(host, recognition, voice)
}

#[cfg(test)]
mod tests {
    use std::net::TcpListener;

    use speech_domain::ConnectionKind;

    use super::*;

    fn listening() -> (TcpListener, u16) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        (listener, port)
    }

    fn closed_port() -> u16 {
        let (listener, port) = listening();
        drop(listener);
        port
    }

    #[test]
    fn a_listening_port_answers() {
        let (_listener, port) = listening();
        assert!(server_answers("127.0.0.1", port, PROBE_TIMEOUT), "open");
    }

    #[test]
    fn a_closed_port_or_unknown_host_does_not_answer() {
        assert!(
            !server_answers("127.0.0.1", closed_port(), PROBE_TIMEOUT),
            "closed"
        );
        assert!(
            !server_answers("no-such-host.invalid", 10_300, PROBE_TIMEOUT),
            "unknown host"
        );
    }

    #[test]
    fn checking_both_servers_reports_each() {
        let (_stt, stt_port) = listening();
        let (_tts, tts_port) = listening();
        let closed = closed_port();
        assert_eq!(
            check_servers("127.0.0.1", stt_port, tts_port).kind,
            ConnectionKind::Connected
        );
        assert_eq!(
            check_servers("127.0.0.1", stt_port, closed).summary,
            "Speech recognition on 127.0.0.1 answers, but the voice server doesn't."
        );
        assert_eq!(
            check_servers("127.0.0.1", closed, tts_port).summary,
            "The voice server on 127.0.0.1 answers, but speech recognition doesn't."
        );
        assert_eq!(
            check_servers("127.0.0.1", closed, closed).kind,
            ConnectionKind::Unreachable
        );
    }

    #[test]
    fn each_server_gets_two_seconds() {
        assert_eq!(PROBE_TIMEOUT, Duration::from_secs(2));
    }
}
