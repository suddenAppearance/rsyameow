//! Thin HTTP layer around `ureq`.

use crate::{Error, Result};
use serde::de::DeserializeOwned;
use serde_json::Value;
use std::io::Read;
use std::time::Duration;
use ureq::Agent;

/// Header expected by the Yandex Music backend.
const MUSIC_CLIENT_HEADER: &str = "YandexMusicAndroid/24023621";
const USER_AGENT: &str = concat!("rsyameow/", env!("CARGO_PKG_VERSION"));

pub(crate) struct Transport {
    agent: Agent,
    timeout: Duration,
}

impl Transport {
    pub(crate) fn new(timeout: Duration) -> Self {
        let agent = ureq::AgentBuilder::new()
            .timeout_connect(Duration::from_secs(10))
            .timeout_read(timeout)
            .timeout_write(Duration::from_secs(10))
            .build();
        Self { agent, timeout }
    }

    fn request(
        &self,
        method: &str,
        url: &str,
        query: &[(&str, String)],
        token: Option<&str>,
    ) -> ureq::Request {
        self.request_with_client(method, url, query, token, MUSIC_CLIENT_HEADER)
    }

    fn request_with_client(
        &self,
        method: &str,
        url: &str,
        query: &[(&str, String)],
        token: Option<&str>,
        client: &str,
    ) -> ureq::Request {
        let mut request = match method {
            "POST" => self.agent.post(url),
            _ => self.agent.get(url),
        }
        .set("X-Yandex-Music-Client", client)
        .set("User-Agent", USER_AGENT)
        .set("Accept-Language", "ru")
        .set("Accept", "application/json, text/xml;q=0.9, */*;q=0.8");

        if let Some(token) = token {
            request = request.set("Authorization", &format!("OAuth {token}"));
        }

        for (key, value) in query {
            request = request.query(key, value);
        }

        request
    }

    /// GET a JSON document whose payload lives in the `result` field.
    pub(crate) fn get<T: DeserializeOwned>(
        &self,
        url: &str,
        query: &[(&str, String)],
        token: Option<&str>,
    ) -> Result<T> {
        let request = self.request("GET", url, query, token).timeout(self.timeout);
        let response = request.call().map_err(map_ureq_error)?;
        let body = read_body(response)?;
        decode_api_envelope(&body)
    }

    /// GET a JSON document with a caller-provided `X-Yandex-Music-Client`
    /// header. Required by endpoints whose signature is tied to the client id.
    pub(crate) fn get_with_client<T: DeserializeOwned>(
        &self,
        url: &str,
        query: &[(&str, String)],
        token: Option<&str>,
        client: &str,
    ) -> Result<T> {
        let request = self
            .request_with_client("GET", url, query, token, client)
            .timeout(self.timeout);
        let response = request.call().map_err(map_ureq_error)?;
        let body = read_body(response)?;
        decode_api_envelope(&body)
    }

    /// POST a form and decode a JSON document whose payload lives in `result`.
    pub(crate) fn post_form<T: DeserializeOwned>(
        &self,
        url: &str,
        form: &[(&str, String)],
        token: Option<&str>,
    ) -> Result<T> {
        let borrowed: Vec<(&str, &str)> = form
            .iter()
            .map(|(key, value)| (*key, value.as_str()))
            .collect();
        let request = self.request("POST", url, &[], token).timeout(self.timeout);
        let response = request.send_form(&borrowed).map_err(map_ureq_error)?;
        let body = read_body(response)?;
        decode_api_envelope(&body)
    }

    /// POST a form and decode a flat JSON document (used by OAuth endpoints).
    pub(crate) fn post_form_flat<T: DeserializeOwned>(
        &self,
        url: &str,
        form: &[(&str, String)],
    ) -> Result<T> {
        let borrowed: Vec<(&str, &str)> = form
            .iter()
            .map(|(key, value)| (*key, value.as_str()))
            .collect();
        let request = self.request("POST", url, &[], None).timeout(self.timeout);
        let response = request.send_form(&borrowed).map_err(map_ureq_error)?;
        let body = read_body(response)?;
        serde_json::from_slice(&body)
            .map_err(|error| Error::Decode(format!("{error}: {}", body_preview(&body))))
    }

    /// GET a text document (the download-info XML).
    pub(crate) fn get_text(&self, url: &str, token: Option<&str>) -> Result<String> {
        let request = self.request("GET", url, &[], token).timeout(self.timeout);
        let response = request.call().map_err(map_ureq_error)?;
        let body = read_body(response)?;
        String::from_utf8(body)
            .map_err(|error| Error::Decode(format!("response is not valid UTF-8: {error}")))
    }

