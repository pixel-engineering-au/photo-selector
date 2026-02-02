use std::collections::HashMap;
use std::path::{Path, PathBuf};

// We'll wrap the actual image in a dummy struct for now
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Image {
    pub path: PathBuf,
    // later: decoded pixels / thumbnail
}

pub struct ImageCache {
    cache: HashMap<PathBuf, Image>,
}

impl ImageCache {
    pub fn new() -> Self { Self { cache: HashMap::new() } }

    /// Get image from cache, load if missing
    pub fn get(&mut self, path: &Path) -> &Image {
        let path_buf = path.to_path_buf();
        self.cache.entry(path_buf.clone()).or_insert_with(|| {
            // Simulate loading
            if !path.exists() {
                panic!("File does not exist: {:?}", path);
            }
            Image { path: path_buf }
        })
    }

    /// Number of cached images
    pub fn len(&self) -> usize {
        self.cache.len()
    }
}



#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    use std::fs;

    #[test]
    fn loads_image_into_cache() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("a.jpg");
        fs::write(&file, "").unwrap();

        let mut cache = ImageCache::new();
        let img = cache.get(&file);

        assert_eq!(img.path, file);
        assert_eq!(cache.len(), 1);
    }

    #[test]
    fn returns_same_reference_for_cached_image() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("b.jpg");
        fs::write(&file, "").unwrap();

        let mut cache = ImageCache::new();
        let first = cache.get(&file) as *const _;
        let second = cache.get(&file) as *const _;

        assert_eq!(first, second, "Cached reference should be reused");
        assert_eq!(cache.len(), 1);
    }

    #[test]
    #[should_panic]
    fn panics_on_missing_file() {
        let mut cache = ImageCache::new();
        let path = PathBuf::from("/non/existent/file.jpg");
        cache.get(&path);
    }
}