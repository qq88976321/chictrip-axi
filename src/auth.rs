//! Token resolution and the on-disk auth file.

use crate::datetime::format_utc_datetime;
use crate::error::{AxiError, ErrorCode};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// chicTrip's public anonymous JWT, hardcoded in the web app's bundle
/// (`homeStore.*.js`) and served to every visitor who has not logged in.
/// It is what makes the read-only commands work with no setup. chicTrip can
/// rotate it at any time; refreshing it here is a patch release, never a
/// runtime lookup.
pub const GUEST_TOKEN: &str = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIwMDAwMDAwMC0wMDAwLTAwMDAtMDAwMC0wMDAwMDAwMDAwMDEiLCJqdGkiOiIzMzVjNmE5ZC03ZGEwLTRiOWYtOTYyMS0wYjhkZTA2ZTNkZjMiLCJuYmYiOjE2ODU0MjgzNTksImV4cCI6MjExNzQyODM1OSwiaWF0IjoxNjg1NDI4MzU5LCJpc3MiOiJDaGljVHJpcEFwaSJ9.qauV9-13W9f4VbgXFD60F9xZ5CwfM4ui-pK-45chrNw";

/// The guest JWT's subject; any token carrying it is the shared anonymous
/// identity, whatever its source.
pub const GUEST_MEMBER_ID: &str = "00000000-0000-0000-0000-000000000001";

pub const AUTH_FILE_ENV: &str = "CHICTRIP_AXI_AUTH_FILE";
pub const TOKEN_ENV: &str = "CHICTRIP_AXI_TOKEN";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenSource {
    Flag,
    Env,
    File,
    Guest,
}

impl TokenSource {
    pub fn as_str(self) -> &'static str {
        match self {
            TokenSource::Flag => "flag",
            TokenSource::Env => "env",
            TokenSource::File => "file",
            TokenSource::Guest => "guest",
        }
    }
}

/// Exactly the JSON the README's browser snippet copies out of localStorage.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthFile {
    pub access_token: String,
    #[serde(default)]
    pub refresh_token: String,
    #[serde(default)]
    pub member_id: String,
}

#[derive(Debug, Clone)]
pub struct Credentials {
    pub token: String,
    pub source: TokenSource,
    pub stored: Option<AuthFile>,
    pub path: PathBuf,
}

impl Credentials {
    pub fn is_guest(&self) -> bool {
        self.source == TokenSource::Guest
            || jwt_subject(&self.token).as_deref() == Some(GUEST_MEMBER_ID)
    }

    pub fn member_id(&self) -> Option<String> {
        jwt_subject(&self.token).or_else(|| {
            self.stored
                .as_ref()
                .map(|s| s.member_id.clone())
                .filter(|id| !id.is_empty())
        })
    }
}

pub fn auth_file_path() -> PathBuf {
    if let Some(path) = env_var(AUTH_FILE_ENV) {
        return PathBuf::from(path);
    }
    let base = env_var("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(env_var("HOME").unwrap_or_default()).join(".config"));
    base.join("chictrip-axi").join("auth.json")
}

/// Collapses the home prefix so paths printed to an agent stay short.
pub fn display_path(path: &Path) -> String {
    let text = path.display().to_string();
    match env_var("HOME") {
        Some(home) if !home.is_empty() && text.starts_with(&home) => {
            format!("~{}", &text[home.len()..])
        }
        _ => text,
    }
}

fn env_var(name: &str) -> Option<String> {
    std::env::var(name).ok().filter(|v| !v.is_empty())
}

pub fn load_file(path: &Path) -> Result<Option<AuthFile>, AxiError> {
    let raw = match std::fs::read_to_string(path) {
        Ok(raw) => raw,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => {
            return Err(AxiError::new(
                ErrorCode::Internal,
                format!("cannot read {}: {}", display_path(path), e.kind()),
            ));
        }
    };
    parse_auth_json(&raw).map(Some)
}

pub fn parse_auth_json(raw: &str) -> Result<AuthFile, AxiError> {
    let auth: AuthFile = serde_json::from_str(raw).map_err(|_| {
        AxiError::usage("auth JSON must be an object with accessToken, refreshToken, and memberId")
    })?;
    if auth.access_token.is_empty() {
        return Err(AxiError::usage("accessToken is empty"));
    }
    if jwt_payload(&auth.access_token).is_none() {
        return Err(AxiError::usage("accessToken is not a JWT"));
    }
    Ok(auth)
}

pub fn save_file(path: &Path, auth: &AuthFile) -> Result<(), AxiError> {
    let io_error = |e: std::io::Error| {
        AxiError::new(
            ErrorCode::Internal,
            format!("cannot write {}: {}", display_path(path), e.kind()),
        )
    };
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(io_error)?;
    }
    let body = serde_json::to_string_pretty(auth)
        .map_err(|e| AxiError::internal(format!("cannot serialize auth: {e}")))?;
    std::fs::write(path, format!("{body}\n")).map_err(io_error)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600)).map_err(io_error)?;
    }
    Ok(())
}

