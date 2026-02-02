use std::fs;
use std::path::{Path, PathBuf};

pub fn move_to_subdir(
    src: &Path,
    base_dir: &Path,
    subdir: &str,
) -> std::io::Result<PathBuf> {
    let target_dir = base_dir.join(subdir);
    fs::create_dir_all(&target_dir)?;

    let filename = src
        .file_name()
        .expect("source has filename");

    let dest = target_dir.join(filename);

    fs::rename(src, &dest)?;
    Ok(dest)
}



#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn moves_file_into_subdir() {
        let dir = tempdir().unwrap();

        let src = dir.path().join("test.jpg");
        fs::write(&src, "data").unwrap();

        let dest = move_to_subdir(&src, dir.path(), "selected").unwrap();

        assert!(!src.exists());
        assert!(dest.exists());
        assert!(dest.ends_with("selected/test.jpg"));
    }
}
