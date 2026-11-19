//! Lexical path normalization and the compiled deny-list.
//!
//! The final path still has to come from a handle. Normalization is what makes the
//! comparison component-wise and case-insensitive, and what rejects `..` escapes.

use serde::{Deserialize, Serialize};

/// A path reduced to lowercase components. UNC paths are marked and always denied.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NormalizedPath {
    components: Vec<String>,
    unc: bool,
}

impl NormalizedPath {
    /// Normalize a Windows path. `..` is resolved. A leading `\\?\` is stripped.
    #[must_use]
    pub fn normalize(path: &std::path::Path) -> Self {
        let raw = path.to_string_lossy();
        let mut text = raw.as_ref();
        let mut unc = false;
        if let Some(rest) = text.strip_prefix(r"\\?\UNC\") {
            unc = true;
            text = rest;
        } else if let Some(rest) = text.strip_prefix(r"\\?\") {
            text = rest;
        } else if text.starts_with(r"\\") {
            unc = true;
            text = text.trim_start_matches('\\');
        }
        let mut components = Vec::new();
        for part in text.split(['\\', '/']) {
            if part.is_empty() || part == "." {
                continue;
            }
            if part == ".." {
                components.pop();
                continue;
            }
            components.push(part.to_lowercase());
        }
        Self { components, unc }
    }

    /// True when this path is a UNC path.
    #[must_use]
    pub const fn is_unc(&self) -> bool {
        self.unc
    }

    /// Components, already lowercased.
    #[must_use]
    pub fn components(&self) -> &[String] {
        &self.components
    }

    /// True when `self` is `root` or a descendant, compared by whole components.
    #[must_use]
    pub fn is_within(&self, root: &Self) -> bool {
        if self.unc || root.unc {
            return false;
        }
        self.components.starts_with(&root.components) && !root.components.is_empty()
    }

    /// True when the path is exactly `root`.
    #[must_use]
    pub fn is_same(&self, root: &Self) -> bool {
        !self.unc && !root.unc && self.components == root.components
    }

    /// A volume root such as `C:\`, which has a single `c:` component.
    #[must_use]
    pub fn is_volume_root(&self) -> bool {
        if self.unc || self.components.len() != 1 {
            return false;
        }
        match self.components.first() {
            Some(first) => is_drive(first),
            None => false,
        }
    }
}

fn is_drive(component: &str) -> bool {
    let mut chars = component.chars();
    matches!(
        (chars.next(), chars.next(), chars.next()),
        (Some(letter), Some(':'), None) if letter.is_ascii_alphabetic()
    )
}

/// Locations the deny-list always protects. They come from Win32, never from a literal `C:\`.
#[derive(Clone, Debug)]
pub struct MachineRoots {
    /// `%SystemRoot%`.
    pub system_root: NormalizedPath,
    /// Program Files.
    pub program_files: NormalizedPath,
    /// Program Files (x86), when the OS has one.
    pub program_files_x86: Option<NormalizedPath>,
    /// ProgramData.
    pub program_data: NormalizedPath,
    /// The user profile directory itself.
    pub user_profile: NormalizedPath,
    /// Documents, Desktop, Pictures, Videos, Music, Downloads, OneDrive.
    pub protected_user_dirs: Vec<NormalizedPath>,
    /// Castellan's own directories.
    pub castellan_dirs: Vec<NormalizedPath>,
}

/// Why the compiled deny-list rejected a path.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DenyHit {
    /// UNC or other network path.
    Unc,
    /// The path is a volume root.
    VolumeRoot,
    /// The user profile directory itself.
    UserProfile,
    /// Documents, Desktop, or another protected user folder.
    ProtectedUserDir,
    /// Program Files.
    ProgramFiles,
    /// ProgramData, outside an explicit allow-list entry (none are offered).
    ProgramData,
    /// Castellan's install, policy, or state directory.
    CastellanDir,
    /// Inside Windows, and not an explicitly allowed subtarget such as `Temp`.
    SystemRoot,
    /// `System Volume Information` or `$Recycle.Bin`.
    Reserved,
}

