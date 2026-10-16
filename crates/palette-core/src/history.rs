//! Named document commands with shared COW snapshots, bounded conservatively.
use crate::document::Document;
use std::collections::VecDeque;

struct Entry {
    label: String,
    state: Document,
    revision: u64,
}

/// Per-document revision history with a conservative 128 MiB snapshot budget.
pub struct History {
    undo: VecDeque<Entry>,
    redo: Vec<Entry>,
    pub revision: u64,
    next_revision: u64,
    budget: usize,
}

impl Default for History {
    fn default() -> Self {
        Self {
            undo: VecDeque::new(),
            redo: vec![],
            revision: 0,
            next_revision: 1,
            budget: 128 * 1024 * 1024,
        }
    }
}

impl History {
    /// Call once before a gesture/command. Brush samples mutate shared tiles
    /// without creating a full document copy for every pointer event.
    pub fn begin(&mut self, label: impl Into<String>, document: &Document) {
        self.undo.push_back(Entry {
            label: label.into(),
            state: document.clone(),
            revision: self.revision,
        });
        self.redo.clear();
        self.revision = self.next_revision;
        self.next_revision += 1;
        let mut bytes: usize = self.undo.iter().map(|e| e.state.byte_len()).sum();
        while bytes > self.budget {
            if let Some(old) = self.undo.pop_front() {
                bytes = bytes.saturating_sub(old.state.byte_len());
            } else {
                break;
            }
        }
    }
    /// Restore the latest pre-command state and transfer the current state to redo.
    pub fn undo(&mut self, document: &mut Document) {
        if let Some(entry) = self.undo.pop_back() {
            self.redo.push(Entry {
                label: entry.label,
                state: std::mem::replace(document, entry.state),
                revision: self.revision,
            });
            self.revision = entry.revision;
        }
    }
    /// Restore the next undone state, preserving its original revision number.
    pub fn redo(&mut self, document: &mut Document) {
        if let Some(entry) = self.redo.pop() {
            self.undo.push_back(Entry {
                label: entry.label,
                state: std::mem::replace(document, entry.state),
                revision: self.revision,
            });
            self.revision = entry.revision;
        }
    }
    /// Human-readable name of the next command to undo.
    pub fn undo_label(&self) -> Option<&str> {
        self.undo.back().map(|e| e.label.as_str())
    }
    /// Human-readable name of the next command to redo.
    pub fn redo_label(&self) -> Option<&str> {
        self.redo.last().map(|e| e.label.as_str())
    }
}
