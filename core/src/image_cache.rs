use std::collections::HashMap;
use std::path::{Path, PathBuf};
//use std::time::SystemTime;

/// Load state of an image entry.
/// Starts as Pending — the GUI shows a skeleton.
/// Transitions to Ready once a thumbnail is generated,
/// or Failed if the file can't be read.
///
/// Tauri note: add `#[derive(serde::Serialize)]` when wiring up Tauri.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "tauri", derive(serde::Serialize, serde::Deserialize))]
pub enum ImageLoadState {
    /// Not yet loaded — show a loading skeleton in the GUI.
    Pending,
    /// Thumbnail bytes are ready — Vec<u8> is a PNG/JPEG blob.
    /// Populated by the thumbnail worker (not yet implemented).
    Ready { thumbnail: Vec<u8> },
    /// File could not be read — show an error tile in the GUI.
    Failed { reason: String },
}

/// Metadata for a single image.
/// All optional fields are `None` until populated by a background worker.
/// The struct shape is intentionally forward-looking so Tauri serialisation
/// never needs a breaking change when metadata loading is implemented.
///
/// Tauri note: add `#[derive(serde::Serialize)]` when wiring up Tauri.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "tauri", derive(serde::Serialize, serde::Deserialize))]
pub struct Image {
    pub path: PathBuf,

    /// Pixel dimensions — None until loaded.
    pub dimensions: Option<(u32, u32)>,

    /// File size in bytes — populated at scan time.
    pub file_size: Option<u64>,

    /// Date taken from EXIF, or file modified time as fallback — None until loaded.
    pub date_taken: Option<u64>,  // Unix timestamp seconds, None until loaded

    /// Current load/thumbnail state.
    pub load_state: ImageLoadState,
}

impl Image {
    /// Construct a minimal Image from a path (all metadata fields None / Pending).
    pub fn pending(path: PathBuf) -> Self {
        Self {
            path,
            dimensions: None,
            file_size: None,
            date_taken: None,
            load_state: ImageLoadState::Pending,
        }
    }

    /// True if this image has a usable thumbnail ready.
    pub fn is_ready(&self) -> bool {
        matches!(self.load_state, ImageLoadState::Ready { .. })
    }
}

pub struct ImageCache {
    cache: HashMap<PathBuf, Image>,
}

impl ImageCache {
    pub fn new() -> Self {
        Self { cache: HashMap::new() }
    }

    /// Get image from cache, inserting a Pending entry if not present.
    /// Stale-file detection is the caller's responsibility (see `current_images`).
    pub fn get(&mut self, path: &Path) -> &Image {
        self.cache
            .entry(path.to_path_buf())
            .or_insert_with(|| {
                // Opportunistically capture file_size at insert time —
                // this is a cheap stat() call with no pixel decoding.
                let file_size = std::fs::metadata(path).ok().map(|m| m.len());
                Image {
                    path: path.to_path_buf(),
                    dimensions: None,
                    file_size,
                    date_taken: None,
                    load_state: ImageLoadState::Pending,
                }
            })
    }

    /// Returns a shared reference without inserting — used by build_page_state.
    pub fn get_cached(&self, path: &Path) -> Option<&Image> {
        self.cache.get(path)
    }

    /// Mark an image as failed — called if the file can't be decoded.
    pub fn mark_failed(&mut self, path: &Path, reason: String) {
        if let Some(img) = self.cache.get_mut(path) {
            img.load_state = ImageLoadState::Failed { reason };
        }
    }

    /// Number of cached entries.
    pub fn len(&self) -> usize {
        self.cache.len()
    }

    pub fn is_empty(&self) -> bool {
        self.cache.is_empty()
    }

    pub fn remove(&mut self, path: &Path) {
        self.cache.remove(path);
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    use std::fs;

    #[test]
    fn new_entry_is_pending() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("a.jpg");
        fs::write(&file, "data").unwrap();

        let mut cache = ImageCache::new();
        let img = cache.get(&file);

        assert_eq!(img.load_state, ImageLoadState::Pending);
        assert_eq!(img.path, file);
    }

    #[test]
    fn file_size_populated_at_insert() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("a.jpg");
        fs::write(&file, "hello").unwrap(); // 5 bytes

        let mut cache = ImageCache::new();
        let img = cache.get(&file);

        assert_eq!(img.file_size, Some(5));
    }

    #[test]
    fn returns_same_entry_on_second_get() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("b.jpg");
        fs::write(&file, "").unwrap();

        let mut cache = ImageCache::new();
        let first = cache.get(&file) as *const _;
        let second = cache.get(&file) as *const _;

        assert_eq!(first, second, "cache must return the same entry");
        assert_eq!(cache.len(), 1);
    }

    #[test]
    fn mark_failed_updates_load_state() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("bad.jpg");
        fs::write(&file, "").unwrap();

        let mut cache = ImageCache::new();
        cache.get(&file);
        cache.mark_failed(&file, "corrupt".to_string());

        let img = cache.get(&file);
        assert!(matches!(img.load_state, ImageLoadState::Failed { .. }));
    }

    #[test]
    fn remove_clears_entry() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("c.jpg");
        fs::write(&file, "").unwrap();

        let mut cache = ImageCache::new();
        cache.get(&file);
        assert_eq!(cache.len(), 1);
        cache.remove(&file);
        assert_eq!(cache.len(), 0);
    }

    #[test]
    fn missing_file_insert_is_safe() {
        let mut cache = ImageCache::new();
        let path = PathBuf::from("/nonexistent/file.jpg");
        let img = cache.get(&path);
        // file_size will be None since stat() fails, but no panic
        assert_eq!(img.file_size, None);
        assert_eq!(img.load_state, ImageLoadState::Pending);
    }
}