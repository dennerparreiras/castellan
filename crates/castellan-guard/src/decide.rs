//! Per-node delete decision. Pure: the caller supplies facts gathered from a handle.

use crate::path::{DenyHit, MachineRoots, NormalizedPath, deny_list};

/// Facts about one filesystem node, gathered without following a reparse point.
#[derive(Clone, Debug)]
pub struct NodeFacts {
    /// Final path of the handle.
    pub final_path: NormalizedPath,
    /// The node is a junction, symlink, or other reparse point.
    pub reparse: bool,
    /// Hard-link count from `FILE_STANDARD_INFO.NumberOfLinks`.
    ///
    /// Elevated deletes skip counts above 1. A caller that has not read the
    /// count yet must pass `u32::MAX`, which fails closed.
    pub link_count: u32,
    /// Directory, not a file.
    pub directory: bool,
    /// The volume is a fixed local disk.
    pub fixed_local_volume: bool,
    /// Creation time, seconds since the Unix epoch.
    pub created_unix: u64,
    /// Modification time, seconds since the Unix epoch.
    pub modified_unix: u64,
    /// Clock the age check uses.
    pub now_unix: u64,
    /// Minimum age, already resolved through policy.
    pub min_age_secs: u64,
    /// An exclusive open succeeded, so nothing else is using the file.
    pub exclusive_open: bool,
    /// The process is the elevated broker.
    pub elevated: bool,
    /// Allocated size, used for the report. Not an estimate from the name.
    pub allocated_size: u64,
}

/// What the sweeper should do with one node.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NodeDecision {
    /// Delete the file, or the reparse point itself in a non-elevated user sweep.
    Delete {
        /// Bytes that will be charged as freed.
        bytes: u64,
    },
    /// Walk children. Never returned for a reparse point.
    Enter,
    /// Remove this directory because it is empty and is not the allow root.
    DeleteEmptyDir,
    /// Leave it alone.
    Skip {
        /// Why.
        reason: SkipReason,
    },
}

/// Why a node was left in place.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SkipReason {
    /// Not a fixed local volume.
    NotFixedVolume,
    /// The final path is outside the allow root.
    OutsideRoot,
    /// The compiled deny-list matched.
    Denied(DenyHit),
    /// The allow root itself is never deleted.
    IsRoot,
    /// Reparse point while elevated. The link is left in place.
    ReparseElevated,
    /// Hard link while elevated.
    HardLink,
    /// Another process has the file open.
    InUse,
    /// Creation or modification is newer than the minimum age.
    TooNew,
    /// The name contains a separator or a colon, so it is not joined.
    HostileName,
}

/// Decide one node. `allow_root` is the final path of the sweep target.
#[must_use]
pub fn decide_node(
    facts: &NodeFacts,
    allow_root: &NormalizedPath,
    roots: &MachineRoots,
) -> NodeDecision {
    if !facts.fixed_local_volume {
        return skip(SkipReason::NotFixedVolume);
    }
    if !facts.final_path.is_within(allow_root) {
        return skip(SkipReason::OutsideRoot);
    }
    if let Some(hit) = deny_list(&facts.final_path, roots) {
        return skip(SkipReason::Denied(hit));
    }
    if facts.final_path.is_same(allow_root) {
        return skip(SkipReason::IsRoot);
    }
    if facts.reparse {
        if facts.elevated {
            return skip(SkipReason::ReparseElevated);
        }
        // User context may remove the link itself. Children are never visited:
        // the walker only enters when this function returns `Enter`.
        return NodeDecision::Delete { bytes: 0 };
    }
    if facts.elevated && facts.link_count > 1 {
        return skip(SkipReason::HardLink);
    }
    if facts.directory {
        return NodeDecision::Enter;
    }
    if !facts.exclusive_open {
        return skip(SkipReason::InUse);
    }
    if !old_enough(facts.now_unix, facts.created_unix, facts.min_age_secs)
        || !old_enough(facts.now_unix, facts.modified_unix, facts.min_age_secs)
    {
        return skip(SkipReason::TooNew);
    }
    NodeDecision::Delete {
        bytes: facts.allocated_size,
    }
}

