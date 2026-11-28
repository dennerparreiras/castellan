//! Read-only directory walk that does not follow reparse points.
//!
//! Deletion is intentionally absent. The memory sweep tests the decision, and this
//! walk is what a future clean will list before it asks the kernel.

use std::path::Path;
use std::time::SystemTime;

use castellan_winapi::WinError;

/// Sizes from a read-only walk. Nothing is opened for writing and nothing is deleted.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Measure {
    /// Ordinary files seen.
    pub files: u64,
    /// Logical size of those files.
    pub bytes: u64,
    /// Files whose creation and modification are both older than the cutoff.
    pub aged_files: u64,
    /// Logical size of the aged files.
    pub aged_bytes: u64,
    /// Reparse points skipped, including their children.
    pub reparse_skipped: u64,
    /// Entries that could not be read. They are counted, not guessed.
    pub unreadable: u64,
    /// The walk stopped at the entry limit, so the numbers are a floor.
    pub truncated: bool,
}

/// Measure `root` without following a junction or symlink.
///
/// `aged_before` is the cutoff the sweep would use: a file counts as aged only
/// when it was both created and last modified before it. `max_entries` bounds
/// the walk. This is an estimate of what a sweep would consider. A real sweep
/// also skips files that are in use.
pub fn measure_no_follow(
    root: &Path,
    aged_before: SystemTime,
    max_entries: u64,
) -> Result<Measure, WinError> {
    let mut measure = Measure::default();
    let mut seen = 0u64;
    let entries = std::fs::read_dir(root).map_err(|error| WinError {
        operation: "read_dir",
        code: error
            .raw_os_error()
            .and_then(|code| u32::try_from(code).ok())
            .unwrap_or(0),
    })?;
    measure_entries(
        entries,
        aged_before,
        max_entries,
        &mut seen,
        &mut measure,
        0,
    );
    Ok(measure)
}

fn measure_entries(
    entries: std::fs::ReadDir,
    aged_before: SystemTime,
    max_entries: u64,
    seen: &mut u64,
    measure: &mut Measure,
    depth: u32,
) {
    for entry in entries {
        if *seen >= max_entries {
            measure.truncated = true;
            return;
        }
        *seen += 1;
        let Ok(entry) = entry else {
            measure.unreadable += 1;
            continue;
        };
        let path = entry.path();
        let Ok(attrs) = castellan_winapi::file_attributes(&path) else {
            measure.unreadable += 1;
            continue;
        };
        if attrs.reparse_point {
            measure.reparse_skipped += 1;
            continue;
        }
        if attrs.directory {
            if depth >= 32 {
                measure.truncated = true;
                continue;
            }
            match std::fs::read_dir(&path) {
                Ok(children) => {
                    measure_entries(children, aged_before, max_entries, seen, measure, depth + 1);
                }
                Err(_) => measure.unreadable += 1,
            }
            continue;
        }
        // `DirEntry::metadata` does not traverse a link, and links were skipped above.
        let Ok(meta) = entry.metadata() else {
            measure.unreadable += 1;
            continue;
        };
        measure.files += 1;
        measure.bytes = measure.bytes.saturating_add(meta.len());
        let old = |time: std::io::Result<SystemTime>| time.is_ok_and(|time| time < aged_before);
        if old(meta.created()) && old(meta.modified()) {
            measure.aged_files += 1;
            measure.aged_bytes = measure.aged_bytes.saturating_add(meta.len());
        }
    }
}

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

    #[test]
    fn measure_counts_bytes_and_only_calls_old_files_aged() {
        let dir = std::env::temp_dir().join(format!("castellan-measure-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("inner")).expect("dir");
        std::fs::write(dir.join("a.tmp"), b"12345").expect("write");
        std::fs::write(dir.join("inner").join("b.tmp"), b"123").expect("write");
        let now = SystemTime::now();
        let past = now - std::time::Duration::from_secs(3_600);
        let future = now + std::time::Duration::from_secs(3_600);
        let fresh = measure_no_follow(&dir, past, 1_000).expect("measure");
        assert_eq!((fresh.files, fresh.bytes), (2, 8));
        assert_eq!((fresh.aged_files, fresh.aged_bytes), (0, 0));
        assert!(!fresh.truncated);
        let aged = measure_no_follow(&dir, future, 1_000).expect("measure");
        assert_eq!((aged.aged_files, aged.aged_bytes), (2, 8));
        let capped = measure_no_follow(&dir, future, 1).expect("measure");
        assert!(capped.truncated);
        assert!(dir.join("a.tmp").exists());
        assert!(dir.join("inner").join("b.tmp").exists());
        assert!(measure_no_follow(&dir.join("missing"), now, 10).is_err());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