/// Return the deny reason, if any. The deny-list is not configurable.
#[must_use]
pub fn deny_list(path: &NormalizedPath, roots: &MachineRoots) -> Option<DenyHit> {
    if path.is_unc() {
        return Some(DenyHit::Unc);
    }
    if path.is_volume_root() {
        return Some(DenyHit::VolumeRoot);
    }
    if contains_reserved(path) {
        return Some(DenyHit::Reserved);
    }
    if path.is_same(&roots.user_profile) {
        return Some(DenyHit::UserProfile);
    }
    if roots
        .protected_user_dirs
        .iter()
        .any(|dir| path.is_within(dir))
    {
        return Some(DenyHit::ProtectedUserDir);
    }
    if path.is_within(&roots.program_files)
        || roots
            .program_files_x86
            .as_ref()
            .is_some_and(|dir| path.is_within(dir))
    {
        return Some(DenyHit::ProgramFiles);
    }
    if path.is_within(&roots.program_data) {
        return Some(DenyHit::ProgramData);
    }
    if roots.castellan_dirs.iter().any(|dir| path.is_within(dir)) {
        return Some(DenyHit::CastellanDir);
    }
    if path.is_within(&roots.system_root) && !is_allowed_system_subpath(path, &roots.system_root) {
        return Some(DenyHit::SystemRoot);
    }
    None
}

fn contains_reserved(path: &NormalizedPath) -> bool {
    path.components()
        .iter()
        .any(|component| component == "system volume information" || component == "$recycle.bin")
}

/// Only `%SystemRoot%\Temp` is an allowed deletion root under Windows.
fn is_allowed_system_subpath(path: &NormalizedPath, system_root: &NormalizedPath) -> bool {
    let mut temp = system_root.clone();
    temp.components.push("temp".to_owned());
    path.is_within(&temp)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    fn roots() -> MachineRoots {
        MachineRoots {
            system_root: NormalizedPath::normalize(Path::new(r"D:\Windows")),
            program_files: NormalizedPath::normalize(Path::new(r"D:\Program Files")),
            program_files_x86: Some(NormalizedPath::normalize(Path::new(
                r"D:\Program Files (x86)",
            ))),
            program_data: NormalizedPath::normalize(Path::new(r"D:\ProgramData")),
            user_profile: NormalizedPath::normalize(Path::new(r"B:\Users\denner")),
            protected_user_dirs: vec![NormalizedPath::normalize(Path::new(
                r"B:\Users\denner\Documents",
            ))],
            castellan_dirs: vec![NormalizedPath::normalize(Path::new(
                r"D:\Program Files\Castellan",
            ))],
        }
    }

    #[test]
    fn dotdot_cannot_escape_the_allow_root() {
        let root = NormalizedPath::normalize(Path::new(r"B:\Users\denner\AppData\Local\Temp"));
        let escaped = NormalizedPath::normalize(Path::new(
            r"B:\Users\denner\AppData\Local\Temp\..\..\..\Documents\secret.txt",
        ));
        assert!(!escaped.is_within(&root));
        assert!(deny_list(&escaped, &roots()).is_some());
    }

    #[test]
    fn component_prefix_is_not_a_string_prefix() {
        let root = NormalizedPath::normalize(Path::new(r"D:\Temp"));
        let sibling = NormalizedPath::normalize(Path::new(r"D:\Tempest\file.txt"));
        assert!(!sibling.is_within(&root));
    }

    #[test]
    fn windows_temp_is_allowed_and_system32_is_not() {
        let machine = roots();
        let temp = NormalizedPath::normalize(Path::new(r"D:\Windows\Temp\old.tmp"));
        let system32 = NormalizedPath::normalize(Path::new(r"D:\Windows\System32\kernel32.dll"));
        let prefetch =
            NormalizedPath::normalize(Path::new(r"D:\Windows\Prefetch\NOTEPAD.EXE-ABC.pf"));
        assert_eq!(deny_list(&temp, &machine), None);
        assert_eq!(deny_list(&system32, &machine), Some(DenyHit::SystemRoot));
        assert_eq!(deny_list(&prefetch, &machine), Some(DenyHit::SystemRoot));
    }

    #[test]
    fn unc_and_volume_roots_are_denied() {
        let machine = roots();
        let unc = NormalizedPath::normalize(Path::new(r"\\server\share\temp\a.txt"));
        let root = NormalizedPath::normalize(Path::new(r"D:\"));
        assert_eq!(deny_list(&unc, &machine), Some(DenyHit::Unc));
        assert_eq!(deny_list(&root, &machine), Some(DenyHit::VolumeRoot));
    }
}
