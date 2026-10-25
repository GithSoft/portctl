use portctl::allocator::PortAllocator;
use portctl::detector::{detect_project_name, find_project_root};
use portctl::probe::probe_port;
use portctl::registry::{CanonicalAppItem, CanonicalAppsFile, PortRegistry};
use std::fs;
use tempfile::tempdir;

#[test]
fn test_find_project_root_and_detect_name() {
    let tmp = tempdir().unwrap();
    let root = tmp.path();
    let pkg_json = root.join("package.json");
    fs::write(&pkg_json, r#"{"name": "@my-scope/awesome-service"}"#).unwrap();

    let sub_dir = root.join("src/deep/nested");
    fs::create_dir_all(&sub_dir).unwrap();

    assert_eq!(find_project_root(&sub_dir), root);
    assert_eq!(detect_project_name(&sub_dir), "awesome-service");
}

#[test]
fn test_probe_port_unbound() {
    // Unbound high port
    let probe = probe_port(59123);
    assert!(!probe.is_listening);
    assert!(probe.pid.is_none());
}

#[test]
fn test_registry_canonical_roundtrip() {
    let tmp = tempdir().unwrap();
    let registry_dir = tmp.path().join("registry");
    let config_dir = tmp.path().join("config");

    let registry = PortRegistry::with_dirs(registry_dir, config_dir);
    let mut apps = CanonicalAppsFile::default();
    apps.apps.insert(
        "test-app".to_string(),
        CanonicalAppItem {
            port: 4100,
            tunnel: Some("test.example.com".to_string()),
            repo: Some("https://github.com/example/test".to_string()),
            description: Some("Test App".to_string()),
        },
    );

    registry.save_canonical_apps(&apps).unwrap();

    let loaded = registry.load_canonical_apps();
    assert!(loaded.apps.contains_key("test-app"));
    assert_eq!(loaded.apps.get("test-app").unwrap().port, 4100);

    let found = registry.find_by_name("test-app").unwrap();
    assert_eq!(found.port, 4100);
    assert_eq!(found.tunnel.as_deref(), Some("test.example.com"));

    let by_port = registry.find_by_port(4100).unwrap();
    assert_eq!(by_port.name, "test-app");
}

#[test]
fn test_allocator_conflict_and_reuse() {
    let tmp = tempdir().unwrap();
    let registry_dir = tmp.path().join("registry");
    let config_dir = tmp.path().join("config");

    let registry = PortRegistry::with_dirs(registry_dir, config_dir);
    let allocator = PortAllocator::new(&registry);

    // Allocate first
    let res1 = allocator
        .allocate("app-one", Some(4200), Some("global"), None, None, None)
        .unwrap();
    assert_eq!(res1.port, 4200);
    assert!(res1.is_new);

    // Re-allocate same app returns existing
    let res2 = allocator
        .allocate("app-one", None, None, None, None, None)
        .unwrap();
    assert_eq!(res2.port, 4200);
    assert!(!res2.is_new);

    // Conflict test: allocate different app with same port
    let err = allocator
        .allocate("app-two", Some(4200), Some("global"), None, None, None)
        .unwrap_err();
    assert!(err.contains("already claimed by service 'app-one'"));
}
