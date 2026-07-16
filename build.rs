use std::fs;
use std::path::Path;
use std::process::Command;

const GEOLITE2_CITY_URL: &str =
    "https://github.com/P3TERX/GeoLite.mmdb/raw/download/GeoLite2-City.mmdb";

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=src/builtin/");

    let dest_dir = Path::new("src/builtin");
    let dest_file = dest_dir.join("geolite2-city.mmdb");

    // If the builtin MMDB already exists and is recent, skip download
    if dest_file.exists() {
        let meta = fs::metadata(&dest_file).unwrap();
        // Re-download if older than 7 days
        let age = meta
            .modified()
            .unwrap()
            .elapsed()
            .unwrap_or(std::time::Duration::from_secs(0));
        if age < std::time::Duration::from_secs(7 * 24 * 3600) {
            println!(
                "cargo:warning=Using existing builtin MMDB ({} MB)",
                meta.len() / 1024 / 1024
            );
            return;
        }
    }

    println!("cargo:warning=Downloading GeoLite2-City.mmdb...");

    // Ensure directory exists
    fs::create_dir_all(dest_dir).ok();

    // Try curl first, then wget
    let downloaded = Command::new("curl")
        .args([
            "-fsSL",
            "--connect-timeout", "10",
            "--max-time", "60",
            "-o", dest_file.to_str().unwrap(),
            GEOLITE2_CITY_URL,
        ])
        .status()
        .and_then(|s| {
            if s.success() {
                Ok(())
            } else {
                Err(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    "curl failed",
                ))
            }
        })
        .or_else(|_| {
            Command::new("wget")
                .args([
                    "-q",
                    "--timeout=10",
                    "-O", dest_file.to_str().unwrap(),
                    GEOLITE2_CITY_URL,
                ])
                .status()
                .and_then(|s| {
                    if s.success() {
                        Ok(())
                    } else {
                        Err(std::io::Error::new(
                            std::io::ErrorKind::Other,
                            "wget failed",
                        ))
                    }
                })
        });

    match downloaded {
        Ok(()) => {
            let size = fs::metadata(&dest_file).map(|m| m.len()).unwrap_or(0);
            if size < 1000 {
                println!(
                    "cargo:warning=Downloaded file too small ({} bytes), may be invalid",
                    size
                );
                fs::remove_file(&dest_file).ok();
                create_fallback_mmdb(&dest_file);
            } else {
                println!(
                    "cargo:warning=Downloaded GeoLite2-City.mmdb ({} MB)",
                    size / 1024 / 1024
                );
            }
        }
        Err(_) => {
            println!("cargo:warning=Failed to download MMDB (no internet or tool missing). Creating minimal fallback.");
            create_fallback_mmdb(&dest_file);
        }
    }
}

fn create_fallback_mmdb(dest: &Path) {
    // Create a minimal but valid MMDB file
    // This covers the binary format specification: https://maxmind.github.io/MaxMind-DB/
    //
    // Structure:
    // [data section] [metadata section] [metadata marker: 0xefffffff]

    let mut mmdb = Vec::with_capacity(4096);

    // === DATA SECTION ===
    // Minimal data: an empty search tree with a single record pointing to a map
    // For simplicity, we create a data section with just metadata information
    // and no actual IP records. This means lookups will return no results,
    // but the file is structurally valid.

    // Data section: empty map (marker byte 0xe0 = map type 7, size 0)
    // 0xe0 = (0 << 3) | 7 = empty map
    mmdb.push(0xe0);

    // === METADATA SECTION ===
    let metadata = vec![
        ("binary_format_major_version", MetaVal::U16(2)),
        ("binary_format_minor_version", MetaVal::U16(0)),
        ("build_epoch", MetaVal::U64(1700000000)),
        ("database_type", MetaVal::Str("GeoLite2-City".into())),
        (
            "description",
            MetaVal::Map(vec![(
                "en".into(),
                MetaVal::Str("geoip233 fallback database".into()),
            )]),
        ),
        ("ip_version", MetaVal::U16(6)),
        ("node_count", MetaVal::U32(0)),
        ("record_size", MetaVal::U16(24)),
    ];

    encode_map(&mut mmdb, &metadata);

    // === METADATA MARKER ===
    mmdb.extend_from_slice(&[0xe0, 0xff, 0xff, 0xff, 0xff]);

    fs::write(dest, &mmdb).expect("Failed to write fallback MMDB");
    println!("cargo:warning=Created minimal fallback MMDB ({} bytes)", mmdb.len());
}

// --- MMDB encoding helpers ---

enum MetaVal {
    U16(u16),
    U32(u32),
    U64(u64),
    Str(String),
    Map(Vec<(String, MetaVal)>),
}

fn encode_control(buf: &mut Vec<u8>, size: u32, type_num: u8) {
    let t = type_num as u32;
    if size < 29 {
        buf.push(((size << 3) | t) as u8);
    } else if size < 29 + 256 {
        buf.push(((29 << 3) | t) as u8);
        buf.push((size - 29) as u8);
    } else if size < 29 + 256 + 65536 {
        buf.push(((30 << 3) | t) as u8);
        let s = size - 29 - 256;
        buf.extend_from_slice(&(s as u16).to_be_bytes());
    } else {
        buf.push(((31 << 3) | t) as u8);
        buf.extend_from_slice(&(size - 29 - 256 - 65536).to_be_bytes());
    }
}

fn encode_utf8(buf: &mut Vec<u8>, s: &str) {
    let bytes = s.as_bytes();
    encode_control(buf, bytes.len() as u32, 2); // type 2 = UTF-8 string
    buf.extend_from_slice(bytes);
}

fn encode_uint(buf: &mut Vec<u8>, val: u64, max_bytes: usize) {
    let bytes = val.to_be_bytes();
    // For val=0, we still need at least 1 byte
    let leading = if val == 0 {
        max_bytes - 1
    } else {
        bytes.iter().position(|&b| b != 0).unwrap_or(max_bytes)
    };
    let size = max_bytes - leading;
    encode_control(buf, size as u32, 6); // type 6 = uint
    buf.extend_from_slice(&bytes[leading..]);
}

fn encode_map(buf: &mut Vec<u8>, entries: &[(&str, MetaVal)]) {
    encode_control(buf, entries.len() as u32, 7); // type 7 = map
    for (key, val) in entries {
        encode_utf8(buf, key);
        encode_val(buf, val);
    }
}

fn encode_val(buf: &mut Vec<u8>, val: &MetaVal) {
    match val {
        MetaVal::U16(v) => encode_uint(buf, *v as u64, 2),
        MetaVal::U32(v) => encode_uint(buf, *v as u64, 4),
        MetaVal::U64(v) => encode_uint(buf, *v, 8),
        MetaVal::Str(s) => encode_utf8(buf, s),
        MetaVal::Map(entries) => {
            encode_control(buf, entries.len() as u32, 7);
            for (key, val) in entries {
                encode_utf8(buf, key);
                encode_val(buf, val);
            }
        }
    }
}
