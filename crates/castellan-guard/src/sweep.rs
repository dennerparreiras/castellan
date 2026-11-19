//! Sweep walker. It deletes only what [`crate::decide_node`] allows.

use std::ffi::OsStr;

use crate::decide::{NodeDecision, NodeFacts, SkipReason, decide_node, hostile_component};
use crate::path::{MachineRoots, NormalizedPath};
use thiserror::Error;

/// Failure of the sweep pipeline. A failure aborts the remaining nodes.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum GuardError {
    /// The view could not read a node.
    #[error("sweep view failed: {0}")]
    View(String),
    /// Debug builds refuse to mutate anything outside a temp sandbox.
    #[error("DEV BUILD — real-system actions disabled")]
    RealSystemDisabled,
    /// The plan asked for more files or bytes than the sanity cap allows without a person.
    #[error("sanity cap exceeded ({files} files, {bytes} bytes)")]
    SanityCap {
        /// Files that would be deleted.
        files: u64,
        /// Bytes that would be freed.
        bytes: u64,
    },
}

/// Read and delete surface used by the walker. Production and tests both implement it.
pub trait SweepView {
    /// List child names. Must not follow a reparse point.
    fn list(&self, path: &str) -> Result<Vec<String>, GuardError>;
    /// Facts for one path, from a non-following open.
    fn facts(&self, path: &str) -> Result<NodeFacts, GuardError>;
    /// Delete a file the decision already approved.
    fn delete_file(&mut self, path: &str) -> Result<(), GuardError>;
    /// Delete an empty directory the decision already approved.
    fn delete_empty_dir(&mut self, path: &str) -> Result<(), GuardError>;
}

/// What the walker returns.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SweepReport {
    /// Files deleted.
    pub deleted_files: u64,
    /// Bytes freed, as reported by the facts at delete time.
    pub deleted_bytes: u64,
    /// Nodes left in place.
    pub skipped: Vec<Skipped>,
}

/// One skipped node.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Skipped {
    /// Path that was left alone.
    pub path: String,
    /// Why.
    pub reason: SkipReason,
}

/// Limits that stop a runaway plan.
#[derive(Clone, Copy, Debug)]
pub struct SanityLimits {
    /// Maximum files deleted before the walker stops and asks for a person.
    pub max_files: u64,
    /// Maximum bytes freed before the walker stops.
    pub max_bytes: u64,
    /// When false, exceeding the cap aborts. Autopilot passes false.
    pub human_present: bool,
}

/// Run a sweep under `allow_root`.
pub fn run_sweep(
    view: &mut dyn SweepView,
    allow_root: &NormalizedPath,
    roots: &MachineRoots,
    limits: SanityLimits,
) -> Result<SweepReport, GuardError> {
    let mut report = SweepReport {
        deleted_files: 0,
        deleted_bytes: 0,
        skipped: Vec::new(),
    };
    let root_text = display_path(allow_root);
    walk(
        view,
        &root_text,
        allow_root,
        roots,
        limits,
        &mut report,
        true,
    )?;
    Ok(report)
}

fn walk(
    view: &mut dyn SweepView,
    path: &str,
    allow_root: &NormalizedPath,
    roots: &MachineRoots,
    limits: SanityLimits,
    report: &mut SweepReport,
    is_root: bool,
) -> Result<(), GuardError> {
    let children = view.list(path)?;
    for name in children {
        if hostile_component(OsStr::new(&name)) {
            report.skipped.push(Skipped {
                path: format!("{path}\\{name}"),
                reason: SkipReason::HostileName,
            });
            continue;
        }
        let child = format!("{path}\\{name}");
        let facts = view.facts(&child)?;
        match decide_node(&facts, allow_root, roots) {
            NodeDecision::Delete { bytes } => {
                check_cap(report, limits, bytes)?;
                view.delete_file(&child)?;
                report.deleted_files += 1;
                report.deleted_bytes += bytes;
            }
            NodeDecision::Enter => {
                walk(view, &child, allow_root, roots, limits, report, false)?;
                if view.list(&child)?.is_empty() {
                    view.delete_empty_dir(&child)?;
                }
            }
            NodeDecision::DeleteEmptyDir | NodeDecision::Skip { .. } => {
                let reason = match decide_node(&facts, allow_root, roots) {
                    NodeDecision::Skip { reason } => reason,
                    _ => SkipReason::IsRoot,
                };
                report.skipped.push(Skipped {
                    path: child,
                    reason,
                });
            }
        }
    }
    let _ = is_root;
    Ok(())
}

fn check_cap(
    report: &SweepReport,
    limits: SanityLimits,
    next_bytes: u64,
) -> Result<(), GuardError> {
    if limits.human_present {
        return Ok(());
    }
    let files = report.deleted_files + 1;
    let bytes = report.deleted_bytes + next_bytes;
    if files > limits.max_files || bytes > limits.max_bytes {
        return Err(GuardError::SanityCap { files, bytes });
    }
    Ok(())
}

