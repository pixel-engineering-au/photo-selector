use std::path::Path;

use crate::image_index::{ImageIndex};
use crate::navigation::NavigationEngine;
use crate::image_cache::{ImageCache, Image};

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

    pub fn load_dir(&mut self, dir: &Path) {
        self.index.scan_dir(dir);
        self.nav.current_index = 0;
    }

    pub fn current_images(&mut self) -> Vec<Image> {
        let total = self.index.images.len();
        let (start, end) = self.nav.range(total);

        self.index.images[start..end]
            .iter()
            .map(|entry| self.cache.get(&entry.path).clone())
            .collect()
    }


    pub fn next(&mut self) {
        let total = self.index.images.len();
        self.nav.next(total);
    }

    pub fn prev(&mut self) {
        self.nav.prev();
    }

    pub fn total_images(&self) -> usize {
        self.index.images.len()
    }

    pub fn act_on_current(&mut self, action: Action) -> std::io::Result<()> {
        self.act_on_current_at(action, 0)
    }

    pub fn act_on_current_at(
        &mut self,
        action: Action,
        view_index: usize,
    ) -> std::io::Result<()> {
        let images = self.current_images();

        let image = match images.get(view_index) {
            Some(img) => img.clone(),
            None => return Ok(()), // invalid index → no-op
        };

        let base_dir = image
            .path
            .parent()
            .expect("image has parent directory");

        let subdir = match action {
            Action::Select => "selected",
            Action::Reject => "rejected",
        };

        crate::file_ops::move_to_subdir(
            &image.path,
            base_dir,
            subdir,
        )?;

        self.index.remove_by_path(&image.path);

        let total = self.index.images.len();
        if self.nav.current_index >= total && total > 0 {
            self.nav.current_index = total - 1;
        }

        Ok(())
    }


}





#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn act_on_specific_index() {
        let dir = tempdir().unwrap();

        fs::write(dir.path().join("a.jpg"), "").unwrap();
        fs::write(dir.path().join("b.jpg"), "").unwrap();
        fs::write(dir.path().join("c.jpg"), "").unwrap();

        let mut app = AppState::new(4);
        app.load_dir(dir.path());

        // Select second image (index 1)
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
        let images = app.current_images();
        assert_eq!(images.len(), 1);
        let name = images[0]
            .path
            .file_name()
            .unwrap()
            .to_string_lossy();
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

        // first page
        let first = app.current_images();
        assert_eq!(first.len(), 2);
        
        let first_names: Vec<String> = first
            .iter()
            .map(|img| {
                img.path
                    .file_name()
                    .unwrap()
                    .to_string_lossy()
                    .to_string()
            })
            .collect();
        
        assert!(first_names.contains(&"a.jpg".to_string()));
        assert!(first_names.contains(&"b.jpg".to_string()));

        // next page
        app.next();
        let second = app.current_images();
        assert_eq!(second.len(), 1);

        let name = second[0]
            .path
            .file_name()
            .unwrap()
            .to_string_lossy();
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
        let images = app.current_images();
        let name = images[0]
            .path
            .file_name()
            .unwrap()
            .to_string_lossy();

        assert_eq!(name, "a.jpg");
    }

    #[test]
    fn select_moves_file_and_updates_state() {
        let dir = tempdir().unwrap();

        let img = dir.path().join("a.jpg");
        fs::write(&img, "data").unwrap();

        let mut app = AppState::new(1);
        app.load_dir(dir.path());

        app.act_on_current(Action::Select).unwrap();

        assert_eq!(app.total_images(), 0);
        assert!(dir.path().join("selected/a.jpg").exists());
    }

    #[test]
    fn reject_moves_file_and_updates_state() {
        let dir = tempdir().unwrap();

        let img = dir.path().join("b.jpg");
        fs::write(&img, "data").unwrap();

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
}
