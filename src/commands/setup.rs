//! The two agent integrations: the installable skill file, generated from the
//! same command index the home view prints, and the Claude Code SessionStart
//! hook. Neither touches the network, which is why `run` takes no context.

use crate::auth::{self, display_path};
use crate::cli::{self, SetupCommand};
use crate::commands::home::COMMAND_INDEX;
use crate::error::{AxiError, ErrorCode};
use crate::output::Document;
use serde_json::{Value, json};
use std::path::{Path, PathBuf};

const SKILL_TEMPLATE: &str = include_str!("skill_template.md");

/// `--skill` is required because this repo also vendors the `axi` skill.
pub const SKILL_INSTALL: &str = "npx skills add qq88976321/chictrip-axi --skill chictrip-axi";

const BIN_NAME: &str = "chictrip-axi";
const BIN_EXE: &str = "chictrip-axi.exe";
const HOOK_EVENT: &str = "SessionStart";
const HOOK_ARGS: &str = "--timeout 5";
const HOOK_TIMEOUT_SECS: i64 = 10;
const CLAUDE_CONFIG_DIR_ENV: &str = "CLAUDE_CONFIG_DIR";

pub fn run(command: &SetupCommand) -> Result<Document, AxiError> {
    match command {
        SetupCommand::Skill { check, out } => skill(out, *check),
        SetupCommand::Hooks { user, remove } => hooks(*user, *remove),
    }
}

pub fn render_skill() -> String {
    SKILL_TEMPLATE
        .replace("{{description}}", cli::DESCRIPTION)
        .replace("{{commands}}", &render_commands())
}

fn render_commands() -> String {
    COMMAND_INDEX
        .iter()
        .map(|(command, summary)| format!("- `chictrip-axi {command}` - {summary}"))
        .collect::<Vec<String>>()
        .join("\n")
}

fn skill(out: &Path, check: bool) -> Result<Document, AxiError> {
    let shown = display_path(out);
    let wanted = render_skill();
    let current = match std::fs::read_to_string(out) {
        Ok(text) => Some(text),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
        Err(e) => {
            return Err(AxiError::internal(format!(
                "cannot read {shown}: {}",
                e.kind()
            )));
        }
    };

    let mut doc = Document::new();
    doc.set("file", shown.as_str());

    if check {
        let Some(current) = current else {
            return Err(AxiError::not_found(format!("{shown} does not exist"))
                .with_help(["Run `chictrip-axi setup skill` to generate it"]));
        };
        if current != wanted {
            return Err(AxiError::new(
                ErrorCode::Conflict,
                format!("{shown} differs from what this build generates"),
            )
            .with_help(["Run `chictrip-axi setup skill` to regenerate"]));
        }
        doc.set("status", "current");
        doc.set_strings("help", &["The committed skill matches this build"]);
        return Ok(doc);
    }

    if current.as_deref() == Some(wanted.as_str()) {
        doc.set("status", "unchanged");
    } else {
        write_text(out, &wanted)?;
        doc.set("status", "written");
    }
    doc.set_strings(
        "help",
        &[
            "Commit the file; `chictrip-axi setup skill --check` fails when it drifts from the CLI"
                .to_string(),
            format!("Agents install it with `{SKILL_INSTALL}`"),
        ],
    );
    Ok(doc)
}

fn write_text(path: &Path, text: &str) -> Result<(), AxiError> {
    let io_error = |e: std::io::Error| {
        AxiError::internal(format!("cannot write {}: {}", display_path(path), e.kind()))
    };
    if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
        std::fs::create_dir_all(parent).map_err(io_error)?;
    }
    std::fs::write(path, text).map_err(io_error)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HookStatus {
    Installed,
    Unchanged,
    Updated,
    Removed,
    Absent,
}

impl HookStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            HookStatus::Installed => "installed",
            HookStatus::Unchanged => "unchanged",
            HookStatus::Updated => "updated",
            HookStatus::Removed => "removed",
            HookStatus::Absent => "absent",
        }
    }

    fn writes(self) -> bool {
        matches!(
            self,
            HookStatus::Installed | HookStatus::Updated | HookStatus::Removed
        )
    }
}

