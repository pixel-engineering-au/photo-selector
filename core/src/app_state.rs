use std::path::PathBuf;

use crate::image_index::{ImageIndex, SortOrder};
use crate::navigation::NavigationEngine;
use crate::image_cache::{ImageCache, Image};
use crate::events::{AppEvent, BoundaryKind, MoveAction, PageState};
use crate::stats::{LibraryStats, count_in_subdir};
use crate::undo::{UndoStack, UndoEntry};

pub enum Action {
    Select,
    Reject,
}

pub struct AppState {
    index: ImageIndex,
    nav: NavigationEngine,
    cache: ImageCache,
    undo_stack: UndoStack,
    /// The base directory currently loaded — needed for stats and undo.
    base_dir: Option<PathBuf>,
}

impl AppState {
    pub fn new(view_count: usize) -> Self {
        Self {
            index: ImageIndex::new(),
            nav: NavigationEngine::new(view_count),
            cache: ImageCache::new(),
            undo_stack: UndoStack::new(50),
            base_dir: None,
        }
    }

    // ── Directory loading ──────────────────────────────────────────────────────

    pub fn load_dir(&mut self, dir: &std::path::Path) -> Vec<AppEvent> {
        self.nav.current_index = 0;
        self.undo_stack.clear(); // never undo across sessions
        self.base_dir = Some(dir.to_path_buf());

        // Collect scan events as the index builds
        let mut events: Vec<AppEvent> = vec![AppEvent::ScanStarted {
            path: dir.to_path_buf(),
        }];

        self.index.scan_dir_with_progress(dir, &mut |scanned| {
            events.push(AppEvent::ScanProgress { scanned });
        });

        let total = self.index.images.len();
        events.push(AppEvent::ScanComplete { total });

        events.push(AppEvent::DirectoryLoaded {
            path: dir.to_path_buf(),
            total,
        });

        if total == 0 {
            events.push(AppEvent::LibraryEmpty);
        } else {
            events.push(AppEvent::PageChanged(self.build_page_state()));
        }

        events.push(AppEvent::StatsChanged(self.build_stats()));
        events
    }

    // ── Navigation ────────────────────────────────────────────────────────────

    pub fn next(&mut self) -> Vec<AppEvent> {
        let total = self.index.images.len();
        let before = self.nav.current_index;
        self.nav.next(total);

        if self.nav.current_index == before {
            vec![AppEvent::NavigationBoundary { kind: BoundaryKind::LastPage }]
        } else {
            vec![AppEvent::PageChanged(self.build_page_state())]
        }
    }

    pub fn prev(&mut self) -> Vec<AppEvent> {
        let before = self.nav.current_index;
        self.nav.prev();

        if self.nav.current_index == before {
            vec![AppEvent::NavigationBoundary { kind: BoundaryKind::FirstPage }]
        } else {
            vec![AppEvent::PageChanged(self.build_page_state())]
        }
    }

    // ── View count ────────────────────────────────────────────────────────────

    /// Change how many images are shown per page.
    /// Clamps navigation to keep it valid, then emits PageChanged.
    pub fn set_view_count(&mut self, count: usize) -> Vec<AppEvent> {
        assert!(count > 0, "view_count must be > 0");
        self.nav.view_count = count;
        self.clamp_nav();
        vec![
            AppEvent::ViewCountChanged { view_count: count },
            AppEvent::PageChanged(self.build_page_state()),
        ]
    }

    // ── Sort order ────────────────────────────────────────────────────────────

    /// Change the sort order. If the order is unchanged, no events are emitted.
    pub fn set_sort_order(&mut self, order: SortOrder) -> Vec<AppEvent> {
        if !self.index.set_sort_order(order.clone()) {
            return vec![]; // already in this order — no-op
        }
        self.nav.current_index = 0; // reset to first page after re-sort
        vec![
            AppEvent::SortChanged { order },
            AppEvent::PageChanged(self.build_page_state()),
        ]
    }

    // ── Actions ───────────────────────────────────────────────────────────────

    pub fn act_on_current(&mut self, action: Action) -> std::io::Result<Vec<AppEvent>> {
        self.act_on_current_at(action, 0)
    }

