//! Custom dock answers — what each custom section's or statusline module's program last said.
//!
//! A failure arrives without rows, and keeps the rows before it: a section that showed 30% and
//! then timed out once still knows it was 30% a moment ago, which is more use than a blank.

use luxe_core::custom_dock::{DockAnswer, DockRow};
use std::collections::BTreeMap;

/// One section's last answer.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct CustomDockAnswer {
    /// The last rows a run gave
    pub(crate) rows: Vec<DockRow>,
    /// Why the runs since then drew nothing, cleared by the next good one
    pub(crate) failure: Option<String>,
}

/// Every custom section's last answer, by name.
#[derive(Debug, Clone, Default)]
pub(crate) struct CustomDockAnswers(BTreeMap<String, CustomDockAnswer>);

impl CustomDockAnswers {
    /// Takes in a run's answer: new rows replace the old ones, a failure sits under them.
    pub(crate) fn record(&mut self, answer: DockAnswer) {
        let DockAnswer { name, rows, failure } = answer;
        let kept = self.0.entry(name).or_default();
        match failure {
            Some(failure) => kept.failure = Some(failure),
            None => *kept = CustomDockAnswer { rows, failure: None },
        }
    }

    /// What `name` last said, or nothing before its first run finished.
    pub(crate) fn get(&self, name: &str) -> Option<&CustomDockAnswer> {
        self.0.get(name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use luxe_core::custom_dock::DockSpan;

    fn answer(name: &str, rows: Vec<DockRow>, failure: Option<&str>) -> DockAnswer {
        DockAnswer { name: name.into(), rows, failure: failure.map(Into::into) }
    }

    fn rows(text: &str) -> Vec<DockRow> {
        vec![DockRow {
            label: None,
            spans: vec![DockSpan { text: text.into(), ..Default::default() }],
        }]
    }

    #[test]
    fn a_failure_keeps_the_rows_before_it_and_a_success_clears_it() {
        let mut answers = CustomDockAnswers::default();
        answers.record(answer("quota", rows("30%"), None));
        answers.record(answer("quota", vec![], Some("timed out")));
        let kept = answers.get("quota").unwrap();
        assert_eq!((kept.rows.clone(), kept.failure.as_deref()), (rows("30%"), Some("timed out")));
        answers.record(answer("quota", rows("40%"), None));
        assert_eq!(
            answers.get("quota"),
            Some(&CustomDockAnswer { rows: rows("40%"), failure: None })
        );
    }

    #[test]
    fn a_first_run_that_fails_has_only_its_failure() {
        let mut answers = CustomDockAnswers::default();
        answers.record(answer("ci", vec![], Some("exited 1")));
        assert!(answers.get("ci").unwrap().rows.is_empty());
        assert_eq!(answers.get("other"), None);
    }
}
