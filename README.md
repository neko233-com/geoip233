# geoip233

Offline GeoIP library for Rust, powered by MaxMind DB (.mmdb) format.

## Features

- **Offline database** — Open any MaxMind-compatible MMDB file
- **Hot-reload** — Update the database at runtime without downtime (opt-in)
- **High performance** — Lock-free reads via `ArcSwap`, optional mmap support
- **Thread-safe** — Safe to share `Arc<GeoIp>` across threads

## Quick Start

```rust
use geoip233::GeoIp;

let geo = GeoIp::open("/path/to/GeoLite2-City.mmdb").unwrap();
let city = geo.lookup_str("8.8.8.8").unwrap();
println!("Country: {}", city.country_str());  // "US"
println!("City: {}", city.city_str());
```

## Custom Database

```rust
use geoip233::GeoIp;

// Load your own .mmdb file
let geo = GeoIp::open("/path/to/GeoLite2-City.mmdb").unwrap();

// Or from bytes
let data = std::fs::read("database.mmdb").unwrap();
let geo = GeoIp::from_bytes(data).unwrap();
```

## Hot Reload

Enable the `hot-reload` feature to update the database at runtime:

```toml
[dependencies]
geoip233 = { version = "0.1", features = ["hot-reload"] }
```

```rust
use geoip233::GeoIp;
use std::sync::Arc;

let geo = Arc::new(GeoIp::open("/path/to/GeoLite2-City.mmdb").unwrap());

// Atomic swap — concurrent readers see old or new data, never inconsistent
geo.update_from_file("/new/database.mmdb").unwrap();
geo.update_from_bytes(new_data).unwrap();
```

## Feature Flags

| Feature | Default | Description |
|---------|---------|-------------|
| `mmap` | ✓ | Memory-mapped file I/O |
| `hot-reload` | ✗ | Enable `update()` for runtime database replacement |

## API

### Core

- `GeoIp::open(path)` — Open from file
- `GeoIp::from_bytes(data)` — Open from bytes
- `GeoIp::open_mmap(path)` — Open with mmap (feature `mmap`)

### Lookup

- `geo.lookup(ip)` — Look up `IpAddr`, returns `Option<GeoIpCity>`
- `geo.lookup_str("8.8.8.8")` — Convenience string version

### Hot-reload (feature `hot-reload`)

- `geo.update_from_file(path)` — Atomic swap from file
- `geo.update_from_bytes(data)` — Atomic swap from bytes

### Metadata

- `geo.metadata()` — Database metadata
- `geo.node_count()` — Node count for size estimation

## Getting Database Files

To obtain a GeoLite2-City database:

1. Register at [MaxMind](https://www.maxmind.com/en/geolite2/signup)
2. Download `GeoLite2-City.mmdb`
3. Use `GeoIp::open()` or `GeoIp::update_from_file()`

## License

Apache-2.0
