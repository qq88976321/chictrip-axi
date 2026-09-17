//! HTTP client for the chicTrip API: the shared envelope, the apiStatus to
//! error mapping, and the 003 refresh-and-retry.

pub mod trips;
pub mod types;

use crate::auth::{self, AuthFile, Credentials, TokenSource};
use crate::error::{AxiError, ErrorCode};
use serde_json::Value;
use std::cell::RefCell;
use std::time::Duration;

pub const BASE_URL_ENV: &str = "CHICTRIP_AXI_BASE_URL";
pub const DEFAULT_BASE_URL: &str = "https://api.chictrip.com.tw/";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Method {
    Get,
    Post,
    Delete,
}

#[derive(Debug, Clone)]
pub struct Envelope {
    pub status: String,
    pub data: Value,
    pub message: Option<String>,
    pub request_id: Option<String>,
}

impl Envelope {
    pub fn is_ok(&self) -> bool {
        self.status == "001"
    }

    /// Only 001 carries a payload; anything else becomes the mapped error.
    pub fn into_data(self) -> Result<Value, AxiError> {
        if self.is_ok() {
            return Ok(self.data);
        }
        Err(self.into_error())
    }

    pub fn into_error(self) -> AxiError {
        let message = self
            .message
            .clone()
            .filter(|m| !m.is_empty())
            .unwrap_or_else(|| format!("chicTrip returned apiStatus {}", self.status));
        let code = match self.status.as_str() {
            "002" if message.contains("Reject Guest Member") => ErrorCode::AuthRequired,
            "002" if message.trim() == "404" => ErrorCode::NotFound,
            "003" => ErrorCode::AuthInvalid,
            "004" => ErrorCode::Conflict,
            "006" => ErrorCode::Forbidden,
            "011" => ErrorCode::NotFound,
            _ => ErrorCode::ApiError,
        };
        let error = AxiError::new(code, message).with_request_id(self.request_id);
        match code {
            ErrorCode::AuthRequired | ErrorCode::AuthInvalid => error.with_help([
                "Run `chictrip-axi auth set --from-json -` and paste the JSON copied from the browser (see README)",
                "Run `chictrip-axi auth status` to check what is configured",
            ]),
            _ => error,
        }
    }
}

pub struct Client {
    base_url: String,
    agent: ureq::Agent,
    timeout: Duration,
    creds: RefCell<Credentials>,
}

pub fn base_url() -> String {
    std::env::var(BASE_URL_ENV)
        .ok()
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| DEFAULT_BASE_URL.to_string())
}

impl Client {
    pub fn new(creds: Credentials, timeout_secs: u64) -> Self {
        let timeout = Duration::from_secs(timeout_secs);
        let config = ureq::Agent::config_builder()
            .timeout_global(Some(timeout))
            .http_status_as_error(false)
            .user_agent(concat!("chictrip-axi/", env!("CARGO_PKG_VERSION")))
            .build();
        Client {
            base_url: base_url(),
            agent: ureq::Agent::new_with_config(config),
            timeout,
            creds: RefCell::new(creds),
        }
    }

    pub fn credentials(&self) -> Credentials {
        self.creds.borrow().clone()
    }

    pub fn is_guest(&self) -> bool {
        self.creds.borrow().is_guest()
    }

    fn url(&self, path: &str) -> String {
        format!(
            "{}/{}",
            self.base_url.trim_end_matches('/'),
            path.trim_start_matches('/')
        )
    }

    pub fn get(&self, path: &str, query: &[(&str, String)]) -> Result<Value, AxiError> {
        self.call(Method::Get, path, query, &[], false)?.into_data()
    }

    pub fn get_envelope(&self, path: &str, query: &[(&str, String)]) -> Result<Envelope, AxiError> {
        self.call(Method::Get, path, query, &[], false)
    }

    pub fn post_form(&self, path: &str, form: &[(&str, String)]) -> Result<Value, AxiError> {
        self.call(Method::Post, path, &[], form, false)?.into_data()
    }

