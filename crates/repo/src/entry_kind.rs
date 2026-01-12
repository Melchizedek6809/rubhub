/// The kind of an entry in a git tree
///
/// This enum mirrors gix's `EntryKind` but is owned by this crate
/// to avoid leaking the gix dependency to consumers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EntryKind {
    /// A directory/tree
    Tree,
    /// A regular file (blob)
    Blob,
    /// An executable file
    BlobExecutable,
    /// A symbolic link
    Link,
    /// A git submodule (commit reference)
    Commit,
}

impl EntryKind {
    /// Returns true if this entry is a directory
    pub fn is_tree(&self) -> bool {
        matches!(self, EntryKind::Tree)
    }

    /// Returns true if this entry is a file (blob or executable)
    pub fn is_blob(&self) -> bool {
        matches!(self, EntryKind::Blob | EntryKind::BlobExecutable)
    }

    /// Convert from gix's EntryKind
    pub(crate) fn from_gix(kind: gix::objs::tree::EntryKind) -> Self {
        use gix::objs::tree::EntryKind as GixKind;
        match kind {
            GixKind::Tree => EntryKind::Tree,
            GixKind::Blob => EntryKind::Blob,
            GixKind::BlobExecutable => EntryKind::BlobExecutable,
            GixKind::Link => EntryKind::Link,
            GixKind::Commit => EntryKind::Commit,
        }
    }
}
