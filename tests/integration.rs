use geoip233::GeoIp;

#[test]
fn test_lookup_builtin_database() {
    let geo = GeoIp::default();

    // Google DNS — known to be in GeoLite2
    let city = geo.lookup_str("8.8.8.8").expect("should find 8.8.8.8");
    println!("8.8.8.8 -> {:?}", city);
    assert_eq!(city.country_str(), "US");
    assert!(city.latitude.is_some());
    assert!(city.longitude.is_some());

    // Baidu DNS — should be in China
    let city = geo.lookup_str("180.76.76.76").expect("should find baidu dns");
    println!("180.76.76.76 -> {:?}", city);
    assert_eq!(city.country_str(), "CN");
}

#[test]
fn test_lookup_private_ip_returns_none() {
    let geo = GeoIp::default();
    assert!(geo.lookup_str("127.0.0.1").is_none());
    assert!(geo.lookup_str("192.168.1.1").is_none());
    assert!(geo.lookup_str("10.0.0.1").is_none());
}

#[test]
fn test_clone_shares_reader() {
    let geo = GeoIp::default();
    let geo2 = geo.clone();

    let city1 = geo.lookup_str("8.8.8.8");
    let city2 = geo2.lookup_str("8.8.8.8");
    assert_eq!(
        city1.as_ref().map(|c| c.country_str()),
        city2.as_ref().map(|c| c.country_str())
    );
}

#[test]
fn test_metadata() {
    let geo = GeoIp::default();
    let meta = geo.metadata().expect("should have metadata");
    println!("Metadata: {:?}", meta);
    assert!(!meta.database_type.is_empty());
    assert!(meta.node_count > Some(0));
}

#[test]
fn test_invalid_ip_string() {
    let geo = GeoIp::default();
    assert!(geo.lookup_str("not-an-ip").is_none());
    assert!(geo.lookup_str("").is_none());
}
