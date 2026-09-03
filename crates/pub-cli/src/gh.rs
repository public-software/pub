//! GitHub through the `gh` CLI — its login, its host configuration, its rate-limit handling.
//!
//! Every call is one `gh api` process; bodies go in as JSON on stdin, replies come back as JSON.

use std::io::Write as _;
use std::process::{Command, Stdio};

use serde_json::Value;

/// What one `gh api` call yielded.
#[derive(Debug)]
pub enum Reply {
    /// A 2xx answer; `Value::Null` when the body was empty.
    Ok(Value),
    /// A 404.
    NotFound,
}

/// Run `gh api -X <method> <path>`, sending `body` as JSON when given.
pub fn api(method: &str, path: &str, body: Option<&Value>) -> Result<Reply, String> {
    let mut cmd = Command::new("gh");
    cmd.args(["api", "-X", method, path]);
    if body.is_some() {
        cmd.args(["--input", "-"]);
    }
    cmd.stdin(if body.is_some() {
        Stdio::piped()
    } else {
        Stdio::null()
    })
    .stdout(Stdio::piped())
    .stderr(Stdio::piped());
    let mut child = cmd
        .spawn()
        .map_err(|e| format!("gh: {e} (is the GitHub CLI installed and logged in?)"))?;
    if let (Some(body), Some(mut stdin)) = (body, child.stdin.take()) {
        stdin
            .write_all(body.to_string().as_bytes())
            .map_err(|e| format!("gh api {method} {path}: {e}"))?;
    }
    let out = child
        .wait_with_output()
        .map_err(|e| format!("gh api {method} {path}: {e}"))?;
    if out.status.success() {
        let text = String::from_utf8_lossy(&out.stdout);
        if text.trim().is_empty() {
            return Ok(Reply::Ok(Value::Null));
        }
        return serde_json::from_str(&text)
            .map(Reply::Ok)
            .map_err(|e| format!("gh api {method} {path}: bad JSON: {e}"));
    }
    let err = String::from_utf8_lossy(&out.stderr);
    if err.contains("HTTP 404") {
        return Ok(Reply::NotFound);
    }
    Err(format!("gh api {method} {path}: {}", err.trim()))
}

/// `GET path`; `None` on 404.
pub fn get(path: &str) -> Result<Option<Value>, String> {
    match api("GET", path, None)? {
        Reply::Ok(value) => Ok(Some(value)),
        Reply::NotFound => Ok(None),
    }
}

/// Percent-encode one path segment (RFC 3986 unreserved characters pass through).
pub fn segment(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    for byte in raw.bytes() {
        if byte.is_ascii_alphanumeric() || b"-._~".contains(&byte) {
            out.push(byte as char);
        } else {
            out.push_str(&format!("%{byte:02X}"));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn segments_are_encoded_for_paths() {
        assert_eq!(segment("kind/bug"), "kind%2Fbug");
        assert_eq!(segment("good first issue"), "good%20first%20issue");
        assert_eq!(segment("prio-p0.x_y~"), "prio-p0.x_y~");
    }
}
