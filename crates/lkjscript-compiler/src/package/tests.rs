#![allow(clippy::unwrap_used)]

use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use super::*;

static NEXT: AtomicU64 = AtomicU64::new(0);

fn directory(label: &str) -> PathBuf {
    let nonce = NEXT.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!(
        "lkjscript-package-{label}-{}-{nonce}",
        std::process::id()
    ));
    fs::create_dir_all(&path).unwrap();
    path
}

fn fixture(label: &str) -> PathBuf {
    let root = directory(label);
    fs::write(
        root.join("main.lkjscript"),
        "main/\nsig/\n->\nUnit\n/sig\nunit\n/main\n",
    )
    .unwrap();
    let manifest = Manifest {
        schema: "lkjscript.package".into(),
        contract: contracts::expected(lkjscript_contracts::PACKAGE_MANIFEST)
            .unwrap()
            .to_hex(),
        name: "fixture".into(),
        source_root: ".".into(),
        modules: vec!["main.lkjscript".into()],
        public: vec!["main.lkjscript".into()],
        dependencies: Vec::new(),
        capabilities: Vec::new(),
        targets: vec![Target {
            name: "main".into(),
            module: "main.lkjscript".into(),
        }],
        resource_profile: None,
    };
    fs::write(
        root.join(MANIFEST_FILE),
        serde_json::to_vec_pretty(&manifest).unwrap(),
    )
    .unwrap();
    root
}

#[test]
fn canonical_lock_detects_every_source_change() {
    let root = fixture("stale");
    let (path, bytes) = create_lock(&root).unwrap();
    fs::write(path, bytes).unwrap();
    assert!(verify(&root).is_ok());
    fs::write(
        root.join("main.lkjscript"),
        "main/\nsig/\n->\nUnit\n/sig\ndo/\nunit\n/do\n/main\n",
    )
    .unwrap();
    assert!(verify(&root).unwrap_err().to_string().contains("stale"));
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn execution_identity_binds_entry_source_module_package_and_lock() {
    let root = fixture("execution-identity");
    let (path, bytes) = create_lock(&root).unwrap();
    fs::write(path, bytes).unwrap();
    let (_, _, first) = execution_identity(&root.join("main.lkjscript")).unwrap();
    assert_eq!(first.module_path, "main.lkjscript");
    assert_ne!(first.source_sha256, [0; 32]);
    assert_ne!(first.module_sha256, [0; 32]);
    assert_ne!(first.package_sha256, [0; 32]);
    assert_ne!(first.lock_sha256, [0; 32]);

    fs::write(
        root.join("main.lkjscript"),
        "main/\nsig/\n->\nUnit\n/sig\ndo/\nunit\n/do\n/main\n",
    )
    .unwrap();
    let (path, bytes) = create_lock(&root).unwrap();
    fs::write(path, bytes).unwrap();
    let (_, _, changed) = execution_identity(&root.join("main.lkjscript")).unwrap();
    assert_ne!(first.source_sha256, changed.source_sha256);
    assert_ne!(first.module_sha256, changed.module_sha256);
    assert_ne!(first.package_sha256, changed.package_sha256);
    assert_ne!(first.lock_sha256, changed.lock_sha256);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn lock_contracts_are_full_current_digests() {
    let root = fixture("contracts");
    let lock = graph::build(&root).unwrap();
    assert_eq!(lock.contracts, contracts::all().unwrap());
    assert!(lock.contracts.values().all(|digest| digest.len() == 64));
    assert_eq!(lock.packages.len(), 1);
    assert_eq!(lock.packages[0].modules.len(), 1);
    assert_eq!(lock.packages[0].modules[0].exports, Vec::<String>::new());
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn public_enum_identity_includes_its_variants() {
    let root = fixture("enum-exports");
    fs::write(
        root.join("color.lkjscript"),
        concat!(
            "enum/\nname/\nColor\n/name\npublic\nvariants/\n",
            "variant/\nname/\nRed\n/name\nfields/\n/fields\n/variant\n",
            "/variants\n/enum\n"
        ),
    )
    .unwrap();
    let manifest_path = root.join(MANIFEST_FILE);
    let mut manifest: Manifest =
        serde_json::from_slice(&fs::read(&manifest_path).unwrap()).unwrap();
    manifest.modules.push("color.lkjscript".into());
    manifest.modules.sort();
    fs::write(manifest_path, serde_json::to_vec_pretty(&manifest).unwrap()).unwrap();
    let lock = graph::build(&root).unwrap();
    let module = lock.packages[0]
        .modules
        .iter()
        .find(|module| module.id == "color.lkjscript")
        .unwrap();
    assert_eq!(module.exports, ["Color", "Red"]);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn package_capabilities_are_closed_and_exact() {
    let root = fixture("capabilities");
    let path = root.join(MANIFEST_FILE);
    let mut manifest: Manifest = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    manifest.capabilities = vec!["AmbientEverything".into()];
    fs::write(&path, serde_json::to_vec_pretty(&manifest).unwrap()).unwrap();
    let error = graph::build(&root).unwrap_err().to_string();
    assert!(error.contains("unknown package capability"));
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn lock_decoder_rejects_noncanonical_bytes() {
    let root = fixture("encoding");
    let (path, bytes) = create_lock(&root).unwrap();
    let lock: LockFile = serde_json::from_slice(&bytes).unwrap();
    let altered = serde_json::to_vec_pretty(&lock).unwrap();
    assert_ne!(altered, bytes);
    fs::write(path, altered).unwrap();
    assert!(verify(&root)
        .unwrap_err()
        .to_string()
        .contains("canonically encoded"));
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn exact_contract_map_is_sorted() {
    let contracts = contracts::all().unwrap();
    let keys: Vec<_> = contracts.keys().cloned().collect();
    let mut expected = keys.clone();
    expected.sort();
    assert_eq!(keys, expected);
}