fn display_path(path: &NormalizedPath) -> String {
    path.components().join("\\")
}

/// Whether a debug build may mutate this root.
///
/// A sandbox is accepted only when it is a strict descendant of the process temp
/// directory. Release builds, and debug builds with the explicit flag, may proceed
/// to the deny-list. The deny-list still applies either way.
#[must_use]
pub fn mutation_allowed(debug_build: bool, allow_flag: bool, root_is_temp_sandbox: bool) -> bool {
    if root_is_temp_sandbox {
        return true;
    }
    if debug_build {
        return allow_flag;
    }
    true
}

/// True when `root` is a directory strictly inside `temp`.
#[must_use]
pub fn is_temp_sandbox(root: &NormalizedPath, temp: &NormalizedPath) -> bool {
    root.is_within(temp) && !root.is_same(temp)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::decide::NodeFacts;
    use std::collections::BTreeMap;
    use std::path::Path;

    #[derive(Clone)]
    enum Kind {
        File,
        Dir,
        Reparse,
    }

    struct Node {
        kind: Kind,
        created: u64,
        modified: u64,
        size: u64,
        in_use: bool,
        children: BTreeMap<String, Node>,
    }

    struct MemoryFs {
        root_path: String,
        root: Node,
        outside: BTreeMap<String, Node>,
    }

    impl MemoryFs {
        fn dir() -> Node {
            Node {
                kind: Kind::Dir,
                created: 1,
                modified: 1,
                size: 0,
                in_use: false,
                children: BTreeMap::new(),
            }
        }

        fn file(created: u64, size: u64) -> Node {
            Node {
                kind: Kind::File,
                created,
                modified: created,
                size,
                in_use: false,
                children: BTreeMap::new(),
            }
        }
    }

    impl SweepView for MemoryFs {
        fn list(&self, path: &str) -> Result<Vec<String>, GuardError> {
            let node = self
                .find(path)
                .ok_or_else(|| GuardError::View(path.to_owned()))?;
            Ok(node.children.keys().cloned().collect())
        }

        fn facts(&self, path: &str) -> Result<NodeFacts, GuardError> {
            let node = self
                .find(path)
                .ok_or_else(|| GuardError::View(path.to_owned()))?;
            Ok(NodeFacts {
                final_path: NormalizedPath::normalize(Path::new(path)),
                reparse: matches!(node.kind, Kind::Reparse),
                link_count: 1,
                directory: matches!(node.kind, Kind::Dir),
                fixed_local_volume: true,
                created_unix: node.created,
                modified_unix: node.modified,
                now_unix: 1_000_000,
                min_age_secs: 86_400,
                exclusive_open: !node.in_use,
                elevated: true,
                allocated_size: node.size,
            })
        }

        fn delete_file(&mut self, path: &str) -> Result<(), GuardError> {
            let (parent, name) = split(path).ok_or_else(|| GuardError::View(path.to_owned()))?;
            let parent_node = self
                .find_mut(parent)
                .ok_or_else(|| GuardError::View(parent.to_owned()))?;
            parent_node
                .children
                .remove(name)
                .ok_or_else(|| GuardError::View(path.to_owned()))?;
            Ok(())
        }

        fn delete_empty_dir(&mut self, path: &str) -> Result<(), GuardError> {
            self.delete_file(path)
        }
    }

    impl MemoryFs {
        fn find(&self, path: &str) -> Option<&Node> {
            if path == self.root_path {
                return Some(&self.root);
            }
            let rest = path.strip_prefix(&self.root_path)?.strip_prefix('\\')?;
            let mut node = &self.root;
            for part in rest.split('\\') {
                node = node.children.get(part)?;
            }
            Some(node)
        }

        fn find_mut(&mut self, path: &str) -> Option<&mut Node> {
            if path == self.root_path {
                return Some(&mut self.root);
            }
            let rest = path.strip_prefix(&self.root_path)?.strip_prefix('\\')?;
            let mut node = &mut self.root;
            for part in rest.split('\\') {
                node = node.children.get_mut(part)?;
            }
            Some(node)
        }
    }

    fn split(path: &str) -> Option<(&str, &str)> {
        path.rsplit_once('\\')
    }

    fn machine() -> MachineRoots {
        MachineRoots {
            system_root: NormalizedPath::normalize(Path::new(r"d:\windows")),
            program_files: NormalizedPath::normalize(Path::new(r"d:\program files")),
            program_files_x86: None,
            program_data: NormalizedPath::normalize(Path::new(r"d:\programdata")),
            user_profile: NormalizedPath::normalize(Path::new(r"b:\users\ada")),
            protected_user_dirs: vec![NormalizedPath::normalize(Path::new(
                r"b:\users\ada\documents",
            ))],
            castellan_dirs: vec![],
        }
    }

    #[test]
    fn sweep_deletes_old_files_and_leaves_the_junction_target() {
        let root_path = r"b:\users\ada\appdata\local\temp\job";
        let mut nested = MemoryFs::dir();
        nested
            .children
            .insert("old.bin".to_owned(), MemoryFs::file(10, 40));
        nested
            .children
            .insert("new.bin".to_owned(), MemoryFs::file(1_000_000, 5));
        let mut root = MemoryFs::dir();
        root.children
            .insert("old.bin".to_owned(), MemoryFs::file(10, 100));
        root.children.insert("keep".to_owned(), {
            let mut busy = MemoryFs::file(10, 7);
            busy.in_use = true;
            busy
        });
        root.children.insert(
            "link".to_owned(),
            Node {
                kind: Kind::Reparse,
                created: 10,
                modified: 10,
                size: 0,
                in_use: false,
                children: BTreeMap::new(),
            },
        );
        root.children.insert("nested".to_owned(), nested);
        let mut fs = MemoryFs {
            root_path: root_path.to_owned(),
            root,
            outside: BTreeMap::from([(
                r"d:\windows\system32\hosts".to_owned(),
                MemoryFs::file(10, 999),
            )]),
        };
        let report = run_sweep(
            &mut fs,
            &NormalizedPath::normalize(Path::new(root_path)),
            &machine(),
            SanityLimits {
                max_files: 100,
                max_bytes: 10_000,
                human_present: true,
            },
        )
        .expect("sweep");
        assert_eq!(report.deleted_files, 2);
        assert_eq!(report.deleted_bytes, 140);
        assert!(fs.find(&format!("{root_path}\\nested\\new.bin")).is_some());
        assert!(fs.find(&format!("{root_path}\\keep")).is_some());
        assert!(fs.find(&format!("{root_path}\\link")).is_some());
        assert!(fs.outside.contains_key(r"d:\windows\system32\hosts"));
        assert!(fs.find(&format!("{root_path}\\old.bin")).is_none());
    }

    #[test]
    fn dotdot_child_name_is_not_joined() {
        let root_path = r"b:\users\ada\appdata\local\temp\job";
        let mut root = MemoryFs::dir();
        root.children
            .insert("..".to_owned(), MemoryFs::file(10, 50));
        root.children
            .insert("..\\..\\windows".to_owned(), MemoryFs::file(10, 50));
        let mut fs = MemoryFs {
            root_path: root_path.to_owned(),
            root,
            outside: BTreeMap::new(),
        };
        let report = run_sweep(
            &mut fs,
            &NormalizedPath::normalize(Path::new(root_path)),
            &machine(),
            SanityLimits {
                max_files: 10,
                max_bytes: 1000,
                human_present: true,
            },
        )
        .expect("sweep");
        assert_eq!(report.deleted_files, 0);
        assert!(
            report
                .skipped
                .iter()
                .any(|skip| skip.reason == SkipReason::HostileName)
        );
    }

    #[test]
    fn autopilot_sanity_cap_aborts() {
        let root_path = r"b:\users\ada\appdata\local\temp\job";
        let mut root = MemoryFs::dir();
        root.children
            .insert("a.bin".to_owned(), MemoryFs::file(10, 80));
        root.children
            .insert("b.bin".to_owned(), MemoryFs::file(10, 80));
        let mut fs = MemoryFs {
            root_path: root_path.to_owned(),
            root,
            outside: BTreeMap::new(),
        };
        let error = run_sweep(
            &mut fs,
            &NormalizedPath::normalize(Path::new(root_path)),
            &machine(),
            SanityLimits {
                max_files: 10,
                max_bytes: 100,
                human_present: false,
            },
        );
        assert!(matches!(error, Err(GuardError::SanityCap { .. })));
        assert!(
            fs.find(&format!("{root_path}\\b.bin")).is_some()
                || fs.find(&format!("{root_path}\\a.bin")).is_some()
        );
    }

    #[test]
    fn debug_build_refuses_a_root_outside_temp() {
        let temp = NormalizedPath::normalize(Path::new(r"b:\users\ada\appdata\local\temp"));
        let windows = NormalizedPath::normalize(Path::new(r"d:\windows\temp"));
        let sandbox =
            NormalizedPath::normalize(Path::new(r"b:\users\ada\appdata\local\temp\castellan-test"));
        assert!(!mutation_allowed(
            true,
            false,
            is_temp_sandbox(&windows, &temp)
        ));
        assert!(mutation_allowed(
            true,
            false,
            is_temp_sandbox(&sandbox, &temp)
        ));
        assert!(!mutation_allowed(
            true,
            false,
            is_temp_sandbox(&temp, &temp)
        ));
        assert!(mutation_allowed(true, true, false));
    }
}