    /// The web app sends `language: zh-tw` (not `zhtw`) on the schedule
    /// mutations; mirror it rather than find out which one the server checks.
    pub fn post_form_zhtw(&self, path: &str, form: &[(&str, String)]) -> Result<Value, AxiError> {
        self.call(Method::Post, path, &[], form, true)?.into_data()
    }

    pub fn delete_form(&self, path: &str, form: &[(&str, String)]) -> Result<Value, AxiError> {
        self.call(Method::Delete, path, &[], form, false)?
            .into_data()
    }

    fn call(
        &self,
        method: Method,
        path: &str,
        query: &[(&str, String)],
        form: &[(&str, String)],
        zhtw: bool,
    ) -> Result<Envelope, AxiError> {
        let envelope = self.send(method, path, query, form, zhtw)?;
        if envelope.status != "003" || !self.refresh_token()? {
            return Ok(envelope);
        }
        self.send(method, path, query, form, zhtw)
    }

    /// Returns true when a fresh access token was stored, so the caller can
    /// replay the request. Only file-backed sessions are refreshable: a token
    /// handed in by flag or environment is not ours to replace.
    fn refresh_token(&self) -> Result<bool, AxiError> {
        let (refresh_token, member_id, path) = {
            let creds = self.creds.borrow();
            let stored = match (&creds.stored, creds.source) {
                (Some(stored), TokenSource::File) => stored.clone(),
                _ => return Ok(false),
            };
            if stored.refresh_token.is_empty() {
                return Ok(false);
            }
            (stored.refresh_token, stored.member_id, creds.path.clone())
        };
        let envelope = self.send(
            Method::Post,
            "Token/Refresh",
            &[],
            &[("refreshToken", refresh_token), ("memberId", member_id)],
            false,
        )?;
        if !envelope.is_ok() {
            return Ok(false);
        }
        let refreshed: AuthFile = match serde_json::from_value(envelope.data) {
            Ok(auth) => auth,
            Err(_) => return Ok(false),
        };
        if refreshed.access_token.is_empty() {
            return Ok(false);
        }
        auth::save_file(&path, &refreshed)?;
        let mut creds = self.creds.borrow_mut();
        creds.token = refreshed.access_token.clone();
        creds.stored = Some(refreshed);
        Ok(true)
    }

    fn send(
        &self,
        method: Method,
        path: &str,
        query: &[(&str, String)],
        form: &[(&str, String)],
        zhtw: bool,
    ) -> Result<Envelope, AxiError> {
        let url = self.url(path);
        let bearer = format!("Bearer {}", self.creds.borrow().token);
        let language = if zhtw { "zh-tw" } else { "zhtw" };
        let headers = [
            ("Authorization", bearer.as_str()),
            ("osType", "web"),
            ("language", language),
        ];
        let pairs = query.iter().map(|(k, v)| (*k, v.as_str()));
        let fields = form.iter().map(|(k, v)| (*k, v.as_str()));

        let result = match method {
            Method::Get => {
                let mut request = self.agent.get(&url).query_pairs(pairs);
                for (key, value) in headers {
                    request = request.header(key, value);
                }
                request.call()
            }
            Method::Post => {
                let mut request = self.agent.post(&url).query_pairs(pairs);
                for (key, value) in headers {
                    request = request.header(key, value);
                }
                request.send_form(fields)
            }
            Method::Delete => {
                let mut request = self.agent.delete(&url).query_pairs(pairs);
                for (key, value) in headers {
                    request = request.header(key, value);
                }
                request.force_send_body().send_form(fields)
            }
        };

        let mut response = result.map_err(|e| self.network_error(&url, e))?;
        let status = response.status().as_u16();
        let body = response
            .body_mut()
            .read_to_string()
            .map_err(|e| self.network_error(&url, e))?;
        parse_envelope(status, &body)
    }

