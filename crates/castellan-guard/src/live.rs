//! Read-only directory walk that does not follow reparse points.
//!
//! Deletion is intentionally absent. The memory sweep tests the decision, and this
//! walk is what a future clean will list before it asks the kernel.

use std::path::Path;

use castellan_winapi::WinError;

/// Counts from a read-only walk.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct LiveScan {
    /// Ordinary files seen.
    pub files: u64,
    /// Directories entered.
    pub directories: u64,
    /// Reparse points skipped, including their children.
    pub reparse_skipped: u64,
}

/// Walk `root` without opening the target of a junction or symlink.
pub fn scan_no_follow(root: &Path) -> Result<LiveScan, WinError> {
    let mut scan = LiveScan::default();
    walk(root, &mut scan, 0)?;
    Ok(scan)
}

fn walk(dir: &Path, scan: &mut LiveScan, depth: u32) -> Result<(), WinError> {
    if depth > 32 {
        return Ok(());
    }
    let entries = std::fs::read_dir(dir).map_err(|_| WinError {
        operation: "read_dir",
        code: 0,
    })?;
    for entry in entries {
        let entry = entry.map_err(|_| WinError {
            operation: "read_dir",
            code: 0,
        })?;
        let path = entry.path();
        let attrs = castellan_winapi::file_attributes(&path)?;
        if attrs.reparse_point {
            scan.reparse_skipped += 1;
            continue;
        }
        if attrs.directory {
            scan.directories += 1;
            walk(&path, scan, depth + 1)?;
        } else {
            scan.files += 1;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scan_sees_a_file_it_created_and_does_not_delete_it() {
        let dir = std::env::temp_dir().join(format!("castellan-scan-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("dir");
        let file = dir.join("note.txt");
        std::fs::write(&file, b"keep").expect("write");
        let scan = scan_no_follow(&dir).expect("scan");
        assert_eq!(scan.files, 1);
        assert!(file.exists());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
