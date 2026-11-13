//! Composition root. The only crate that knows which adapter backs which port.

use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::Arc;

use axum::Router;
use tasks_application::TaskService;
use tasks_infrastructure::{InMemoryTaskRepository, UuidGenerator};
use thiserror::Error;

/// Port used when `PORT` is unset.
pub const DEFAULT_PORT: u16 = 3000;

/// Host used when `HOST` is unset. Set `HOST=0.0.0.0` inside containers.
pub const DEFAULT_HOST: IpAddr = IpAddr::V4(Ipv4Addr::LOCALHOST);

/// Invalid startup configuration.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ConfigError {
    /// `PORT` is not a number from 0 to 65535.
    #[error("PORT must be a number from 0 to 65535, got {0:?}")]
    InvalidPort(String),
    /// `HOST` is not an IP address.
    #[error("HOST must be an IP address, got {0:?}")]
    InvalidHost(String),
}

/// The full application with in-memory storage and UUID ids.
pub fn router() -> Router {
    let service = TaskService::new(
        Arc::new(InMemoryTaskRepository::default()),
        Arc::new(UuidGenerator),
    );
    http_api::router(service)
}

/// The address to listen on, from the raw `HOST` and `PORT` values.
///
/// # Errors
///
/// [`ConfigError`] if either value is set but does not parse.
pub fn listen_addr(host: Option<&str>, port: Option<&str>) -> Result<SocketAddr, ConfigError> {
    let ip = host.map_or(Ok(DEFAULT_HOST), |raw| {
        raw.parse()
            .map_err(|_| ConfigError::InvalidHost(raw.to_owned()))
    })?;
    let port = port.map_or(Ok(DEFAULT_PORT), |raw| {
        raw.parse()
            .map_err(|_| ConfigError::InvalidPort(raw.to_owned()))
    })?;
    Ok(SocketAddr::new(ip, port))
}

#[cfg(test)]
mod tests {
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use tower::ServiceExt;

    use super::*;

    #[test]
    fn defaults_to_localhost_3000() {
        assert_eq!(
            listen_addr(None, None),
            Ok(SocketAddr::from(([127, 0, 0, 1], 3000)))
        );
    }

    #[test]
    fn reads_host_and_port() {
        assert_eq!(
            listen_addr(Some("0.0.0.0"), Some("8080")),
            Ok(SocketAddr::from(([0, 0, 0, 0], 8080)))
        );
    }

    #[test]
    fn rejects_bad_values() {
        assert_eq!(
            listen_addr(None, Some("http")),
            Err(ConfigError::InvalidPort("http".into()))
        );
        assert_eq!(
            listen_addr(None, Some("70000")),
            Err(ConfigError::InvalidPort("70000".into()))
        );
        assert_eq!(
            listen_addr(Some("localhost"), None),
            Err(ConfigError::InvalidHost("localhost".into()))
        );
    }

    #[test]
    fn error_messages_name_the_variable() {
        assert_eq!(
            ConfigError::InvalidPort("x".into()).to_string(),
            r#"PORT must be a number from 0 to 65535, got "x""#
        );
        assert_eq!(
            ConfigError::InvalidHost("y".into()).to_string(),
            r#"HOST must be an IP address, got "y""#
        );
    }

    #[tokio::test]
    async fn router_wires_real_adapters() {
        let request = Request::post("/tasks")
            .header("content-type", "application/json")
            .body(Body::from(r#"{"title":"wired"}"#))
            .unwrap();
        let response = router().oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::CREATED);
    }
}