    pub fn act_on_current_at(
        &mut self,
        action: Action,
        view_index: usize,
    ) -> std::io::Result<Vec<AppEvent>> {
        let path = match self.image_at(view_index) {
            Some(entry) => entry.path.clone(),
            None => return Ok(vec![]),
        };

        let base_dir = path
            .parent()
            .expect("image has parent directory")
            .to_path_buf();

        let move_action = match action {
            Action::Select => MoveAction::Select,
            Action::Reject => MoveAction::Reject,
        };

        let subdir = match move_action {
            MoveAction::Select => "selected",
            MoveAction::Reject => "rejected",
        };

        let dest = crate::file_ops::move_to_subdir(&path, &base_dir, subdir)?;

        // Record undo entry before mutating index
        self.undo_stack.push(UndoEntry {
            current_path: dest.clone(),
            original_path: path.clone(),
            action: move_action.clone(),
        });

        self.index.remove_by_path(&path);
        self.cache.remove(&path);
        self.clamp_nav();

        let mut events = vec![AppEvent::FileMoved {
            from: path,
            to: dest,
            action: move_action,
        }];

        if self.index.images.is_empty() {
            events.push(AppEvent::LibraryEmpty);
        } else {
            events.push(AppEvent::PageChanged(self.build_page_state()));
        }

        events.push(AppEvent::StatsChanged(self.build_stats()));
        Ok(events)
    }

    // ── Undo ──────────────────────────────────────────────────────────────────

    /// Undo the most recent select or reject.
    /// Moves the file back to its original location and re-inserts it
    /// into the index at the correct sorted position.
    pub fn undo(&mut self) -> std::io::Result<Vec<AppEvent>> {
        let entry = match self.undo_stack.pop() {
            Some(e) => e,
            None => return Ok(vec![AppEvent::UndoStackEmpty]),
        };

        // Move file back: current_path → original_path
        std::fs::rename(&entry.current_path, &entry.original_path)?;

        // Re-insert into index — scan re-reads the file so metadata is fresh
        let meta = std::fs::metadata(&entry.original_path).ok();
        let file_size = meta.as_ref().map(|m| m.len()).unwrap_or(0);
        let date_modified = meta
            .and_then(|m| m.modified().ok())
            .unwrap_or(std::time::SystemTime::UNIX_EPOCH);
        let filename = entry.original_path
            .file_name()
            .unwrap()
            .to_string_lossy()
            .to_string();

        self.index.images.push(crate::image_index::ImageEntry {
            path: entry.original_path.clone(),
            filename,
            file_size,
            date_modified,
        });

        // Re-apply current sort so the restored entry lands in the right position
        self.index.resort();
        self.clamp_nav();

        let mut events = vec![AppEvent::Undone {
            path: entry.original_path,
            action: entry.action,
        }];

        events.push(AppEvent::PageChanged(self.build_page_state()));
        events.push(AppEvent::StatsChanged(self.build_stats()));
        Ok(events)
    }

    // ── Queries ───────────────────────────────────────────────────────────────

    pub fn total_images(&self) -> usize {
        self.index.images.len()
    }

    pub fn can_undo(&self) -> bool {
        !self.undo_stack.is_empty()
    }

    /// Returns current library stats (remaining / selected / rejected).
    pub fn stats(&self) -> LibraryStats {
        self.build_stats()
    }

    /// Returns images on the current page plus any stale-removal events.
    pub fn current_images(&mut self) -> (Vec<Image>, Vec<AppEvent>) {
        let mut events = Vec::new();
        let mut result = Vec::new();

        let total = self.index.images.len();
        let (start, end) = self.nav.range(total);
        let slice: Vec<_> = self.index.images[start..end].to_vec();

        for entry in slice {
            if entry.path.exists() {
                result.push(self.cache.get(&entry.path).clone());
            } else {
                events.push(AppEvent::StaleEntryRemoved {
                    path: entry.path.clone(),
                });
                self.index.remove_by_path(&entry.path);
                self.cache.remove(&entry.path);
            }
        }

        self.clamp_nav();

        if !events.is_empty() {
            if self.index.images.is_empty() {
                events.push(AppEvent::LibraryEmpty);
            } else {
                events.push(AppEvent::PageChanged(self.build_page_state()));
            }
            events.push(AppEvent::StatsChanged(self.build_stats()));
        }

        (result, events)
    }

