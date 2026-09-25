//! Remote file and path types.
//!
//! [`RemoteFile`] is the directory-entry record surfaced to the UI. It mirrors
//! `RemoteFile` in `apps/desktop/src/lib/types.ts` exactly.

use std::fmt;

use serde::{Deserialize, Serialize};

/// Distinguishes a directory entry kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FileKind {
    File,
    Directory,
    Symlink,
}

/// A POSIX-style remote path. Stored as a normalized string so it works across
/// FTP and SFTP without leaking `PathBuf`'s platform semantics.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct FilePath(String);

impl FilePath {
    /// Root path.
    pub const ROOT: &'static str = "/";

    pub fn new(path: impl Into<String>) -> Self {
        Self(path.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Whether this is the filesystem root (`/`).
    pub fn is_root(&self) -> bool {
        self.0 == Self::ROOT
    }

    /// Append a child segment, returning a new path. Normalizes repeated
    /// slashes.
    pub fn join(&self, child: &str) -> Self {
        if self.is_root() {
            return Self(format!("/{}", child.trim_start_matches('/')));
        }
        Self(format!("{}/{}", self.0.trim_end_matches('/'), child.trim_matches('/')))
    }

    /// The parent path, or `/` if already at root.
    pub fn parent(&self) -> Self {
        if self.is_root() {
            return Self::new(Self::ROOT);
        }
        match self.0.rsplit_once('/') {
            Some(("", _)) => Self::new(Self::ROOT),
            Some((head, _)) => Self::new(head),
            None => Self::new(Self::ROOT),
        }
    }

    /// The final path segment (file/directory name), or `/` at root.
    pub fn name(&self) -> &str {
        if self.is_root() {
            return Self::ROOT;
        }
        match self.0.rsplit_once('/') {
            // Trailing slash: segment is the part before it.
            Some((head, "")) if !head.is_empty() => {
                head.rsplit_once('/').map(|(_, name)| name).unwrap_or(head)
            }
            Some((_, name)) => name,
            None => &self.0,
        }
    }
}

impl fmt::Display for FilePath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// A directory entry on a remote filesystem.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RemoteFile {
    pub name: String,
    pub kind: FileKind,
    /// Bytes; `0` for directories.
    pub size: u64,
    /// Modified time, epoch milliseconds. `0` means unknown.
    pub modified: i64,
    /// Unix permission string, e.g. `rwxr-xr-x`, when available.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub permissions: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub owner: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub group: Option<String>,
}

/// Format the low 9 permission bits as the classic `rwxr-xr-x` string — the
/// canonical display form for [`RemoteFile::permissions`].
pub fn format_permissions(bits: u32) -> String {
    let mut out = String::with_capacity(9);
    for shift in [6, 3, 0] {
        let triad = (bits >> shift) & 0o7;
        out.push(if triad & 0o4 != 0 { 'r' } else { '-' });
        out.push(if triad & 0o2 != 0 { 'w' } else { '-' });
        out.push(if triad & 0o1 != 0 { 'x' } else { '-' });
    }
    out
}

impl RemoteFile {
    /// Synthesize the special `..` parent entry used in directory listings.
    pub fn parent_entry() -> Self {
        Self {
            name: "..".into(),
            kind: FileKind::Directory,
            size: 0,
            modified: 0,
            permissions: None,
            owner: None,
            group: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn path_join_at_root() {
        let root = FilePath::new("/");
        assert_eq!(root.join("home").as_str(), "/home");
        assert_eq!(root.join("/home").as_str(), "/home");
    }

    #[test]
    fn path_join_deeper() {
        let p = FilePath::new("/home");
        assert_eq!(p.join("tom").as_str(), "/home/tom");
        assert_eq!(p.join("tom/").as_str(), "/home/tom");
        assert_eq!(p.join("/docs/").as_str(), "/home/docs");
    }

    #[test]
    fn path_parent() {
        assert_eq!(FilePath::new("/").parent().as_str(), "/");
        assert_eq!(FilePath::new("/home").parent().as_str(), "/");
        assert_eq!(FilePath::new("/home/tom").parent().as_str(), "/home");
        assert_eq!(FilePath::new("/a/b/c").parent().as_str(), "/a/b");
    }

    #[test]
    fn path_name() {
        assert_eq!(FilePath::new("/home/tom").name(), "tom");
        assert_eq!(FilePath::new("/").name(), "/");
    }

    #[test]
    fn parent_entry_shape() {
        let e = RemoteFile::parent_entry();
        assert_eq!(e.name, "..");
        assert_eq!(e.kind, FileKind::Directory);
    }

    #[test]
    fn file_serde_round_trip() {
        let f = RemoteFile {
            name: "deploy.sh".into(),
            kind: FileKind::File,
            size: 12_400,
            modified: 1_700_000_000_000,
            permissions: Some("rwxr-xr-x".into()),
            owner: Some("tom".into()),
            group: None,
        };
        let json = serde_json::to_string(&f).unwrap();
        let back: RemoteFile = serde_json::from_str(&json).unwrap();
        assert_eq!(f, back);
    }

    #[test]
    fn skips_none_optionals_in_json() {
        let f = RemoteFile {
            name: "a".into(),
            kind: FileKind::File,
            size: 0,
            modified: 0,
            permissions: None,
            owner: None,
            group: None,
        };
        let json = serde_json::to_string(&f).unwrap();
        assert!(!json.contains("permissions"));
        assert!(!json.contains("owner"));
    }

    #[test]
    fn permission_formatting() {
        assert_eq!(format_permissions(0o755), "rwxr-xr-x");
        assert_eq!(format_permissions(0o644), "rw-r--r--");
        assert_eq!(format_permissions(0o000), "---------");
        assert_eq!(format_permissions(0o777), "rwxrwxrwx");
        // High bits (setuid etc.) are masked off.
        assert_eq!(format_permissions(0o104755), "rwxr-xr-x");
    }
}
