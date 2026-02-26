use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct ImageEntry {
    pub path: PathBuf,
    pub filename: String,
}

#[derive(Default)]
pub struct ImageIndex {
    pub images: Vec<ImageEntry>,
}

impl ImageIndex {
    pub fn new() -> Self {
        Self { images: Vec::new() }
    }

    pub fn scan_dir(&mut self, dir: &Path) {
        self.images.clear();

        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() && Self::is_supported(&path) {
                    let filename = path
                        .file_name()
                        .unwrap()
                        .to_string_lossy()
                        .to_string();

                    self.images.push(ImageEntry { path, filename });
                }
            }
        }

        self.images.sort_by(|a, b| a.filename.cmp(&b.filename));
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
    
    // O(n) — acceptable for user-triggered single removals; revisit if batch ops are added
    // O(n) linear scan — acceptable for user-triggered single removals.
    // If batch operations are ever added, switch to a HashMap<PathBuf, usize>
    // lookup table + swap_remove for O(1) removal while maintaining sort order
    // via a secondary sorted structure.
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
    fn remove_by_path_removes_image() {
        let mut index = ImageIndex::new();

        let path = PathBuf::from("a.jpg");

        index.images.push(ImageEntry {
            path: path.clone(),
            filename: "a.jpg".to_string(),
        });

        assert_eq!(index.images.len(), 1);

        index.remove_by_path(&path);

        assert!(index.images.is_empty());
    }

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
    fn images_are_sorted_by_filename() {
        let dir = tempdir().unwrap();

        fs::write(dir.path().join("z.jpg"), "").unwrap();
        fs::write(dir.path().join("a.jpg"), "").unwrap();

        let mut index = ImageIndex::new();
        index.scan_dir(dir.path());

        assert_eq!(index.images[0].filename, "a.jpg");
        assert_eq!(index.images[1].filename, "z.jpg");
    }
}
