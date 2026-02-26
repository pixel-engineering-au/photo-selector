use std::path::{Path, PathBuf};
use std::time::SystemTime;

/// How the image list should be ordered.
///
/// Tauri note: add `#[derive(serde::Serialize, serde::Deserialize)]`
/// when wiring up Tauri so the frontend can send sort commands.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum SortOrder {
    /// A → Z by filename (default).
    #[default]
    NameAsc,
    /// Z → A by filename.
    NameDesc,
    /// Oldest modified first.
    DateModifiedAsc,
    /// Newest modified first.
    DateModifiedDesc,
    /// Smallest file first.
    SizeAsc,
    /// Largest file first.
    SizeDesc,
}

#[derive(Debug, Clone)]
pub struct ImageEntry {
    pub path: PathBuf,
    pub filename: String,
    /// Captured at scan time — used for date/size sorting without extra stat() calls.
    pub file_size: u64,
    pub date_modified: SystemTime,
}

#[derive(Default)]
pub struct ImageIndex {
    pub images: Vec<ImageEntry>,
    sort_order: SortOrder,
}

impl ImageIndex {
    pub fn new() -> Self {
        Self {
            images: Vec::new(),
            sort_order: SortOrder::default(),
        }
    }

    pub fn scan_dir(&mut self, dir: &Path) {
        self.scan_dir_with_progress(dir, &mut |_| {});
    }

