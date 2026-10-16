//! Named document commands bounded by uniquely retained COW backing allocations.
use crate::document::Document;
use std::collections::VecDeque;

struct Entry {
    label: String,
    state: Document,
    revision: u64,
}

/// Per-document history capped at 256 commands and 128 MiB of backing buffers
/// retained in addition to the live document. Shared tiles are counted once.
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
        self.finish(document);
    }
    /// Trim after a completed command/gesture, when modified tiles are known.
    pub fn finish(&mut self, document: &Document) {
        while self.undo.len() > 256 {
            self.undo.pop_front();
        }
        while self.retained_bytes(document) > self.budget {
            if self.undo.pop_front().is_none() {
                self.redo.clear();
                break;
            }
        }
    }
    /// Bytes owned exclusively by undo/redo snapshots, excluding live buffers.
    pub fn retained_bytes(&self, document: &Document) -> usize {
        let live = document.allocations();
        let mut allocations = std::collections::HashMap::new();
        for entry in self.undo.iter().chain(&self.redo) {
            allocations.extend(entry.state.allocations());
        }
        allocations
            .into_iter()
            .filter(|(address, _)| !live.contains_key(address))
            .map(|(_, bytes)| bytes)
            .sum()
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
            self.finish(document);
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
            self.finish(document);
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
