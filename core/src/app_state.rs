use crate::image_index::ImageIndex;
use crate::navigation::NavigationEngine;
use crate::image_cache::{ImageCache, Image};
use crate::events::{AppEvent, BoundaryKind, MoveAction, PageState};

pub enum Action {
    Select,
    Reject,
}


pub struct AppState {
    index: ImageIndex,
    nav: NavigationEngine,
    cache: ImageCache,
}

impl AppState {
    pub fn new(view_count: usize) -> Self {
        Self {
            index: ImageIndex::new(),
            nav: NavigationEngine::new(view_count),
            cache: ImageCache::new(),
        }
    }

    pub fn load_dir(&mut self, dir: &std::path::Path) -> Vec<AppEvent> {
        self.index.scan_dir(dir);
        self.nav.current_index = 0;

        let total = self.index.images.len();
        let mut events = vec![AppEvent::DirectoryLoaded {
            path: dir.to_path_buf(),
            total,
        }];

        if total == 0 {
            events.push(AppEvent::LibraryEmpty);
        } else {
            events.push(AppEvent::PageChanged(self.build_page_state()));
        }

        events
    }

    /// Returns the images on the current page plus any events that fired
    /// during the call (e.g. stale entry removals).
    /// The CLI uses the Vec<Image> directly; Tauri forwards the events.
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

        // If stale entries were removed, append an updated PageChanged
        if !events.is_empty() {
            if self.index.images.is_empty() {
                events.push(AppEvent::LibraryEmpty);
            } else {
                events.push(AppEvent::PageChanged(self.build_page_state()));
            }
        }