    /// GET binary data (audio bytes or a cover image).
    pub(crate) fn get_bytes(
        &self,
        url: &str,
        token: Option<&str>,
        timeout: Duration,
    ) -> Result<Vec<u8>> {
        let request = self.request("GET", url, &[], token).timeout(timeout);
        let response = request.call().map_err(map_ureq_error)?;
        let mut reader = response.into_reader();
        let mut bytes = Vec::new();
        reader
            .read_to_end(&mut bytes)
            .map_err(|error| Error::Network(format!("failed to read response body: {error}")))?;
        Ok(bytes)
    }
}

fn read_body(response: ureq::Response) -> Result<Vec<u8>> {
    let mut reader = response.into_reader();
    let mut bytes = Vec::new();
    reader
        .read_to_end(&mut bytes)
        .map_err(|error| Error::Network(format!("failed to read response body: {error}")))?;
    Ok(bytes)
}

fn decode_api_envelope<T: DeserializeOwned>(body: &[u8]) -> Result<T> {
    let value: Value = serde_json::from_slice(body)
        .map_err(|error| Error::Decode(format!("{error}: {}", body_preview(body))))?;

    let payload = match value {
        Value::Object(mut object) => match object.remove("result") {
            Some(result) => result,
            None => Value::Object(object),
        },
        other => other,
    };

    serde_json::from_value(payload)
        .map_err(|error| Error::Decode(format!("{error}: {}", body_preview(body))))
}

fn map_ureq_error(error: ureq::Error) -> Error {
    match error {
        ureq::Error::Status(status, response) => {
            let body = response
                .into_string()
                .unwrap_or_else(|_| String::from("<unreadable response body>"));
            let message = extract_error_message(&body);
            map_status(status, message)
        }
        ureq::Error::Transport(transport) => {
            let message = transport.message().unwrap_or("transport error").to_string();
            if message.to_ascii_lowercase().contains("timed out") {
                Error::Timeout
            } else {
                Error::Network(message)
            }
        }
    }
}

fn map_status(status: u16, message: String) -> Error {
    match status {
        400 => Error::BadRequest(message),
        401 | 403 => Error::Unauthorized(message),
        404 => Error::NotFound(message),
        _ => Error::Api { status, message },
    }
}

/// Extract a human-readable message from both flat OAuth errors and the nested
/// error objects returned by the music API.
fn extract_error_message(body: &str) -> String {
    if let Ok(value) = serde_json::from_str::<Value>(body) {
        let mut parts = Vec::new();

        if let Some(error) = value.get("error") {
            match error {
                Value::String(code) => parts.push(code.clone()),
                Value::Object(object) => {
                    if let Some(name) = object.get("name").and_then(Value::as_str) {
                        parts.push(name.to_string());
                    }
                    if let Some(message) = object.get("message").and_then(Value::as_str) {
                        parts.push(message.to_string());
                    }
                }
                _ => {}
            }
        }

        for key in ["error_description", "message"] {
            if let Some(message) = value.get(key).and_then(Value::as_str) {
                parts.push(message.to_string());
            }
        }

        if let Some(errors) = value.get("errors").and_then(Value::as_array) {
            for error in errors {
                if let Some(message) = error.get("message").and_then(Value::as_str) {
                    parts.push(message.to_string());
                }
            }
        }

        parts.retain(|part| !part.trim().is_empty());
        if !parts.is_empty() {
            return parts.join(" ");
        }
    }

    let trimmed = body.trim();
    if trimmed.is_empty() {
        return "empty response body".to_string();
    }
    trimmed.chars().take(300).collect()
}

fn body_preview(body: &[u8]) -> String {
    String::from_utf8_lossy(&body[..body.len().min(300)]).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_flat_oauth_error() {
        let message = extract_error_message(
            r#"{"error":"authorization_pending","error_description":"User code not confirmed"}"#,
        );
        assert!(message.contains("authorization_pending"));
        assert!(message.contains("User code not confirmed"));
    }

    #[test]
    fn extracts_nested_api_error() {
        let message =
            extract_error_message(r#"{"error":{"name":"not-found","message":"Track not found"}}"#);
        assert!(message.contains("not-found"));
        assert!(message.contains("Track not found"));
    }

    #[test]
    fn extracts_plain_text_error() {
        assert_eq!(extract_error_message("Bad Gateway"), "Bad Gateway");
    }

    #[test]
    fn decodes_result_envelope() {
        #[derive(serde::Deserialize, Debug, PartialEq)]
        struct Payload {
            value: i32,
        }

        let payload: Payload = decode_api_envelope(br#"{"result":{"value":42}}"#).unwrap();
        assert_eq!(payload, Payload { value: 42 });
    }
}
