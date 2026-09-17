//! Output layer: commands build an ordered [`Document`]; this module renders
//! it as TOON (default) or as one line of compact JSON (`--json`).
//!
//! The document keeps its own insertion order instead of relying on a
//! `serde_json::Map`, whose ordering depends on a cargo feature that a
//! dependency may or may not enable.

use crate::error::AxiError;
use serde::ser::{SerializeMap, SerializeSeq};
use serde::{Serialize, Serializer};
use serde_json::Value;

/// Long free text is cut at this many characters by the AXI output contract.
pub const TRUNCATE_LIMIT: usize = 500;

#[derive(Debug, Clone, Default)]
pub struct Document {
    entries: Vec<(String, Node)>,
    primary: Option<String>,
}

#[derive(Debug, Clone)]
pub enum Node {
    Scalar(Value),
    List(Vec<Value>),
    Table(Table),
    Object(Document),
}

impl Document {
    pub fn new() -> Self {
        Document::default()
    }

    /// Scalars that are null are dropped: the contract omits empty fields.
    pub fn set(&mut self, key: &str, value: impl Into<Value>) {
        let value = value.into();
        if value.is_null() {
            return;
        }
        self.entries.push((key.to_string(), Node::Scalar(value)));
    }

    pub fn set_list(&mut self, key: &str, values: Vec<Value>) {
        self.entries.push((key.to_string(), Node::List(values)));
    }

    pub fn set_strings<S: AsRef<str>>(&mut self, key: &str, values: &[S]) {
        self.set_list(
            key,
            values.iter().map(|v| Value::from(v.as_ref())).collect(),
        );
    }

    pub fn set_table(&mut self, key: &str, table: Table) {
        self.entries.push((key.to_string(), Node::Table(table)));
    }

    pub fn set_object(&mut self, key: &str, doc: Document) {
        self.entries.push((key.to_string(), Node::Object(doc)));
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn keys(&self) -> Vec<String> {
        self.entries.iter().map(|(k, _)| k.clone()).collect()
    }

    /// Names the entry `--fields` projects. Commands set it because a
    /// document can hold several tables (a detail view plus its sub-tables).
    pub fn set_primary(&mut self, key: &str) {
        self.primary = Some(key.to_string());
    }

    pub fn apply_fields(&mut self, fields: &[String]) -> Result<(), AxiError> {
        let primary = self.primary.clone();
        let target = match primary {
            Some(key) => self.entries.iter_mut().find(|(k, _)| *k == key),
            None => self
                .entries
                .iter_mut()
                .find(|(_, node)| matches!(node, Node::Table(_) | Node::Object(_))),
        };
        match target {
            Some((_, Node::Table(table))) => table.select(fields),
            Some((_, Node::Object(doc))) => doc.retain_fields(fields),
            _ => Err(AxiError::usage("--fields is not supported by this command")),
        }
    }

    fn retain_fields(&mut self, fields: &[String]) -> Result<(), AxiError> {
        let available = self.keys();
        for field in fields {
            if !available.contains(field) {
                return Err(unknown_field(field, &available));
            }
        }
        let mut kept = Vec::new();
        for field in fields {
            if let Some(pos) = self.entries.iter().position(|(k, _)| k == field) {
                kept.push(self.entries[pos].clone());
            }
        }
        self.entries = kept;
        Ok(())
    }
}

/// A uniform table. Rows carry every known column so that `--fields` can
/// surface extras without a second fetch; only the active columns are
/// rendered.
#[derive(Debug, Clone)]
pub struct Table {
    columns: Vec<String>,
    active: Vec<usize>,
    rows: Vec<Vec<Value>>,
}

impl Table {
    pub fn new(default_columns: &[&str], extra_columns: &[&str]) -> Self {
        let mut columns: Vec<String> = default_columns.iter().map(|c| c.to_string()).collect();
        let active = (0..columns.len()).collect();
        columns.extend(extra_columns.iter().map(|c| c.to_string()));
        Table {
            columns,
            active,
            rows: Vec::new(),
        }
    }

    pub fn push(&mut self, cells: &[(&str, Value)]) {
        let mut row = vec![Value::Null; self.columns.len()];
        for (name, value) in cells {
            match self.columns.iter().position(|c| c == name) {
                Some(index) => row[index] = value.clone(),
                None => debug_assert!(false, "unknown column {name}"),
            }
        }
        self.rows.push(row);
    }

    pub fn len(&self) -> usize {
        self.rows.len()
    }

    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }

    pub fn select(&mut self, fields: &[String]) -> Result<(), AxiError> {
        let mut active = Vec::new();
        for field in fields {
            match self.columns.iter().position(|c| c == field) {
                Some(index) => active.push(index),
                None => return Err(unknown_field(field, &self.columns)),
            }
        }
        self.active = active;
        Ok(())
    }
}