/// A directory name that must not be joined onto the parent.
#[must_use]
pub fn hostile_component(name: &std::ffi::OsStr) -> bool {
    let text = name.to_string_lossy();
    text.is_empty() || text.contains(['\\', '/', ':', '\u{0}']) || text == "." || text == ".."
}

fn old_enough(now: u64, stamp: u64, min_age: u64) -> bool {
    now.saturating_sub(stamp) >= min_age
}

fn skip(reason: SkipReason) -> NodeDecision {
    NodeDecision::Skip { reason }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    fn machine() -> MachineRoots {
        MachineRoots {
            system_root: NormalizedPath::normalize(Path::new(r"D:\Windows")),
            program_files: NormalizedPath::normalize(Path::new(r"D:\Program Files")),
            program_files_x86: None,
            program_data: NormalizedPath::normalize(Path::new(r"D:\ProgramData")),
            user_profile: NormalizedPath::normalize(Path::new(r"B:\Users\ada")),
            protected_user_dirs: vec![NormalizedPath::normalize(Path::new(
                r"B:\Users\ada\Documents",
            ))],
            castellan_dirs: vec![],
        }
    }

    fn facts(path: &str) -> NodeFacts {
        NodeFacts {
            final_path: NormalizedPath::normalize(Path::new(path)),
            reparse: false,
            link_count: 1,
            directory: false,
            fixed_local_volume: true,
            created_unix: 1_000,
            modified_unix: 1_000,
            now_unix: 1_000 + 86_400 * 3,
            min_age_secs: 86_400,
            exclusive_open: true,
            elevated: true,
            allocated_size: 10,
        }
    }

    #[test]
    fn junction_final_path_outside_the_root_is_not_deleted() {
        let root = NormalizedPath::normalize(Path::new(r"B:\Users\ada\AppData\Local\Temp\job"));
        let mut outside = facts(r"D:\Windows\System32\drivers\etc\hosts");
        outside.reparse = false;
        let decision = decide_node(&outside, &root, &machine());
        assert!(matches!(
            decision,
            NodeDecision::Skip {
                reason: SkipReason::OutsideRoot
            }
        ));
    }

    #[test]
    fn elevated_reparse_inside_the_root_is_skipped() {
        let root = NormalizedPath::normalize(Path::new(r"B:\Users\ada\AppData\Local\Temp\job"));
        let mut link = facts(r"B:\Users\ada\AppData\Local\Temp\job\link");
        link.reparse = true;
        let decision = decide_node(&link, &root, &machine());
        assert!(matches!(
            decision,
            NodeDecision::Skip {
                reason: SkipReason::ReparseElevated
            }
        ));
    }

    #[test]
    fn a_new_file_and_an_in_use_file_are_kept() {
        let root = NormalizedPath::normalize(Path::new(r"B:\Users\ada\AppData\Local\Temp\job"));
        let mut young = facts(r"B:\Users\ada\AppData\Local\Temp\job\new.bin");
        young.created_unix = young.now_unix;
        assert!(matches!(
            decide_node(&young, &root, &machine()),
            NodeDecision::Skip {
                reason: SkipReason::TooNew
            }
        ));
        let mut busy = facts(r"B:\Users\ada\AppData\Local\Temp\job\busy.bin");
        busy.exclusive_open = false;
        assert!(matches!(
            decide_node(&busy, &root, &machine()),
            NodeDecision::Skip {
                reason: SkipReason::InUse
            }
        ));
    }

    #[test]
    fn an_old_file_inside_the_root_is_deleted() {
        let root = NormalizedPath::normalize(Path::new(r"B:\Users\ada\AppData\Local\Temp\job"));
        let decision = decide_node(
            &facts(r"B:\Users\ada\AppData\Local\Temp\job\old.bin"),
            &root,
            &machine(),
        );
        assert_eq!(decision, NodeDecision::Delete { bytes: 10 });
    }
}