fn hooks(user: bool, remove: bool) -> Result<Document, AxiError> {
    let path = settings_path(user)?;
    let mut settings = load_settings(&path)?;

    let mut command = String::new();
    let status = if remove {
        remove_hook(&mut settings)?
    } else {
        let exe = std::env::current_exe()
            .map_err(|e| AxiError::internal(format!("cannot locate this binary: {}", e.kind())))?;
        command = hook_command(&exe, &path_dirs());
        upsert_hook(&mut settings, &command)?
    };
    if status.writes() {
        save_settings(&path, &settings)?;
    }

    let mut doc = Document::new();
    doc.set("app", "claude-code");
    doc.set("file", display_path(&path));
    doc.set("event", HOOK_EVENT);
    if !remove {
        doc.set("command", command.as_str());
    }
    doc.set("status", status.as_str());
    doc.set_strings("help", &hook_help(user, remove));
    Ok(doc)
}

fn hook_help(user: bool, remove: bool) -> Vec<String> {
    let user_flag = if user { " --user" } else { "" };
    if remove {
        return vec![format!(
            "Run `chictrip-axi setup hooks{user_flag}` to install it again"
        )];
    }
    let scope = if user { "" } else { " in this project" };
    vec![
        format!("Start a new Claude Code session{scope} to see the home view"),
        format!("Run `chictrip-axi setup hooks --remove{user_flag}` to uninstall"),
    ]
}

/// `CLAUDE_CONFIG_DIR` relocates the user-level settings file, so honour it
/// rather than assuming `~/.claude`.
fn settings_path(user: bool) -> Result<PathBuf, AxiError> {
    let base = if user {
        match auth::env_var(CLAUDE_CONFIG_DIR_ENV) {
            Some(dir) => PathBuf::from(dir),
            None => {
                let home = auth::env_var("HOME").ok_or_else(|| {
                    AxiError::internal("HOME is unset, so ~/.claude cannot be located")
                })?;
                PathBuf::from(home).join(".claude")
            }
        }
    } else {
        std::env::current_dir()
            .map_err(|e| {
                AxiError::internal(format!("cannot read the current directory: {}", e.kind()))
            })?
            .join(".claude")
    };
    Ok(base.join("settings.json"))
}

fn load_settings(path: &Path) -> Result<Value, AxiError> {
    let raw = match std::fs::read_to_string(path) {
        Ok(raw) => raw,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(json!({})),
        Err(e) => {
            return Err(AxiError::internal(format!(
                "cannot read {}: {}",
                display_path(path),
                e.kind()
            )));
        }
    };
    if raw.trim().is_empty() {
        return Ok(json!({}));
    }
    let parsed: Value = serde_json::from_str(&raw)
        .map_err(|e| unreadable(format!("{} is not valid JSON: {e}", display_path(path))))?;
    if !parsed.is_object() {
        return Err(unreadable(format!(
            "{} is not a JSON object",
            display_path(path)
        )));
    }
    Ok(parsed)
}

fn save_settings(path: &Path, settings: &Value) -> Result<(), AxiError> {
    let body = serde_json::to_string_pretty(settings)
        .map_err(|e| AxiError::internal(format!("cannot serialize the settings: {e}")))?;
    write_text(path, &format!("{body}\n"))
}

fn unreadable(message: String) -> AxiError {
    AxiError::new(ErrorCode::Conflict, message)
        .with_help(["Fix the file by hand; setup hooks never rewrites a file it cannot read"])
}

