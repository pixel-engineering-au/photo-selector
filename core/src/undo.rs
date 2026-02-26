use std::path::PathBuf;
use crate::events::MoveAction;

/// A single undoable move operation.
#[derive(Debug, Clone)]
pub struct UndoEntry {
    /// Where the file is now (in selected/ or rejected/).
    pub current_path: PathBuf,
    /// Where it was before the move (the original directory).
    pub original_path: PathBuf,
    /// What action was taken — so we can decrement the right counter.
    pub action: MoveAction,
}

/// A bounded stack of undoable operations.
/// The most recent action is at the top (end of the Vec).
pub struct UndoStack {
    entries: Vec<UndoEntry>,
    /// Maximum number of undo steps retained.
    capacity: usize,
}

impl UndoStack {
    /// Create a new UndoStack with the given capacity.
    /// Recommended default: 50.
    pub fn new(capacity: usize) -> Self {
        assert!(capacity > 0, "undo capacity must be > 0");
        Self {
            entries: Vec::with_capacity(capacity),
            capacity,
        }
    }

    /// Push a new undoable entry. If at capacity, the oldest entry is dropped.
    pub fn push(&mut self, entry: UndoEntry) {
        if self.entries.len() == self.capacity {
            self.entries.remove(0); // drop oldest
        }
        self.entries.push(entry);
    }

    /// Pop the most recent entry for undoing.
    /// Returns None if the stack is empty.
    pub fn pop(&mut self) -> Option<UndoEntry> {
        self.entries.pop()
    }

    /// How many undo steps are available.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Clear all undo history — called on load_dir to avoid
    /// undoing across different directory sessions.
    pub fn clear(&mut self) {
        self.entries.clear();
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    fn entry(from: &str, to: &str) -> UndoEntry {
        UndoEntry {
            current_path: PathBuf::from(to),
            original_path: PathBuf::from(from),
            action: MoveAction::Select,
        }
    }

    #[test]
    fn push_and_pop() {
        let mut stack = UndoStack::new(10);
        stack.push(entry("dir/a.jpg", "dir/selected/a.jpg"));

        let e = stack.pop().unwrap();
        assert_eq!(e.original_path, PathBuf::from("dir/a.jpg"));
        assert_eq!(e.current_path, PathBuf::from("dir/selected/a.jpg"));
    }

    #[test]
    fn pop_empty_returns_none() {
        let mut stack = UndoStack::new(10);
        assert!(stack.pop().is_none());
    }

    #[test]
    fn respects_capacity_by_dropping_oldest() {
        let mut stack = UndoStack::new(2);
        stack.push(entry("a.jpg", "selected/a.jpg"));
        stack.push(entry("b.jpg", "selected/b.jpg"));
        stack.push(entry("c.jpg", "selected/c.jpg")); // drops a.jpg

        assert_eq!(stack.len(), 2);
        // Most recent is c
        let top = stack.pop().unwrap();
        assert_eq!(top.original_path, PathBuf::from("c.jpg"));
        // Next is b (a was dropped)
        let next = stack.pop().unwrap();
        assert_eq!(next.original_path, PathBuf::from("b.jpg"));
    }

    #[test]
    fn clear_empties_stack() {
        let mut stack = UndoStack::new(10);
        stack.push(entry("a.jpg", "selected/a.jpg"));
        stack.clear();
        assert!(stack.is_empty());
    }

    #[test]
    fn lifo_order() {
        let mut stack = UndoStack::new(10);
        stack.push(entry("first.jpg", "selected/first.jpg"));
        stack.push(entry("second.jpg", "selected/second.jpg"));

        assert_eq!(stack.pop().unwrap().original_path, PathBuf::from("second.jpg"));
        assert_eq!(stack.pop().unwrap().original_path, PathBuf::from("first.jpg"));
        assert!(stack.is_empty());
    }
}