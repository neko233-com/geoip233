use std::fmt;

/// Errors that can occur when using the GeoIP database.
#[derive(Debug)]
pub enum GeoIpError {
    /// The specified file was not found.
    FileNotFound(String),

    /// The database file is not valid MMDB format.
    InvalidDatabase(String),

    /// An I/O error occurred.
    Io(String),

    /// Memory mapping failed.
    #[cfg(feature = "mmap")]
    MmapFailed(String),

    /// The internal lock was poisoned (a thread panicked while holding the lock).
    /// Only occurs when `hot-reload` feature is disabled.
    #[cfg(not(feature = "hot-reload"))]
    LockPoisoned(String),
}

impl fmt::Display for GeoIpError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::FileNotFound(path) => write!(f, "database file not found: {}", path),
            Self::InvalidDatabase(msg) => write!(f, "invalid database: {}", msg),
            Self::Io(msg) => write!(f, "I/O error: {}", msg),
            #[cfg(feature = "mmap")]
            Self::MmapFailed(msg) => write!(f, "mmap failed: {}", msg),
            #[cfg(not(feature = "hot-reload"))]
            Self::LockPoisoned(msg) => write!(f, "lock poisoned: {}", msg),
        }
    }
}

impl std::error::Error for GeoIpError {}
