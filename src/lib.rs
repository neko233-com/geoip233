//! # geoip233
//!
//! Offline GeoIP library for Rust, powered by MaxMind DB (.mmdb) format.
//!
//! ## Feature Flags
//!
//! | Feature | Default | Description |
//! |---------|---------|-------------|
//! | `builtin` | ✓ | Embed GeoLite2-City database in binary |
//! | `mmap` | ✓ | Memory-mapped file I/O for custom databases |
//! | `hot-reload` | ✗ | Enable `update()` for runtime database replacement |
//!
//! ## Quick Start
//!
//! ```no_run
//! use geoip233::GeoIp;
//!
//! let geo = GeoIp::default();
//! let city = geo.lookup_str("8.8.8.8").unwrap();
//! println!("Country: {}", city.country_str());
//! ```

mod error;
mod types;

pub use error::GeoIpError;
pub use types::*;

use maxminddb::Reader;
use std::net::IpAddr;
use std::path::Path;
use std::sync::Arc;

/// A thread-safe GeoIP database reader with optional hot-reload support.
pub struct GeoIp {
    reader: GeoIpInner,
}

/// Internal: hot-reload uses ArcSwap for lock-free atomic swaps.
#[cfg(feature = "hot-reload")]
type GeoIpInner = arc_swap::ArcSwap<Reader<Vec<u8>>>;

/// Internal: standard mode uses Arc for shared ownership.
#[cfg(not(feature = "hot-reload"))]
type GeoIpInner = Arc<std::sync::RwLock<Reader<Vec<u8>>>>;

impl GeoIp {
    /// Open a MaxMind `.mmdb` database from a file path.
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self, GeoIpError> {
        let path = path.as_ref();
        let reader = Reader::open_readfile(path).map_err(|e| {
            let msg = e.to_string();
            if msg.contains("No such file") || msg.contains("not found") || msg.contains("os error") {
                GeoIpError::FileNotFound(path.display().to_string())
            } else {
                GeoIpError::InvalidDatabase(msg)
            }
        })?;
        Ok(Self::new_inner(reader))
    }

    /// Create a reader from raw database bytes.
    pub fn from_bytes(data: Vec<u8>) -> Result<Self, GeoIpError> {
        let reader =
            Reader::from_source(data).map_err(|e| GeoIpError::InvalidDatabase(e.to_string()))?;
        Ok(Self::new_inner(reader))
    }

    /// Create a reader from a memory-mapped file for maximum I/O performance.
    ///
    /// Note: The mmap is read into memory immediately. For true mmap performance,
    /// use `GeoIp::open()` which uses `open_readfile` internally.
    #[cfg(feature = "mmap")]
    pub fn open_mmap<P: AsRef<Path>>(path: P) -> Result<Self, GeoIpError> {
        let path = path.as_ref();
        let data = std::fs::read(path).map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                GeoIpError::FileNotFound(path.display().to_string())
            } else {
                GeoIpError::Io(e.to_string())
            }
        })?;
        Self::from_bytes(data)
    }

    // ------------------------------------------------------------------
    // Lookup — the hot path
    // ------------------------------------------------------------------

    /// Look up an IP address and return structured GeoIP data.
    pub fn lookup(&self, ip: IpAddr) -> Option<GeoIpCity> {
        let reader = self.reader_read()?;
        let result: maxminddb::geoip2::City = reader.lookup(ip).ok()?;
        Some(GeoIpCity::from_mmdb(result))
    }

    /// Look up an IP from a string.
    pub fn lookup_str(&self, ip_str: &str) -> Option<GeoIpCity> {
        let ip: IpAddr = ip_str.parse().ok()?;
        self.lookup(ip)
    }

    // ------------------------------------------------------------------
    // Hot-reload
    // ------------------------------------------------------------------

    /// Hot-reload the database from a file (atomic swap).
    #[cfg(feature = "hot-reload")]
    pub fn update_from_file<P: AsRef<Path>>(&self, path: P) -> Result<(), GeoIpError> {
        let path = path.as_ref();
        let reader = Reader::open_readfile(path).map_err(|e| {
            let msg = e.to_string();
            if msg.contains("No such file") || msg.contains("not found") {
                GeoIpError::FileNotFound(path.display().to_string())
            } else {
                GeoIpError::InvalidDatabase(msg)
            }
        })?;
        self.reader.store(Arc::new(reader));
        Ok(())
    }

    /// Hot-reload the database from raw bytes (atomic swap).
    #[cfg(feature = "hot-reload")]
    pub fn update_from_bytes(&self, data: Vec<u8>) -> Result<(), GeoIpError> {
        let reader =
            Reader::from_source(data).map_err(|e| GeoIpError::InvalidDatabase(e.to_string()))?;
        self.reader.store(Arc::new(reader));
        Ok(())
    }

    // ------------------------------------------------------------------
    // Metadata
    // ------------------------------------------------------------------

    /// Get metadata about the current database.
    pub fn metadata(&self) -> Option<GeoIpMetadata> {
        let reader = self.reader_read()?;
        let meta = &reader.metadata;
        Some(GeoIpMetadata {
            database_type: meta.database_type.clone(),
            description: meta.description.values().next().cloned().map(|s| s.to_owned()),
            build_epoch: Some(meta.build_epoch),
            node_count: Some(meta.node_count as u64),
        })
    }

    /// Get the node count.
    pub fn node_count(&self) -> Option<u64> {
        Some(self.reader_read()?.metadata.node_count as u64)
    }

    // ------------------------------------------------------------------
    // Internal
    // ------------------------------------------------------------------

    #[cfg(feature = "hot-reload")]
    fn new_inner(reader: Reader<Vec<u8>>) -> Self {
        Self {
            reader: arc_swap::ArcSwap::from_pointee(reader),
        }
    }

    #[cfg(not(feature = "hot-reload"))]
    fn new_inner(reader: Reader<Vec<u8>>) -> Self {
        Self {
            reader: Arc::new(std::sync::RwLock::new(reader)),
        }
    }

    #[cfg(feature = "hot-reload")]
    fn reader_read(&self) -> Option<arc_swap::Guard<Arc<Reader<Vec<u8>>>>> {
        Some(self.reader.load())
    }

    #[cfg(not(feature = "hot-reload"))]
    fn reader_read(&self) -> Option<std::sync::RwLockReadGuard<'_, Reader<Vec<u8>>>> {
        self.reader.read().ok()
    }
}

// ---------------------------------------------------------------------------
// Default — built-in database
// ---------------------------------------------------------------------------

impl Default for GeoIp {
    fn default() -> Self {
        #[cfg(feature = "builtin")]
        {
            const BUILTIN_DB: &[u8] = include_bytes!("builtin/geolite2-city.mmdb");
            Self::from_bytes(BUILTIN_DB.to_vec())
                .expect("built-in database is always valid")
        }

        #[cfg(not(feature = "builtin"))]
        {
            panic!(
                "No database configured. Options:\n\
                 1. Enable the `builtin` feature (default)\n\
                 2. Use `GeoIp::open(\"path/to/file.mmdb\")`\n\
                 3. Use `GeoIp::from_bytes(data)`"
            )
        }
    }
}

impl Clone for GeoIp {
    fn clone(&self) -> Self {
        Self {
            reader: self.reader.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_bytes_invalid() {
        let result = GeoIp::from_bytes(b"not a valid mmdb".to_vec());
        assert!(result.is_err());
    }

    #[test]
    fn test_open_nonexistent() {
        let result = GeoIp::open("/nonexistent/path/to/file.mmdb");
        assert!(matches!(result, Err(GeoIpError::FileNotFound(_))));
    }
}