    // ── Private helpers ───────────────────────────────────────────────────────

    fn image_at(&self, view_index: usize) -> Option<&crate::image_index::ImageEntry> {
        let total = self.index.images.len();
        let (start, end) = self.nav.range(total);
        self.index.images[start..end].get(view_index)
    }

    fn build_page_state(&self) -> PageState {
        let total = self.index.images.len();
        let (start, end) = self.nav.range(total);
        let view_count = self.nav.view_count;

        let images = self.index.images[start..end]
            .iter()
            .map(|e| {
                // Use cached entry if present — it may have richer metadata.
                // Otherwise build a pending Image but carry file_size across
                // from the index entry, which already has it from scan time.
                self.cache
                    .get_cached(&e.path)
                    .cloned()
                    .unwrap_or_else(|| Image {
                        file_size: Some(e.file_size),
                        ..Image::pending(e.path.clone())
                    })
            })
            .collect();

        let total_pages = if total == 0 { 1 } else { (total + view_count - 1) / view_count };
        let current_page = self.nav.current_index / view_count;

        PageState {
            images,
            current_index: self.nav.current_index,
            total,
            total_pages,
            current_page,
            view_count,
        }
    }

    fn build_stats(&self) -> LibraryStats {
        let remaining = self.index.images.len();
        let base = self.base_dir.as_deref();

        let (selected, rejected) = match base {
            Some(dir) => (
                count_in_subdir(dir, "selected"),
                count_in_subdir(dir, "rejected"),
            ),
            None => (0, 0),
        };

        LibraryStats { remaining, selected, rejected }
    }

    fn clamp_nav(&mut self) {
        let total = self.index.images.len();
        let page = self.nav.view_count;
        if total == 0 {
            self.nav.current_index = 0;
        } else {
            let max_page_start = ((total - 1) / page) * page;
            if self.nav.current_index > max_page_start {
                self.nav.current_index = max_page_start;
            }
        }
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::AppEvent;
    use crate::image_index::SortOrder;
    use std::fs;
    use tempfile::tempdir;

    // ── helpers ────────────────────────────────────────────────────────────────

    fn images(app: &mut AppState) -> Vec<Image> {
        app.current_images().0
    }

    fn has_page_changed(events: &[AppEvent]) -> bool {
        events.iter().any(|e| matches!(e, AppEvent::PageChanged(_)))
    }

    fn has_library_empty(events: &[AppEvent]) -> bool {
        events.iter().any(|e| matches!(e, AppEvent::LibraryEmpty))
    }

    fn has_stats_changed(events: &[AppEvent]) -> bool {
        events.iter().any(|e| matches!(e, AppEvent::StatsChanged(_)))
    }

    fn extract_stats(events: &[AppEvent]) -> Option<&LibraryStats> {
        events.iter().find_map(|e| {
            if let AppEvent::StatsChanged(s) = e { Some(s) } else { None }
        })
    }

    // ── existing behaviour ─────────────────────────────────────────────────────

    #[test]
    fn load_dir_emits_scan_started_progress_complete() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("a.jpg"), "").unwrap();
        fs::write(dir.path().join("b.jpg"), "").unwrap();
        fs::write(dir.path().join("c.jpg"), "").unwrap();

        let mut app = AppState::new(4);
        let events = app.load_dir(dir.path());

        // ScanStarted is first
        assert!(matches!(&events[0], AppEvent::ScanStarted { .. }));

        // Three ScanProgress events — one per image
        let progress: Vec<usize> = events.iter().filter_map(|e| {
            if let AppEvent::ScanProgress { scanned } = e { Some(*scanned) } else { None }
        }).collect();
        assert_eq!(progress.len(), 3);
        assert_eq!(progress, vec![1, 2, 3]);

        // ScanComplete carries the final total
        let complete = events.iter().find_map(|e| {
            if let AppEvent::ScanComplete { total } = e { Some(total) } else { None }
        });
        assert_eq!(complete, Some(&3));

