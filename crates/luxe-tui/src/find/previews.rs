//! Find previews — the files Find's preview pane has read, and the reads still out.
//!
//! The pane shows the file around the row under the cursor, so moving the cursor needs that
//! file's text. Each read goes to the engine with Find's own request id, which keeps it apart from
//! the preview pane's reads, and lands here. A few recent files stay, so moving back and forth
//! between hits doesn't read them again.

use luxe_core::project_files::{FileReadRequestId, ProjectFile, ProjectFileFailure};
use std::collections::{HashMap, VecDeque};

/// Files kept, newest last; browsing past this lets the oldest go.
const MAX_FILES: usize = 12;

/// What the pane has of one file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum PreviewText {
    /// The read is out
    Reading,
    /// The file as the engine read it
    Ready(Box<ProjectFile>),
    /// Why it couldn't be read
    Failed(String),
}

/// The files read for the pane, and which path each read still out is for.
#[derive(Debug, Clone, Default)]
pub(crate) struct PreviewCache {
    files: VecDeque<(String, PreviewText)>,
    reads: HashMap<FileReadRequestId, String>,
}

impl PreviewCache {
    /// What the pane has of `path`, if anything.
    pub(crate) fn get(&self, path: &str) -> Option<&PreviewText> {
        self.files.iter().find(|(held, _)| held == path).map(|(_, text)| text)
    }

    /// Whether `path` needs reading: neither here nor on its way.
    pub(crate) fn wants(&self, path: &str) -> bool {
        self.get(path).is_none()
    }

    /// Records a read sent for `path`, which only its own answer can fill.
    pub(crate) fn reading(&mut self, request: FileReadRequestId, path: &str) {
        self.reads.insert(request, path.to_owned());
        self.hold(path, PreviewText::Reading);
    }

    /// Takes a read's answer, if Find asked for it.
    pub(crate) fn accept(&mut self, request: &FileReadRequestId, file: &ProjectFile) -> bool {
        let Some(path) = self.reads.remove(request) else { return false };
        self.hold(&path, PreviewText::Ready(Box::new(file.clone())));
        true
    }

    /// Takes a read's failure, if Find asked for it.
    pub(crate) fn fail(&mut self, request: &FileReadRequestId, error: &ProjectFileFailure) -> bool {
        let Some(path) = self.reads.remove(request) else { return false };
        self.hold(&path, PreviewText::Failed(error.to_string()));
        true
    }

    /// Keeps `text` for `path` as the newest file, letting the oldest go past the limit.
    fn hold(&mut self, path: &str, text: PreviewText) {
        self.files.retain(|(held, _)| held != path);
        self.files.push_back((path.to_owned(), text));
        while self.files.len() > MAX_FILES {
            self.files.pop_front();
        }
    }
}

#[cfg(test)]
mod tests;