pub fn remove_file(path: &Path) -> Result<bool, AxiError> {
    match std::fs::remove_file(path) {
        Ok(()) => Ok(true),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(e) => Err(AxiError::new(
            ErrorCode::Internal,
            format!("cannot remove {}: {}", display_path(path), e.kind()),
        )),
    }
}

/// Precedence: `--token` > `CHICTRIP_AXI_TOKEN` > the auth file > guest.
pub fn resolve(token_flag: Option<&str>) -> Result<Credentials, AxiError> {
    let path = auth_file_path();
    let stored = load_file(&path)?;
    if let Some(token) = token_flag.filter(|t| !t.is_empty()) {
        return Ok(Credentials {
            token: token.to_string(),
            source: TokenSource::Flag,
            stored,
            path,
        });
    }
    if let Some(token) = env_var(TOKEN_ENV) {
        return Ok(Credentials {
            token,
            source: TokenSource::Env,
            stored,
            path,
        });
    }
    if let Some(auth) = &stored {
        return Ok(Credentials {
            token: auth.access_token.clone(),
            source: TokenSource::File,
            stored: stored.clone(),
            path,
        });
    }
    Ok(Credentials {
        token: GUEST_TOKEN.to_string(),
        source: TokenSource::Guest,
        stored,
        path,
    })
}

fn jwt_payload(token: &str) -> Option<serde_json::Value> {
    let mut parts = token.split('.');
    let (_, payload, signature) = (parts.next()?, parts.next()?, parts.next()?);
    if signature.is_empty() || parts.next().is_some() {
        return None;
    }
    serde_json::from_slice(&base64url_decode(payload)?).ok()
}

pub fn jwt_subject(token: &str) -> Option<String> {
    jwt_payload(token)?.get("sub")?.as_str().map(str::to_string)
}

pub fn jwt_expiry(token: &str) -> Option<i64> {
    jwt_payload(token)?.get("exp")?.as_i64()
}

pub fn jwt_expiry_utc(token: &str) -> Option<String> {
    jwt_expiry(token).map(format_utc_datetime)
}

fn base64url_decode(input: &str) -> Option<Vec<u8>> {
    let mut out = Vec::with_capacity(input.len() * 3 / 4);
    let mut buffer = 0u32;
    let mut bits = 0u32;
    for byte in input.bytes() {
        let value = match byte {
            b'A'..=b'Z' => byte - b'A',
            b'a'..=b'z' => byte - b'a' + 26,
            b'0'..=b'9' => byte - b'0' + 52,
            b'-' => 62,
            b'_' => 63,
            b'=' => break,
            _ => return None,
        };
        buffer = (buffer << 6) | u32::from(value);
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((buffer >> bits) as u8);
        }
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_guest_token_decodes_to_the_anonymous_member() {
        assert_eq!(jwt_subject(GUEST_TOKEN).as_deref(), Some(GUEST_MEMBER_ID));
        assert_eq!(
            jwt_expiry_utc(GUEST_TOKEN).as_deref(),
            Some("2037-02-05T06:32:39Z")
        );
    }

    #[test]
    fn garbage_tokens_have_no_payload() {
        assert!(jwt_subject("not-a-jwt").is_none());
        assert!(jwt_subject("a.b").is_none());
        assert!(jwt_expiry("aaa.###.bbb").is_none());
    }

    #[test]
    fn auth_json_needs_a_jwt_access_token() {
        let good =
            format!(r#"{{"accessToken":"{GUEST_TOKEN}","refreshToken":"r","memberId":"m"}}"#);
        let parsed = parse_auth_json(&good).unwrap();
        assert_eq!(parsed.member_id, "m");
        assert!(parse_auth_json(r#"{"accessToken":""}"#).is_err());
        assert!(parse_auth_json(r#"{"accessToken":"plain"}"#).is_err());
        assert!(parse_auth_json("not json").is_err());
    }

    #[test]
    fn a_guest_subject_is_guest_whatever_the_source() {
        let creds = Credentials {
            token: GUEST_TOKEN.to_string(),
            source: TokenSource::Flag,
            stored: None,
            path: PathBuf::from("/tmp/none"),
        };
        assert!(creds.is_guest());
    }
}