/// A bare name keeps a globally installed binary portable across machines;
/// anything else has to be pinned, or the hook would run whichever
/// `chictrip-axi` PATH happens to resolve to.
pub fn hook_command(current_exe: &Path, path_dirs: &[PathBuf]) -> String {
    let canonical = current_exe
        .canonicalize()
        .unwrap_or_else(|_| current_exe.to_path_buf());
    let on_path = path_dirs.iter().find_map(|dir| {
        [BIN_NAME, BIN_EXE]
            .iter()
            .map(|name| dir.join(name))
            .find(|candidate| candidate.is_file())
    });
    let resolves_here = on_path
        .and_then(|found| found.canonicalize().ok())
        .is_some_and(|found| found == canonical);
    let program = if resolves_here {
        BIN_NAME.to_string()
    } else {
        quote_if_spaced(&canonical.display().to_string())
    };
    format!("{program} {HOOK_ARGS}")
}

fn quote_if_spaced(path: &str) -> String {
    if path.contains(char::is_whitespace) {
        format!("\"{path}\"")
    } else {
        path.to_string()
    }
}

fn path_dirs() -> Vec<PathBuf> {
    std::env::var_os("PATH")
        .map(|path| std::env::split_paths(&path).collect())
        .unwrap_or_default()
}

fn is_our_hook(hook: &Value) -> bool {
    let Some(command) = hook.get("command").and_then(Value::as_str) else {
        return false;
    };
    first_word(command)
        .as_deref()
        .map(Path::new)
        .and_then(Path::file_name)
        .and_then(|name| name.to_str())
        .is_some_and(|name| name == BIN_NAME || name == BIN_EXE)
}

/// The command runs through `sh -c`, so a path with spaces arrives double
/// quoted and the closing quote ends the word.
fn first_word(command: &str) -> Option<String> {
    let command = command.trim_start();
    match command.strip_prefix('"') {
        Some(rest) => rest.split('"').next().map(str::to_string),
        None => command.split_whitespace().next().map(str::to_string),
    }
}

/// Returns the `SessionStart` group list. A settings file whose `hooks` is
/// not the documented shape is reported, never rewritten.
fn session_start_groups(
    settings: &mut Value,
    create: bool,
) -> Result<Option<&mut Vec<Value>>, AxiError> {
    let Some(root) = settings.as_object_mut() else {
        return Err(malformed("the settings file"));
    };
    if !root.contains_key("hooks") {
        if !create {
            return Ok(None);
        }
        root.insert("hooks".to_string(), json!({}));
    }
    let Some(events) = root.get_mut("hooks").and_then(Value::as_object_mut) else {
        return Err(malformed("hooks"));
    };
    if !events.contains_key(HOOK_EVENT) {
        if !create {
            return Ok(None);
        }
        events.insert(HOOK_EVENT.to_string(), json!([]));
    }
    match events.get_mut(HOOK_EVENT).and_then(Value::as_array_mut) {
        Some(groups) => Ok(Some(groups)),
        None => Err(malformed(&format!("hooks.{HOOK_EVENT}"))),
    }
}

fn malformed(what: &str) -> AxiError {
    AxiError::new(
        ErrorCode::Conflict,
        format!("{what} is not the shape Claude Code documents"),
    )
    .with_help(["Fix the file by hand; setup hooks never rewrites a file it cannot read"])
}

/// Omits `matcher`, which is how Claude Code spells "every start reason".
pub fn upsert_hook(settings: &mut Value, command: &str) -> Result<HookStatus, AxiError> {
    let groups = session_start_groups(settings, true)?.expect("create always yields the list");
    for group in groups.iter_mut() {
        let Some(installed) = group.get_mut("hooks").and_then(Value::as_array_mut) else {
            continue;
        };
        for hook in installed.iter_mut() {
            if !is_our_hook(hook) {
                continue;
            }
            if hook.get("command").and_then(Value::as_str) == Some(command) {
                return Ok(HookStatus::Unchanged);
            }
            if let Some(hook) = hook.as_object_mut() {
                hook.insert("command".to_string(), Value::from(command));
            }
            return Ok(HookStatus::Updated);
        }
    }
    groups.push(json!({
        "hooks": [{
            "type": "command",
            "command": command,
            "timeout": HOOK_TIMEOUT_SECS,
        }],
    }));
    Ok(HookStatus::Installed)
}

