//! Custom dock output — what a program answered, as bounded, sanitized rows.
//!
//! Two shapes: a JSON object `{"rows": [...]}`, or plain text where each line is a row. Every
//! string is stripped of escape sequences, control characters and invisible marks, since a section
//! is drawn straight into the terminal and a program must not be able to write to it through luxe.

use super::{DockInk, DockRow, DockSpan};
use serde::Deserialize;

/// Most rows a section draws; the rest are dropped.
pub(crate) const MAX_ROWS: usize = 8;

/// Most a program may print before its run counts as a failure.
pub(crate) const MAX_OUTPUT_BYTES: usize = 16 * 1024;

/// Longest a single string is kept, in characters. A front-end truncates to the rail anyway; this
/// bounds what an event carries.
const MAX_TEXT_CHARS: usize = 200;

/// Why a program's answer couldn't be read.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub(crate) enum DockOutputError {
    /// It looked like JSON and wasn't the shape a section reads.
    ///
    /// Args: the parser's reason.
    #[error("not valid output: {0}")]
    Invalid(String),
}

/// The JSON shape of an answer.
#[derive(Deserialize)]
struct Answer {
    #[serde(default)]
    rows: Vec<DockRow>,
}

/// The rows in a program's stdout, cleaned and capped at [`MAX_ROWS`].
pub(crate) fn parse_output(stdout: &str) -> Result<Vec<DockRow>, DockOutputError> {
    let trimmed = stdout.trim_start();
    let rows = match trimmed.starts_with('{') {
        true => {
            serde_json::from_str::<Answer>(trimmed)
                .map_err(|error| DockOutputError::Invalid(error.to_string()))?
                .rows
        }
        false => plain_rows(stdout),
    };
    Ok(rows.into_iter().map(clean_row).filter(has_content).take(MAX_ROWS).collect())
}

/// Each line as a row of body text with no label.
fn plain_rows(stdout: &str) -> Vec<DockRow> {
    let row = |line: &str| DockRow {
        label: None,
        spans: vec![DockSpan { text: line.trim_end().to_string(), ink: DockInk::Text }],
    };
    stdout.lines().map(row).collect()
}

/// The row with every string cleaned and empty spans dropped.
fn clean_row(row: DockRow) -> DockRow {
    let label = row.label.map(|label| clean_text(&label)).filter(|label| !label.is_empty());
    let spans = row
        .spans
        .into_iter()
        .map(|span| DockSpan { text: clean_text(&span.text), ink: span.ink })
        .filter(|span| !span.text.is_empty())
        .collect();
    DockRow { label, spans }
}

fn has_content(row: &DockRow) -> bool {
    row.label.is_some() || !row.spans.is_empty()
}

/// `text` without escape sequences, control characters or invisible marks, tabs as spaces, and no
/// longer than [`MAX_TEXT_CHARS`].
pub(crate) fn clean_text(text: &str) -> String {
    crate::compression::strip_ansi(text)
        .chars()
        .map(|c| if c == '\t' { ' ' } else { c })
        .filter(|c| !c.is_control() && !crate::instructions::is_invisible(*c))
        .take(MAX_TEXT_CHARS)
        .collect()
}

#[cfg(test)]
mod tests;
