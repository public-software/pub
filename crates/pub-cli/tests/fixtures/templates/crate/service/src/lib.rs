//! `{{CRATE}}` — the `{{DAEMON_PREFIX}}-{{COMPONENT}}` service of [`{{REPO}}`](https://github.com/{{ORG}}/{{REPO}}).
//!
//! The library holds the service's logic so it is testable without a process; `src/main.rs` is the
//! thin daemon around it. The health check mirrors the `health` interface of `{{WIT_NAMESPACE}}:core`
//! (the `interfaces` repository): a host calls it on a schedule and on demand.
//!
//! ```
//! use {{CRATE_IDENT}}::{Health, Service};
//! assert_eq!(Service::default().health(), Health::Ok);
//! ```

#![forbid(unsafe_code)]

use std::fmt;

/// The three states every orchestrator understands (`{{WIT_NAMESPACE}}:core/health.state`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Health {
    /// Serving.
    Ok,
    /// Serving with reduced function; the detail says what is missing.
    Degraded(String),
    /// Not serving; the host may restart the service.
    Failing(String),
}

impl Health {
    /// The process exit code `--health` answers with: 0 for ok, 1 for degraded, 2 for failing.
    pub fn exit_code(&self) -> u8 {
        match self {
            Health::Ok => 0,
            Health::Degraded(_) => 1,
            Health::Failing(_) => 2,
        }
    }
}

impl fmt::Display for Health {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Health::Ok => f.write_str("ok"),
            Health::Degraded(detail) => write!(f, "degraded: {detail}"),
            Health::Failing(detail) => write!(f, "failing: {detail}"),
        }
    }
}

/// The service. Replace the fields with its real state.
#[derive(Debug, Default)]
pub struct Service {
    draining: bool,
}

impl Service {
    /// Answers the health check.
    pub fn health(&self) -> Health {
        if self.draining {
            Health::Degraded("draining".to_owned())
        } else {
            Health::Ok
        }
    }

    /// Stops accepting new work; work in flight finishes.
    pub fn drain(&mut self) {
        self.draining = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_fresh_service_is_ok() {
        assert_eq!(Service::default().health(), Health::Ok);
        assert_eq!(Health::Ok.exit_code(), 0);
    }

    #[test]
    fn a_draining_service_is_degraded_and_says_why() {
        let mut service = Service::default();
        service.drain();
        let health = service.health();
        assert_eq!(health, Health::Degraded("draining".to_owned()));
        assert_eq!(health.to_string(), "degraded: draining");
        assert_eq!(health.exit_code(), 1);
    }

    #[test]
    fn failing_carries_its_detail_and_exit_code() {
        let health = Health::Failing("no store".to_owned());
        assert_eq!(health.to_string(), "failing: no store");
        assert_eq!(health.exit_code(), 2);
    }
}
