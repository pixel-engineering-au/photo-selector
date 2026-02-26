use std::path::Path;

/// Counts of images in each state within a session.
/// Returned by `AppState::stats()` and emitted as `AppEvent::StatsChanged`.
///
/// Tauri note: add `#[derive(serde::Serialize)]` when wiring up Tauri.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct LibraryStats {
    /// Images still in the main directory (not yet actioned).
    pub remaining: usize,
    /// Images moved to selected/ since the directory was loaded.
    pub selected: usize,
    /// Images moved to rejected/ since the directory was loaded.
    pub rejected: usize,
}

impl LibraryStats {
    /// Total images in the session (remaining + selected + rejected).
    pub fn total_session(&self) -> usize {
        self.remaining + self.selected + self.rejected
    }

    /// Percentage of images actioned (0–100), rounded down.
    pub fn progress_percent(&self) -> u8 {
        let total = self.total_session();
        if total == 0 {
            return 0;
        }
        let actioned = self.selected + self.rejected;
        ((actioned * 100) / total).min(100) as u8
    }
}

/// Count how many files currently exist in `base_dir/subdir`.
/// Returns 0 if the directory doesn't exist yet.
pub fn count_in_subdir(base_dir: &Path, subdir: &str) -> usize {
    let target = base_dir.join(subdir);
    if !target.exists() {
        return 0;
    }
    std::fs::read_dir(&target)
        .map(|entries| {
            entries
                .flatten()
                .filter(|e| {
                    e.path().is_file() && is_supported_ext(&e.path())
                })
                .count()
        })
        .unwrap_or(0)
}

fn is_supported_ext(path: &Path) -> bool {
    matches!(
        path.extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_lowercase())
            .as_deref(),
        Some("jpg") | Some("jpeg") | Some("png")
    )
}


#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn total_session_sums_all_fields() {
        let stats = LibraryStats { remaining: 5, selected: 3, rejected: 2 };
        assert_eq!(stats.total_session(), 10);
    }

    #[test]
    fn progress_percent_zero_when_empty() {
        let stats = LibraryStats::default();
        assert_eq!(stats.progress_percent(), 0);
    }

    #[test]
    fn progress_percent_correct() {
        let stats = LibraryStats { remaining: 5, selected: 3, rejected: 2 };
        assert_eq!(stats.progress_percent(), 50); // 5/10
    }

    #[test]
    fn progress_percent_complete() {
        let stats = LibraryStats { remaining: 0, selected: 8, rejected: 2 };
        assert_eq!(stats.progress_percent(), 100);
    }

    #[test]
    fn count_in_subdir_returns_zero_when_missing() {
        let dir = tempdir().unwrap();
        assert_eq!(count_in_subdir(dir.path(), "selected"), 0);
    }

    #[test]
    fn count_in_subdir_counts_images() {
        let dir = tempdir().unwrap();
        let sel = dir.path().join("selected");
        fs::create_dir(&sel).unwrap();
        fs::write(sel.join("a.jpg"), "").unwrap();
        fs::write(sel.join("b.jpg"), "").unwrap();
        fs::write(sel.join("notes.txt"), "").unwrap(); // ignored

        assert_eq!(count_in_subdir(dir.path(), "selected"), 2);
    }
}