fn unknown_field(field: &str, available: &[String]) -> AxiError {
    AxiError::usage(format!(
        "unknown --fields name '{field}'; valid names: {}",
        available.join(",")
    ))
}

/// Cut `text` to [`TRUNCATE_LIMIT`] characters, appending the size hint the
/// AXI contract asks for. Returns the text and whether anything was cut.
pub fn truncate(text: &str) -> (String, bool) {
    let total = text.chars().count();
    if total <= TRUNCATE_LIMIT {
        return (text.to_string(), false);
    }
    let head: String = text.chars().take(TRUNCATE_LIMIT).collect();
    (format!("{head}... (truncated, {total} chars total)"), true)
}

pub fn render(doc: &Document, json: bool) -> String {
    let body = if json {
        serde_json::to_string(doc)
            .unwrap_or_else(|e| format!("{{\"error\":\"internal\",\"message\":\"{e}\"}}"))
    } else {
        toon_format::encode_default(doc)
            .unwrap_or_else(|e| format!("error: internal\nmessage: {e}"))
    };
    if body.ends_with('\n') {
        body
    } else {
        format!("{body}\n")
    }
}

impl Serialize for Document {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut map = serializer.serialize_map(Some(self.entries.len()))?;
        for (key, node) in &self.entries {
            map.serialize_entry(key, node)?;
        }
        map.end()
    }
}

impl Serialize for Node {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            Node::Scalar(value) => value.serialize(serializer),
            Node::List(values) => values.serialize(serializer),
            Node::Table(table) => table.serialize(serializer),
            Node::Object(doc) => doc.serialize(serializer),
        }
    }
}

impl Serialize for Table {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut seq = serializer.serialize_seq(Some(self.rows.len()))?;
        for row in &self.rows {
            seq.serialize_element(&Row {
                table: self,
                row: row.as_slice(),
            })?;
        }
        seq.end()
    }
}

struct Row<'a> {
    table: &'a Table,
    row: &'a [Value],
}

impl Serialize for Row<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut map = serializer.serialize_map(Some(self.table.active.len()))?;
        for &index in &self.table.active {
            map.serialize_entry(&self.table.columns[index], &self.row[index])?;
        }
        map.end()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Document {
        let mut table = Table::new(&["id", "name"], &["city"]);
        table.push(&[
            ("id", "a".into()),
            ("name", "Senso-ji".into()),
            ("city", "Tokyo".into()),
        ]);
        table.push(&[("id", "b".into()), ("name", "Bridge".into())]);
        let mut doc = Document::new();
        doc.set("count", 2);
        doc.set_table("pois", table);
        doc.set_strings("help", &["Run `chictrip-axi poi view <id>` for details"]);
        doc
    }

    #[test]
    fn toon_keeps_insertion_order_and_default_columns() {
        let rendered = render(&sample(), false);
        assert_eq!(
            rendered,
            concat!(
                "count: 2\n",
                "pois[2]{id,name}:\n",
                "  a,\"Senso-ji\"\n",
                "  b,Bridge\n",
                "help[1]: \"Run `chictrip-axi poi view <id>` for details\"\n"
            )
        );
    }

    #[test]
    fn json_is_one_compact_line() {
        let rendered = render(&sample(), true);
        assert_eq!(rendered.lines().count(), 1);
        assert!(rendered.starts_with(r#"{"count":2,"pois":[{"id":"a","name":"Senso-ji"}"#));
    }

    #[test]
    fn fields_reorder_and_reveal_extra_columns() {
        let mut doc = sample();
        doc.apply_fields(&["city".to_string(), "id".to_string()])
            .unwrap();
        assert!(render(&doc, false).contains("pois[2]{city,id}:"));
    }

    #[test]
    fn unknown_field_is_a_usage_error_listing_the_valid_names() {
        let mut doc = sample();
        let err = doc.apply_fields(&["nope".to_string()]).unwrap_err();
        assert_eq!(err.code, crate::error::ErrorCode::Usage);
        assert!(err.message.contains("valid names: id,name,city"));
    }

    #[test]
    fn empty_tables_still_print_a_definitive_header() {
        let mut doc = Document::new();
        doc.set("count", 0);
        doc.set_table("trips", Table::new(&["id", "name"], &[]));
        assert_eq!(render(&doc, false), "count: 0\ntrips[0]:\n");
    }

    #[test]
    fn truncate_reports_the_total_size() {
        let text: String = "a".repeat(TRUNCATE_LIMIT + 12);
        let (cut, was_cut) = truncate(&text);
        assert!(was_cut);
        assert!(cut.ends_with("... (truncated, 512 chars total)"));
        assert_eq!(truncate("short"), ("short".to_string(), false));
    }
}