    /// Scan `dir` for supported images, calling `on_progress(scanned_so_far)`
    /// after each image is discovered.
    ///
    /// The callback receives the running count (1, 2, 3 ...) so callers can
    /// emit `ScanProgress` events or update a UI counter without this module
    /// knowing anything about events or channels.
    ///
    /// Tauri migration path: replace the closure with a channel sender —
    /// the signature and internal logic stay identical.
    pub fn scan_dir_with_progress(
        &mut self,
        dir: &Path,
        on_progress: &mut impl FnMut(usize),
    ) {
        self.images.clear();

        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() && Self::is_supported(&path) {
                    let meta = std::fs::metadata(&path).ok();
                    let file_size = meta.as_ref().map(|m| m.len()).unwrap_or(0);
                    let date_modified = meta
                        .and_then(|m| m.modified().ok())
                        .unwrap_or(SystemTime::UNIX_EPOCH);

                    let filename = path
                        .file_name()
                        .unwrap()
                        .to_string_lossy()
                        .to_string();

                    self.images.push(ImageEntry {
                        path,
                        filename,
                        file_size,
                        date_modified,
                    });

                    on_progress(self.images.len());
                }
            }
        }

        self.apply_sort();
    }

    /// Change sort order and re-sort in place.
    /// Returns true if the order actually changed.
    pub fn set_sort_order(&mut self, order: SortOrder) -> bool {
        if self.sort_order == order {
            return false;
        }
        self.sort_order = order;
        self.apply_sort();
        true
    }

    /// Re-sort using the current order without changing it.
    /// Used after manually inserting entries (e.g. undo).
    pub fn resort(&mut self) {
        self.apply_sort();
    }

    pub fn current_sort_order(&self) -> &SortOrder {
        &self.sort_order
    }

    fn apply_sort(&mut self) {
        match self.sort_order {
            SortOrder::NameAsc => {
                self.images.sort_by(|a, b| a.filename.cmp(&b.filename));
            }
            SortOrder::NameDesc => {
                self.images.sort_by(|a, b| b.filename.cmp(&a.filename));
            }
            SortOrder::DateModifiedAsc => {
                self.images.sort_by(|a, b| a.date_modified.cmp(&b.date_modified));
            }
            SortOrder::DateModifiedDesc => {
                self.images.sort_by(|a, b| b.date_modified.cmp(&a.date_modified));
            }
            SortOrder::SizeAsc => {
                self.images.sort_by_key(|e| e.file_size);
            }
            SortOrder::SizeDesc => {
                self.images.sort_by(|a, b| b.file_size.cmp(&a.file_size));
            }
        }
    }

    fn is_supported(path: &Path) -> bool {
        matches!(
            path.extension()
                .and_then(|e| e.to_str())
                .map(|e| e.to_lowercase())
                .as_deref(),
            Some("jpg") | Some("jpeg") | Some("png")
        )
    }

    // O(n) — acceptable for user-triggered single removals.
    // If batch operations are ever added, switch to a HashMap<PathBuf, usize>
    // lookup table + swap_remove for O(1) removal.
    pub fn remove_by_path(&mut self, path: &Path) {
        self.images.retain(|img| img.path != path);
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn scans_only_supported_images() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("a.jpg"), "").unwrap();
        fs::write(dir.path().join("b.png"), "").unwrap();
        fs::write(dir.path().join("c.txt"), "").unwrap();

        let mut index = ImageIndex::new();
        index.scan_dir(dir.path());

        let names: Vec<String> = index.images.iter().map(|i| i.filename.clone()).collect();
        assert_eq!(names.len(), 2);
        assert!(names.contains(&"a.jpg".to_string()));
        assert!(names.contains(&"b.png".to_string()));
    }

    #[test]
    fn default_sort_is_name_asc() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("z.jpg"), "").unwrap();
        fs::write(dir.path().join("a.jpg"), "").unwrap();

        let mut index = ImageIndex::new();
        index.scan_dir(dir.path());

        assert_eq!(index.images[0].filename, "a.jpg");
        assert_eq!(index.images[1].filename, "z.jpg");
    }

    #[test]
    fn name_desc_sort() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("a.jpg"), "").unwrap();
        fs::write(dir.path().join("z.jpg"), "").unwrap();

        let mut index = ImageIndex::new();
        index.scan_dir(dir.path());
        index.set_sort_order(SortOrder::NameDesc);

        assert_eq!(index.images[0].filename, "z.jpg");
        assert_eq!(index.images[1].filename, "a.jpg");
    }

    #[test]
    fn size_sort() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("small.jpg"), "hi").unwrap();       // 2 bytes
        fs::write(dir.path().join("large.jpg"), "hello world").unwrap(); // 11 bytes

        let mut index = ImageIndex::new();
        index.scan_dir(dir.path());
        index.set_sort_order(SortOrder::SizeAsc);

        assert_eq!(index.images[0].filename, "small.jpg");
        assert_eq!(index.images[1].filename, "large.jpg");
    }

    #[test]
    fn size_desc_sort() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("small.jpg"), "hi").unwrap();
        fs::write(dir.path().join("large.jpg"), "hello world").unwrap();

        let mut index = ImageIndex::new();
        index.scan_dir(dir.path());
        index.set_sort_order(SortOrder::SizeDesc);

        assert_eq!(index.images[0].filename, "large.jpg");
    }

    #[test]
    fn set_same_sort_order_returns_false() {
        let mut index = ImageIndex::new();
        // default is NameAsc — setting it again should return false
        assert!(!index.set_sort_order(SortOrder::NameAsc));
    }

    #[test]
    fn set_different_sort_order_returns_true() {
        let mut index = ImageIndex::new();
        assert!(index.set_sort_order(SortOrder::NameDesc));
    }

    #[test]
    fn file_size_populated_at_scan() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("a.jpg"), "hello").unwrap(); // 5 bytes

        let mut index = ImageIndex::new();
        index.scan_dir(dir.path());

        assert_eq!(index.images[0].file_size, 5);
    }

    #[test]
    fn scan_dir_with_progress_fires_callback_per_image() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("a.jpg"), "").unwrap();
        fs::write(dir.path().join("b.jpg"), "").unwrap();
        fs::write(dir.path().join("c.jpg"), "").unwrap();
        fs::write(dir.path().join("skip.txt"), "").unwrap(); // not counted

        let mut index = ImageIndex::new();
        let mut counts: Vec<usize> = Vec::new();

        index.scan_dir_with_progress(dir.path(), &mut |n| counts.push(n));

        // One callback per supported image, in discovery order
        assert_eq!(counts.len(), 3);
        // Counts must be strictly increasing starting from 1
        assert_eq!(counts[0], 1);
        assert_eq!(counts[1], 2);
        assert_eq!(counts[2], 3);
    }

    #[test]
    fn scan_dir_plain_still_works() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("a.jpg"), "").unwrap();

        let mut index = ImageIndex::new();
        index.scan_dir(dir.path()); // must not panic

        assert_eq!(index.images.len(), 1);
    }

    #[test]
    fn scan_dir_with_progress_empty_dir_fires_no_callbacks() {
        let dir = tempdir().unwrap();
        let mut index = ImageIndex::new();
        let mut count = 0usize;

        index.scan_dir_with_progress(dir.path(), &mut |_| count += 1);

        assert_eq!(count, 0);
        assert!(index.images.is_empty());
    }

    #[test]
    fn remove_by_path_removes_entry() {
        let mut index = ImageIndex::new();
        let path = PathBuf::from("a.jpg");
        index.images.push(ImageEntry {
            path: path.clone(),
            filename: "a.jpg".to_string(),
            file_size: 0,
            date_modified: SystemTime::UNIX_EPOCH,
        });
        index.remove_by_path(&path);
        assert!(index.images.is_empty());
    }
}