    fn network_error(&self, url: &str, error: ureq::Error) -> AxiError {
        let host = url
            .split("//")
            .nth(1)
            .and_then(|rest| rest.split('/').next())
            .unwrap_or(url);
        let detail = match error {
            ureq::Error::Timeout(_) => format!("timed out after {}s", self.timeout.as_secs()),
            ureq::Error::HostNotFound => "host not found".to_string(),
            ureq::Error::ConnectionFailed => "connection failed".to_string(),
            ureq::Error::Io(e) => e.kind().to_string(),
            _ => "request failed".to_string(),
        };
        AxiError::new(ErrorCode::Network, format!("{host}: {detail}")).with_help([format!(
            "Run the command again with `--timeout {}` to allow more time",
            self.timeout.as_secs() * 2
        )])
    }
}

fn parse_envelope(http_status: u16, body: &str) -> Result<Envelope, AxiError> {
    let parsed: Value = match serde_json::from_str(body) {
        Ok(value) => value,
        Err(_) if http_status == 404 => return Err(not_found_path()),
        Err(_) => {
            return Err(AxiError::internal(format!(
                "chicTrip answered HTTP {http_status} with a body that is not the API envelope"
            )));
        }
    };
    let status = parsed
        .get("apiStatus")
        .or_else(|| parsed.get("ApiStatus"))
        .and_then(Value::as_str);
    let status = match status {
        Some(status) => status.to_string(),
        None if http_status == 404 => return Err(not_found_path()),
        None => {
            return Err(AxiError::internal(format!(
                "chicTrip answered HTTP {http_status} without an apiStatus field"
            )));
        }
    };
    Ok(Envelope {
        status,
        data: parsed.get("data").cloned().unwrap_or(Value::Null),
        message: parsed
            .get("message")
            .and_then(Value::as_str)
            .map(str::to_string),
        request_id: parsed
            .get("requestId")
            .and_then(Value::as_str)
            .map(str::to_string),
    })
}

fn not_found_path() -> AxiError {
    AxiError::not_found("chicTrip has no such endpoint")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn envelope(body: &str) -> Envelope {
        parse_envelope(200, body).unwrap()
    }

    #[test]
    fn both_spellings_of_the_status_key_are_accepted() {
        assert_eq!(envelope(r#"{"apiStatus":"001","data":1}"#).status, "001");
        assert_eq!(envelope(r#"{"ApiStatus":"001","data":1}"#).status, "001");
    }

    #[test]
    fn statuses_map_to_the_documented_codes() {
        let cases = [
            (
                r#"{"apiStatus":"002","message":"Reject Guest Member"}"#,
                ErrorCode::AuthRequired,
            ),
            (
                r#"{"apiStatus":"002","message":"404"}"#,
                ErrorCode::NotFound,
            ),
            (
                r#"{"apiStatus":"002","message":"A non-empty request body is required"}"#,
                ErrorCode::ApiError,
            ),
            (r#"{"apiStatus":"003"}"#, ErrorCode::AuthInvalid),
            (
                r#"{"apiStatus":"004","message":"Update time conflict"}"#,
                ErrorCode::Conflict,
            ),
            (
                r#"{"apiStatus":"006","message":"Quit collaboration"}"#,
                ErrorCode::Forbidden,
            ),
            (r#"{"apiStatus":"011"}"#, ErrorCode::NotFound),
            (
                r#"{"apiStatus":"014","message":"nope"}"#,
                ErrorCode::ApiError,
            ),
        ];
        for (body, expected) in cases {
            assert_eq!(envelope(body).into_error().code, expected, "{body}");
        }
    }

    #[test]
    fn the_request_id_travels_with_api_errors() {
        let error = envelope(r#"{"apiStatus":"011","requestId":"abc-1"}"#).into_error();
        assert_eq!(error.request_id.as_deref(), Some("abc-1"));
    }

    #[test]
    fn a_non_envelope_body_is_internal_unless_it_is_a_404() {
        assert_eq!(
            parse_envelope(200, "<html>").unwrap_err().code,
            ErrorCode::Internal
        );
        assert_eq!(
            parse_envelope(404, "Not Found").unwrap_err().code,
            ErrorCode::NotFound
        );
    }
}