pub fn remove_hook(settings: &mut Value) -> Result<HookStatus, AxiError> {
    let emptied = {
        let Some(groups) = session_start_groups(settings, false)? else {
            return Ok(HookStatus::Absent);
        };
        let mut removed = false;
        for group in groups.iter_mut() {
            let Some(installed) = group.get_mut("hooks").and_then(Value::as_array_mut) else {
                continue;
            };
            let before = installed.len();
            installed.retain(|hook| !is_our_hook(hook));
            removed |= installed.len() != before;
        }
        if !removed {
            return Ok(HookStatus::Absent);
        }
        groups.retain(|group| {
            group
                .get("hooks")
                .and_then(Value::as_array)
                .is_none_or(|installed| !installed.is_empty())
        });
        groups.is_empty()
    };
    if emptied {
        if let Some(events) = settings
            .as_object_mut()
            .and_then(|root| root.get_mut("hooks"))
            .and_then(Value::as_object_mut)
        {
            events.remove(HOOK_EVENT);
            if events.is_empty() {
                if let Some(root) = settings.as_object_mut() {
                    root.remove("hooks");
                }
            }
        }
    }
    Ok(HookStatus::Removed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_template_placeholders_are_all_filled() {
        assert!(!render_skill().contains("{{"));
    }

    #[test]
    fn every_indexed_command_appears_in_the_skill() {
        let rendered = render_skill();
        for (command, _) in COMMAND_INDEX.iter() {
            assert!(
                rendered.contains(&format!("`chictrip-axi {command}`")),
                "{command} is missing from the skill"
            );
        }
    }

    #[test]
    fn the_skill_is_ascii_with_the_frontmatter_an_agent_loads_on() {
        let rendered = render_skill();
        assert!(rendered.is_ascii());
        assert!(rendered.starts_with("---\nname: chictrip-axi\ndescription: "));
        assert!(rendered.ends_with('\n'));
        assert!(!rendered.ends_with("\n\n"));
        assert!(rendered.contains("chictrip-axi --version"));
    }

    /// The committed file is generated, so a hand edit or a stale copy is a
    /// compile-time-loaded mismatch here as well as a `--check` failure.
    #[test]
    fn the_committed_skill_matches_this_build() {
        assert_eq!(
            include_str!("../../skills/chictrip-axi/SKILL.md"),
            render_skill()
        );
    }

    fn installed(command: &str) -> Value {
        json!({
            "hooks": {
                "SessionStart": [{
                    "hooks": [{
                        "type": "command",
                        "command": command,
                        "timeout": 10,
                    }],
                }],
            },
        })
    }

    #[test]
    fn installing_writes_the_documented_hook_shape_and_repeats_as_a_no_op() {
        let mut settings = json!({});
        assert_eq!(
            upsert_hook(&mut settings, "chictrip-axi --timeout 5").unwrap(),
            HookStatus::Installed
        );
        assert_eq!(settings, installed("chictrip-axi --timeout 5"));

        assert_eq!(
            upsert_hook(&mut settings, "chictrip-axi --timeout 5").unwrap(),
            HookStatus::Unchanged
        );
        assert_eq!(settings, installed("chictrip-axi --timeout 5"));
    }

    #[test]
    fn a_moved_binary_is_repaired_without_touching_anything_else() {
        let mut settings = json!({
            "permissions": {"allow": ["Bash(ls:*)"]},
            "hooks": {
                "Stop": [{"hooks": [{"type": "command", "command": "echo bye"}]}],
                "SessionStart": [
                    {"hooks": [{"type": "command", "command": "echo hi"}]},
                    {"matcher": "startup", "hooks": [
                        {"type": "command", "command": "/old/bin/chictrip-axi --timeout 5", "timeout": 30},
                    ]},
                ],
            },
        });
        assert_eq!(
            upsert_hook(&mut settings, "/new/bin/chictrip-axi --timeout 5").unwrap(),
            HookStatus::Updated
        );
        assert_eq!(
            settings,
            json!({
                "permissions": {"allow": ["Bash(ls:*)"]},
                "hooks": {
                    "Stop": [{"hooks": [{"type": "command", "command": "echo bye"}]}],
                    "SessionStart": [
                        {"hooks": [{"type": "command", "command": "echo hi"}]},
                        {"matcher": "startup", "hooks": [
                            {"type": "command", "command": "/new/bin/chictrip-axi --timeout 5", "timeout": 30},
                        ]},
                    ],
                },
            })
        );
    }

    #[test]
    fn removing_takes_only_our_hook_and_leaves_no_empty_scaffolding() {
        let mut settings = json!({});
        upsert_hook(&mut settings, "chictrip-axi --timeout 5").unwrap();
        assert_eq!(remove_hook(&mut settings).unwrap(), HookStatus::Removed);
        assert_eq!(settings, json!({}));

        let mut settings = json!({
            "hooks": {"SessionStart": [{"hooks": [
                {"type": "command", "command": "echo hi"},
                {"type": "command", "command": "chictrip-axi --timeout 5", "timeout": 10},
            ]}]},
        });
        assert_eq!(remove_hook(&mut settings).unwrap(), HookStatus::Removed);
        assert_eq!(
            settings,
            json!({
                "hooks": {"SessionStart": [{"hooks": [
                    {"type": "command", "command": "echo hi"},
                ]}]},
            })
        );

        let stranger = json!({
            "hooks": {"SessionStart": [{"hooks": [
                {"type": "command", "command": "echo hi"},
            ]}]},
        });
        let mut settings = stranger.clone();
        assert_eq!(remove_hook(&mut settings).unwrap(), HookStatus::Absent);
        assert_eq!(settings, stranger);

        let mut settings = json!({});
        assert_eq!(remove_hook(&mut settings).unwrap(), HookStatus::Absent);
        assert_eq!(settings, json!({}));
    }

    #[test]
    fn only_a_hook_whose_program_is_this_binary_is_ours() {
        let ours = |command: &str| is_our_hook(&json!({"type": "command", "command": command}));
        assert!(ours("/x/y/chictrip-axi --timeout 5"));
        assert!(ours("chictrip-axi.exe --timeout 5"));
        assert!(ours("\"/with space/chictrip-axi\" --timeout 5"));
        assert!(!ours("echo chictrip-axi"));
        assert!(!is_our_hook(&json!("chictrip-axi --timeout 5")));
    }

    #[test]
    fn a_settings_file_of_the_wrong_shape_is_reported_never_rewritten() {
        for mut settings in [json!({"hooks": []}), json!({"hooks": {"SessionStart": {}}})] {
            let before = settings.clone();
            let error = upsert_hook(&mut settings, "chictrip-axi --timeout 5").unwrap_err();
            assert_eq!(error.code, ErrorCode::Conflict);
            assert_eq!(
                remove_hook(&mut settings).unwrap_err().code,
                ErrorCode::Conflict
            );
            assert_eq!(settings, before);
        }
    }

    #[test]
    fn the_hook_uses_the_bare_name_only_when_path_resolves_to_this_binary() {
        let root = std::env::temp_dir().join(format!("chictrip-axi-hook-{}", std::process::id()));
        let (bin, other, empty) = (root.join("bin"), root.join("other"), root.join("empty"));
        for dir in [&bin, &other, &empty] {
            std::fs::create_dir_all(dir).unwrap();
        }
        let exe = bin.join(BIN_NAME);
        std::fs::write(&exe, "").unwrap();
        std::fs::write(other.join(BIN_NAME), "").unwrap();

        assert_eq!(
            hook_command(&exe, std::slice::from_ref(&bin)),
            format!("{BIN_NAME} --timeout 5")
        );
        let pinned = format!("{} --timeout 5", exe.canonicalize().unwrap().display());
        assert_eq!(hook_command(&exe, std::slice::from_ref(&empty)), pinned);
        assert_eq!(hook_command(&exe, &[other, bin]), pinned);

        std::fs::remove_dir_all(&root).unwrap();
    }
}
