//! `setup skill`: the installable agent skill file, generated from the same
//! command index the home view prints, so the two cannot drift apart.

use crate::auth::display_path;
use crate::cli::{self, SetupCommand};
use crate::commands::home::COMMAND_INDEX;
use crate::error::{AxiError, ErrorCode};
use crate::output::Document;
use std::path::Path;

const SKILL_TEMPLATE: &str = include_str!("skill_template.md");

/// `--skill` is required because this repo also vendors the `axi` skill.
pub const SKILL_INSTALL: &str = "npx skills add qq88976321/chictrip-axi --skill chictrip-axi";

pub fn run(command: &SetupCommand) -> Result<Document, AxiError> {
    match command {
        SetupCommand::Skill { check, out } => skill(out, *check),
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
}