        (result, events)
    }


    pub fn next(&mut self) -> Vec<AppEvent> {
        let total = self.index.images.len();
        let before = self.nav.current_index;
        self.nav.next(total);

        if self.nav.current_index == before {
            // Already on last page — tell callers so they can disable the button
            vec![AppEvent::NavigationBoundary { kind: BoundaryKind::LastPage }]
        } else {
            vec![AppEvent::PageChanged(self.build_page_state())]
        }
    }

    pub fn prev(&mut self) -> Vec<AppEvent> {
        let before = self.nav.current_index;
        self.nav.prev();

        if self.nav.current_index == before {
            // Already on first page
            vec![AppEvent::NavigationBoundary { kind: BoundaryKind::FirstPage }]
        } else {
            vec![AppEvent::PageChanged(self.build_page_state())]
        }
    }

    pub fn total_images(&self) -> usize {
        self.index.images.len()
    }

    /// Returns the ImageEntry at `view_index` within the current page window.
    /// Pure — no side effects, no stale cleanup, no nav mutation.
    fn image_at(&self, view_index: usize) -> Option<&crate::image_index::ImageEntry> {
        let total = self.index.images.len();
        let (start, end) = self.nav.range(total);
        let page_entries = &self.index.images[start..end];
        page_entries.get(view_index)
    }

    /// Builds a PageState snapshot from current index + nav state.
    /// Pure — call any time after clamp_nav() has run.
    fn build_page_state(&self) -> PageState {
        let total = self.index.images.len();
        let (start, end) = self.nav.range(total);
        let images = self.index.images[start..end]
            .iter()
            .map(|e| Image { path: e.path.clone() })
            .collect();

        let view_count = self.nav.view_count;
        let total_pages = if total == 0 { 1 } else { (total + view_count - 1) / view_count };
        let current_page = if view_count == 0 { 0 } else { self.nav.current_index / view_count };

        PageState {
            images,
            current_index: self.nav.current_index,
            total,
            total_pages,
            current_page,
        }
    }

    /// Clamps `nav.current_index` so it always points to a valid page start.
    /// Must be called after any operation that changes `index.images.len()`.
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
            None => return Ok(vec![]), // invalid index → no-op, no events
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

        Ok(events)
    }


}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::AppEvent;
    use std::fs;
    use tempfile::tempdir;

    // ── helpers ────────────────────────────────────────────────────────────────

    /// Extract images from current_images(), discarding events.
    fn images(app: &mut AppState) -> Vec<Image> {
        app.current_images().0
    }

    /// Assert that a PageChanged event is present in the list.
    fn has_page_changed(events: &[AppEvent]) -> bool {
        events.iter().any(|e| matches!(e, AppEvent::PageChanged(_)))
    }

    /// Assert that LibraryEmpty event is present.
    fn has_library_empty(events: &[AppEvent]) -> bool {
        events.iter().any(|e| matches!(e, AppEvent::LibraryEmpty))
    }

    // ── existing behaviour ─────────────────────────────────────────────────────

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
    fn load_dir_resets_navigation() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("a.jpg"), "").unwrap();
        fs::write(dir.path().join("b.jpg"), "").unwrap();

        let mut app = AppState::new(1);
        app.load_dir(dir.path());

        assert_eq!(app.total_images(), 2);
        let imgs = images(&mut app);
        assert_eq!(imgs.len(), 1);
        let name = imgs[0].path.file_name().unwrap().to_string_lossy();
        assert_eq!(name, "a.jpg");
    }

    #[test]
    fn next_moves_through_images() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("a.jpg"), "").unwrap();
        fs::write(dir.path().join("b.jpg"), "").unwrap();
        fs::write(dir.path().join("c.jpg"), "").unwrap();

        let mut app = AppState::new(2);
        app.load_dir(dir.path());

        let first = images(&mut app);
        assert_eq!(first.len(), 2);
        let first_names: Vec<String> = first
            .iter()
            .map(|img| img.path.file_name().unwrap().to_string_lossy().to_string())
            .collect();
        assert!(first_names.contains(&"a.jpg".to_string()));
        assert!(first_names.contains(&"b.jpg".to_string()));

        app.next();
        let second = images(&mut app);
        assert_eq!(second.len(), 1);
        let name = second[0].path.file_name().unwrap().to_string_lossy();
        assert_eq!(name, "c.jpg");
    }

    #[test]
    fn prev_goes_back() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("a.jpg"), "").unwrap();
        fs::write(dir.path().join("b.jpg"), "").unwrap();
        fs::write(dir.path().join("c.jpg"), "").unwrap();

        let mut app = AppState::new(2);
        app.load_dir(dir.path());

        app.next();
        app.prev();
        let imgs = images(&mut app);
        let name = imgs[0].path.file_name().unwrap().to_string_lossy();
        assert_eq!(name, "a.jpg");
    }

    #[test]
    fn select_moves_file_and_updates_state() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("a.jpg"), "data").unwrap();

        let mut app = AppState::new(1);
        app.load_dir(dir.path());
        app.act_on_current(Action::Select).unwrap();

        assert_eq!(app.total_images(), 0);
        assert!(dir.path().join("selected/a.jpg").exists());
    }

    #[test]
    fn reject_moves_file_and_updates_state() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("b.jpg"), "data").unwrap();

        let mut app = AppState::new(1);
        app.load_dir(dir.path());
        app.act_on_current(Action::Reject).unwrap();

        assert_eq!(app.total_images(), 0);
        assert!(dir.path().join("rejected/b.jpg").exists());
    }

    #[test]
    fn act_on_empty_state_is_safe() {
        let dir = tempdir().unwrap();
        let mut app = AppState::new(1);
        app.load_dir(dir.path());
        app.act_on_current(Action::Select).unwrap();
        assert_eq!(app.total_images(), 0);
    }

    #[test]
    fn stale_file_is_removed_automatically() {
        let dir = tempdir().unwrap();
        let img = dir.path().join("a.jpg");
        fs::write(&img, "data").unwrap();

        let mut app = AppState::new(1);
        app.load_dir(dir.path());
        fs::remove_file(&img).unwrap();

        let (imgs, _events) = app.current_images();
        assert!(imgs.is_empty());
        assert_eq!(app.total_images(), 0);
    }

    // ── new event-shape tests ──────────────────────────────────────────────────

    #[test]
    fn load_dir_emits_directory_loaded_and_page_changed() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("a.jpg"), "").unwrap();

        let mut app = AppState::new(1);
        let events = app.load_dir(dir.path());

        assert!(events.iter().any(|e| matches!(e, AppEvent::DirectoryLoaded { total: 1, .. })));
        assert!(has_page_changed(&events));
    }

    #[test]
    fn load_empty_dir_emits_library_empty() {
        let dir = tempdir().unwrap();
        let mut app = AppState::new(1);
        let events = app.load_dir(dir.path());
        assert!(has_library_empty(&events));
    }

    #[test]
    fn next_emits_page_changed() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("a.jpg"), "").unwrap();
        fs::write(dir.path().join("b.jpg"), "").unwrap();
        fs::write(dir.path().join("c.jpg"), "").unwrap();

        let mut app = AppState::new(2);
        app.load_dir(dir.path());
        let events = app.next();
        assert!(has_page_changed(&events));
    }

    #[test]
    fn next_at_last_page_emits_boundary() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("a.jpg"), "").unwrap();

        let mut app = AppState::new(4);
        app.load_dir(dir.path());
        let events = app.next(); // already on only page
        assert!(events
            .iter()
            .any(|e| matches!(e, AppEvent::NavigationBoundary { kind: BoundaryKind::LastPage })));
    }

    #[test]
    fn prev_at_first_page_emits_boundary() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("a.jpg"), "").unwrap();

        let mut app = AppState::new(4);
        app.load_dir(dir.path());
        let events = app.prev();
        assert!(events
            .iter()
            .any(|e| matches!(e, AppEvent::NavigationBoundary { kind: BoundaryKind::FirstPage })));
    }

    #[test]
    fn select_emits_file_moved_then_page_changed() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("a.jpg"), "data").unwrap();
        fs::write(dir.path().join("b.jpg"), "data").unwrap();

        let mut app = AppState::new(2);
        app.load_dir(dir.path());
        let events = app.act_on_current(Action::Select).unwrap();

        assert!(events.iter().any(|e| matches!(e, AppEvent::FileMoved {
            action: MoveAction::Select, ..
        })));
        assert!(has_page_changed(&events));
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
    fn stale_removal_emits_stale_event_and_page_changed() {
        let dir = tempdir().unwrap();
        let img = dir.path().join("a.jpg");
        fs::write(&img, "data").unwrap();
        fs::write(dir.path().join("b.jpg"), "data").unwrap();

        let mut app = AppState::new(4);
        app.load_dir(dir.path());
        fs::remove_file(&img).unwrap();

        let (_imgs, events) = app.current_images();
        assert!(events
            .iter()
            .any(|e| matches!(e, AppEvent::StaleEntryRemoved { .. })));
        assert!(has_page_changed(&events));
    }

    #[test]
    fn page_state_totals_are_correct() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("a.jpg"), "").unwrap();
        fs::write(dir.path().join("b.jpg"), "").unwrap();
        fs::write(dir.path().join("c.jpg"), "").unwrap();

        let mut app = AppState::new(2);
        let events = app.load_dir(dir.path());

        let page = events.iter().find_map(|e| {
            if let AppEvent::PageChanged(p) = e { Some(p) } else { None }
        }).unwrap();

        assert_eq!(page.total, 3);
        assert_eq!(page.total_pages, 2);
        assert_eq!(page.current_page, 0);
    }
}