        // Followed by DirectoryLoaded then PageChanged
        assert!(events.iter().any(|e| matches!(e, AppEvent::DirectoryLoaded { total: 3, .. })));
        assert!(has_page_changed(&events));
    }

    #[test]
    fn load_empty_dir_emits_scan_events_with_zero() {
        let dir = tempdir().unwrap();
        let mut app = AppState::new(1);
        let events = app.load_dir(dir.path());

        assert!(matches!(&events[0], AppEvent::ScanStarted { .. }));

        // No ScanProgress events for empty dir
        assert!(!events.iter().any(|e| matches!(e, AppEvent::ScanProgress { .. })));

        assert!(events.iter().any(|e| matches!(e, AppEvent::ScanComplete { total: 0 })));
        assert!(has_library_empty(&events));
    }

    #[test]
    fn scan_started_path_matches_loaded_dir() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("a.jpg"), "").unwrap();

        let mut app = AppState::new(1);
        let events = app.load_dir(dir.path());

        let started_path = events.iter().find_map(|e| {
            if let AppEvent::ScanStarted { path } = e { Some(path) } else { None }
        }).unwrap();

        assert_eq!(started_path, dir.path());
    }

    #[test]
    fn load_dir_emits_directory_loaded_page_and_stats() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("a.jpg"), "").unwrap();

        let mut app = AppState::new(1);
        let events = app.load_dir(dir.path());

        assert!(events.iter().any(|e| matches!(e, AppEvent::DirectoryLoaded { total: 1, .. })));
        assert!(has_page_changed(&events));
        assert!(has_stats_changed(&events));
    }

    #[test]
    fn load_empty_dir_emits_library_empty_and_stats() {
        let dir = tempdir().unwrap();
        let mut app = AppState::new(1);
        let events = app.load_dir(dir.path());
        assert!(has_library_empty(&events));
        assert!(has_stats_changed(&events));
    }

    #[test]
    fn load_dir_resets_navigation() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("a.jpg"), "").unwrap();
        fs::write(dir.path().join("b.jpg"), "").unwrap();

        let mut app = AppState::new(1);
        app.load_dir(dir.path());
        assert_eq!(app.total_images(), 2);

        let imgs = images(&mut app);
        assert_eq!(imgs.len(), 1);
        assert_eq!(imgs[0].path.file_name().unwrap().to_string_lossy(), "a.jpg");
    }

    #[test]
    fn next_and_prev_navigate_pages() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("a.jpg"), "").unwrap();
        fs::write(dir.path().join("b.jpg"), "").unwrap();
        fs::write(dir.path().join("c.jpg"), "").unwrap();

        let mut app = AppState::new(2);
        app.load_dir(dir.path());

        app.next();
        let second = images(&mut app);
        assert_eq!(second.len(), 1);
        assert_eq!(second[0].path.file_name().unwrap().to_string_lossy(), "c.jpg");

        app.prev();
        let first = images(&mut app);
        assert_eq!(first.len(), 2);
    }

    #[test]
    fn next_at_last_page_emits_boundary() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("a.jpg"), "").unwrap();

        let mut app = AppState::new(4);
        app.load_dir(dir.path());
        let events = app.next();

        assert!(events.iter().any(|e| matches!(e,
            AppEvent::NavigationBoundary { kind: BoundaryKind::LastPage }
        )));
    }

    #[test]
    fn prev_at_first_page_emits_boundary() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("a.jpg"), "").unwrap();

        let mut app = AppState::new(4);
        app.load_dir(dir.path());
        let events = app.prev();

        assert!(events.iter().any(|e| matches!(e,
            AppEvent::NavigationBoundary { kind: BoundaryKind::FirstPage }
        )));
    }

    #[test]
    fn select_moves_file_emits_events_and_updates_stats() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("a.jpg"), "data").unwrap();
        fs::write(dir.path().join("b.jpg"), "data").unwrap();

        let mut app = AppState::new(2);
        app.load_dir(dir.path());
        let events = app.act_on_current(Action::Select).unwrap();

        assert!(dir.path().join("selected/a.jpg").exists());
        assert!(events.iter().any(|e| matches!(e, AppEvent::FileMoved {
            action: MoveAction::Select, ..
        })));
        assert!(has_page_changed(&events));

        let stats = extract_stats(&events).unwrap();
        assert_eq!(stats.selected, 1);
        assert_eq!(stats.remaining, 1);
    }

    #[test]
    fn reject_moves_file_and_updates_stats() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("a.jpg"), "data").unwrap();

        let mut app = AppState::new(1);
        app.load_dir(dir.path());
        app.act_on_current(Action::Reject).unwrap();

        assert!(dir.path().join("rejected/a.jpg").exists());
        assert_eq!(app.total_images(), 0);
    }

    #[test]
    fn select_last_image_emits_library_empty() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("a.jpg"), "data").unwrap();

        let mut app = AppState::new(1);
        app.load_dir(dir.path());
        let events = app.act_on_current(Action::Select).unwrap();

        assert!(has_library_empty(&events));
        assert!(!has_page_changed(&events));
    }

    #[test]
    fn act_on_empty_state_is_safe() {
        let dir = tempdir().unwrap();
        let mut app = AppState::new(1);
        app.load_dir(dir.path());
        let events = app.act_on_current(Action::Select).unwrap();
        assert!(events.is_empty()); // invalid index — no-op
    }

    #[test]
    fn act_on_specific_index() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("a.jpg"), "").unwrap();
        fs::write(dir.path().join("b.jpg"), "").unwrap();
        fs::write(dir.path().join("c.jpg"), "").unwrap();

        let mut app = AppState::new(4);
        app.load_dir(dir.path());
        app.act_on_current_at(Action::Select, 1).unwrap();

        assert!(dir.path().join("selected/b.jpg").exists());
        assert_eq!(app.total_images(), 2);
    }

    #[test]
    fn stale_file_removed_automatically() {
        let dir = tempdir().unwrap();
        let img = dir.path().join("a.jpg");
        fs::write(&img, "data").unwrap();

        let mut app = AppState::new(1);
        app.load_dir(dir.path());
        fs::remove_file(&img).unwrap();

        let (imgs, events) = app.current_images();
        assert!(imgs.is_empty());
        assert_eq!(app.total_images(), 0);
        assert!(events.iter().any(|e| matches!(e, AppEvent::StaleEntryRemoved { .. })));
    }

    // ── undo ──────────────────────────────────────────────────────────────────

    #[test]
    fn undo_restores_file_and_emits_events() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("a.jpg"), "data").unwrap();

        let mut app = AppState::new(1);
        app.load_dir(dir.path());
        app.act_on_current(Action::Select).unwrap();

        assert!(!dir.path().join("a.jpg").exists());
        assert_eq!(app.total_images(), 0);

        let events = app.undo().unwrap();

        assert!(dir.path().join("a.jpg").exists());
        assert_eq!(app.total_images(), 1);
        assert!(events.iter().any(|e| matches!(e, AppEvent::Undone { .. })));
        assert!(has_page_changed(&events));
        assert!(has_stats_changed(&events));
    }

    #[test]
    fn undo_empty_stack_emits_undo_stack_empty() {
        let dir = tempdir().unwrap();
        let mut app = AppState::new(1);
        app.load_dir(dir.path());

        let events = app.undo().unwrap();
        assert!(events.iter().any(|e| matches!(e, AppEvent::UndoStackEmpty)));
    }

    #[test]
    fn can_undo_reflects_stack_state() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("a.jpg"), "data").unwrap();

        let mut app = AppState::new(1);
        app.load_dir(dir.path());
        assert!(!app.can_undo());

        app.act_on_current(Action::Select).unwrap();
        assert!(app.can_undo());

        app.undo().unwrap();
        assert!(!app.can_undo());
    }

    #[test]
    fn load_dir_clears_undo_stack() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("a.jpg"), "data").unwrap();

        let mut app = AppState::new(1);
        app.load_dir(dir.path());
        app.act_on_current(Action::Select).unwrap();
        assert!(app.can_undo());

        // Re-loading same dir should clear undo
        fs::write(dir.path().join("b.jpg"), "data").unwrap();
        app.load_dir(dir.path());
        assert!(!app.can_undo());
    }

    // ── set_view_count ────────────────────────────────────────────────────────

    #[test]
    fn set_view_count_emits_view_count_changed_and_page_changed() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("a.jpg"), "").unwrap();
        fs::write(dir.path().join("b.jpg"), "").unwrap();

        let mut app = AppState::new(1);
        app.load_dir(dir.path());
        let events = app.set_view_count(2);

        assert!(events.iter().any(|e| matches!(e,
            AppEvent::ViewCountChanged { view_count: 2 }
        )));
        assert!(has_page_changed(&events));
    }

    #[test]
    fn set_view_count_changes_page_size() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("a.jpg"), "").unwrap();
        fs::write(dir.path().join("b.jpg"), "").unwrap();
        fs::write(dir.path().join("c.jpg"), "").unwrap();

        let mut app = AppState::new(1);
        app.load_dir(dir.path());

        app.set_view_count(3);
        let imgs = images(&mut app);
        assert_eq!(imgs.len(), 3);
    }

    // ── set_sort_order ────────────────────────────────────────────────────────

    #[test]
    fn set_sort_order_reorders_images() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("a.jpg"), "").unwrap();
        fs::write(dir.path().join("z.jpg"), "").unwrap();

        let mut app = AppState::new(2);
        app.load_dir(dir.path());

        app.set_sort_order(SortOrder::NameDesc);
        let imgs = images(&mut app);
        assert_eq!(imgs[0].path.file_name().unwrap().to_string_lossy(), "z.jpg");
    }

    #[test]
    fn set_sort_order_same_order_is_noop() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("a.jpg"), "").unwrap();

        let mut app = AppState::new(1);
        app.load_dir(dir.path());

        let events = app.set_sort_order(SortOrder::NameAsc); // already default
        assert!(events.is_empty());
    }

    #[test]
    fn set_sort_order_emits_sort_changed_and_page_changed() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("a.jpg"), "").unwrap();

        let mut app = AppState::new(1);
        app.load_dir(dir.path());
        let events = app.set_sort_order(SortOrder::NameDesc);

        assert!(events.iter().any(|e| matches!(e, AppEvent::SortChanged { .. })));
        assert!(has_page_changed(&events));
    }

    // ── stats ─────────────────────────────────────────────────────────────────

    #[test]
    fn stats_counts_remaining_selected_rejected() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("a.jpg"), "data").unwrap();
        fs::write(dir.path().join("b.jpg"), "data").unwrap();
        fs::write(dir.path().join("c.jpg"), "data").unwrap();

        let mut app = AppState::new(4);
        app.load_dir(dir.path());

        app.act_on_current_at(Action::Select, 0).unwrap();
        app.act_on_current_at(Action::Reject, 0).unwrap();

        let stats = app.stats();
        assert_eq!(stats.remaining, 1);
        assert_eq!(stats.selected, 1);
        assert_eq!(stats.rejected, 1);
        assert_eq!(stats.progress_percent(), 66);
    }

    #[test]
    fn page_state_includes_view_count() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("a.jpg"), "").unwrap();
        fs::write(dir.path().join("b.jpg"), "").unwrap();

        let mut app = AppState::new(2);
        let events = app.load_dir(dir.path());

        let page = events.iter().find_map(|e| {
            if let AppEvent::PageChanged(p) = e { Some(p) } else { None }
        }).unwrap();

        assert_eq!(page.view_count, 2);
        assert_eq!(page.total, 2);
        assert_eq!(page.total_pages, 1);
    }

    #[test]
    fn page_state_images_have_file_size_populated() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("a.jpg"), "hello").unwrap(); // 5 bytes

        let mut app = AppState::new(1);
        let events = app.load_dir(dir.path());

        let page = events.iter().find_map(|e| {
            if let AppEvent::PageChanged(p) = e { Some(p) } else { None }
        }).unwrap();

        assert_eq!(page.images.len(), 1);
        assert_eq!(
            page.images[0].file_size,
            Some(5),
            "file_size must be populated from ImageEntry, not left as None"
        );
    }
}