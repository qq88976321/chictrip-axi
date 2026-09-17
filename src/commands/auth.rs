//! `auth set`, `auth status`, and `auth clear`.

use crate::auth::{self, TokenSource};
use crate::cli::{AuthCommand, Context};
use crate::error::AxiError;
use crate::output::Document;

pub fn run(ctx: &Context, command: &AuthCommand) -> Result<Document, AxiError> {
    match command {
        AuthCommand::Set {
            from_json,
            access_token,
            refresh_token,
            member_id,
        } => set(
            from_json.as_deref(),
            access_token.as_deref(),
            refresh_token.as_deref(),
            member_id.as_deref(),
        ),
        AuthCommand::Status => status(ctx),
        AuthCommand::Clear => clear(),
    }
}

fn set(
    from_json: Option<&str>,
    access_token: Option<&str>,
    refresh_token: Option<&str>,
    member_id: Option<&str>,
) -> Result<Document, AxiError> {
    let mut stored = match from_json {
        Some("-") => auth::parse_auth_json(&read_stdin()?)?,
        Some(path) => {
            let raw = std::fs::read_to_string(path).map_err(|e| {
                AxiError::usage(format!("cannot read --from-json {path}: {}", e.kind()))
            })?;
            auth::parse_auth_json(&raw)?
        }
        None => {
            let access_token = access_token.ok_or_else(|| {
                AxiError::usage(
                    "auth set needs --from-json FILE, or --access-token with --member-id",
                )
                .with_help([
                    "Run `chictrip-axi auth set --from-json -` and paste the JSON copied from the browser (see README)",
                ])
            })?;
            auth::parse_auth_json(&serde_json::json!({ "accessToken": access_token }).to_string())?
        }
    };
    if let Some(token) = refresh_token {
        stored.refresh_token = token.to_string();
    }
    if let Some(id) = member_id {
        stored.member_id = id.to_string();
    }
    if stored.member_id.is_empty() {
        stored.member_id = auth::jwt_subject(&stored.access_token).unwrap_or_default();
    }

    let path = auth::auth_file_path();
    auth::save_file(&path, &stored)?;

    let mut detail = Document::new();
    detail.set("member_id", stored.member_id.as_str());
    if let Some(expiry) = auth::jwt_expiry_utc(&stored.access_token) {
        detail.set("access_token_expires", expiry);
    }
    detail.set("file", auth::display_path(&path));

    let mut doc = Document::new();
    doc.set_object("auth", detail);
    doc.set_primary("auth");
    doc.set_strings(
        "help",
        &["Run `chictrip-axi auth status` to verify the token against the API"],
    );
    Ok(doc)
}

fn read_stdin() -> Result<String, AxiError> {
    std::io::read_to_string(std::io::stdin()).map_err(|e| {
        AxiError::usage(format!(
            "cannot read the auth JSON from stdin: {}",
            e.kind()
        ))
    })
}

fn status(ctx: &Context) -> Result<Document, AxiError> {
    let creds = auth::resolve(ctx.global.token.as_deref())?;
    let mut detail = Document::new();
    detail.set("source", creds.source.as_str());
    if let Some(id) = creds.member_id() {
        detail.set("member_id", id);
    }
    if let Some(expiry) = auth::jwt_expiry_utc(&creds.token) {
        detail.set("access_token_expires", expiry);
    }
    if creds.source == TokenSource::File {
        detail.set("file", auth::display_path(&creds.path));
    }

    let client = ctx.client()?;
    match client.get("TravelScheduleUserLabel/Get", &[]) {
        Ok(_) => detail.set("valid", true),
        Err(e) => {
            detail.set("valid", false);
            detail.set("message", e.message.as_str());
        }
    }

    let mut doc = Document::new();
    doc.set_object("auth", detail);
    doc.set_primary("auth");
    if creds.is_guest() {
        doc.set_strings(
            "help",
            &["Run `chictrip-axi auth set --from-json -` and paste the JSON copied from the browser to use your own trips"],
        );
    }
    Ok(doc)
}

fn clear() -> Result<Document, AxiError> {
    let path = auth::auth_file_path();
    let removed = auth::remove_file(&path)?;
    let mut doc = Document::new();
    if removed {
        doc.set("auth", format!("cleared {}", auth::display_path(&path)));
    } else {
        doc.set("auth", "nothing stored (no-op)");
    }
    doc.set_strings(
        "help",
        &["Run `chictrip-axi auth status` to see which token is in use now"],
    );
    Ok(doc)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn set_without_any_input_is_a_usage_error_pointing_at_the_browser_snippet() {
        let error = set(None, None, None, None).unwrap_err();
        assert_eq!(error.code, crate::error::ErrorCode::Usage);
        assert!(error.help[0].contains("--from-json -"));
    }

    #[test]
    fn a_bare_access_token_must_still_be_a_jwt() {
        assert!(set(None, Some("not-a-jwt"), None, Some("m")).is_err());
    }
}
