use std::fmt;
use std::io;

/// Error type for repository operations
#[derive(Debug)]
pub enum RepoError {
    /// Invalid path component (user, project, branch, etc.)
    InvalidPath(String),
    /// Repository or ref not found
    NotFound,
    /// Git operation failed
    Git(String),
    /// I/O error
    Io(io::Error),
    /// Task join error
    TaskJoin(String),
}

impl fmt::Display for RepoError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RepoError::InvalidPath(msg) => write!(f, "invalid path: {}", msg),
            RepoError::NotFound => write!(f, "not found"),
            RepoError::Git(msg) => write!(f, "git error: {}", msg),
            RepoError::Io(e) => write!(f, "I/O error: {}", e),
            RepoError::TaskJoin(msg) => write!(f, "task join error: {}", msg),
        }
    }
}

impl std::error::Error for RepoError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            RepoError::Io(e) => Some(e),
            _ => None,
        }
    }
}

impl From<io::Error> for RepoError {
    fn from(e: io::Error) -> Self {
        RepoError::Io(e)
    }
}

impl From<anyhow::Error> for RepoError {
    fn from(e: anyhow::Error) -> Self {
        RepoError::Git(e.to_string())
    }
}
