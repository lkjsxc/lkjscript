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
        "main/\nsig/\ninputs/\n/inputs\noutput/\nunit\n/output\n/sig\nunit\n/main\n",
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
        "main/\nsig/\ninputs/\n/inputs\noutput/\nunit\n/output\n/sig\ndo/\nunit\n/do\n/main\n",
    )
    .unwrap();
    assert!(verify(&root).unwrap_err().to_string().contains("stale"));
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
            "enum/\nname/\ncolor\n/name\npublic\nvariants/\n",
            "variant/\nname/\nred\n/name\nfields/\n/fields\n/variant\n",
            "/variants\n/enum\n"
        ),
    )
    .unwrap();
    let manifest_path = root.join(MANIFEST_FILE);
    let mut manifest: Manifest =
        serde_json::from_slice(&fs::read(&manifest_path).unwrap()).unwrap();
    manifest.modules.push("color.lkjscript".into());
    manifest.modules.sort();
    manifest.public.push("color.lkjscript".into());
    manifest.public.sort();
    fs::write(manifest_path, serde_json::to_vec_pretty(&manifest).unwrap()).unwrap();
    let lock = graph::build(&root).unwrap();
    let module = lock.packages[0]
        .modules
        .iter()
        .find(|module| module.id == "color.lkjscript")
        .unwrap();
    assert_eq!(module.exports, ["color", "red"]);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn public_generic_interface_binds_hidden_transport_witness() {
    let root = fixture("generic-witness");
    fs::write(
        root.join("history.lkjscript"),
        concat!(
            "def/\nname/\nkeep\n/name\npublic\nfn/\nforall/\nt\n/forall\n",
            "bounds/\nbound/\nt\ncopy\n/bound\n/bounds\n",
            "sig/\ninputs/\nt\n/inputs\noutput/\nt\n/output\n/sig\n",
            "params/\nvalue\nt\n/params\nvalue\n/fn\n/def\n"
        ),
    )
    .unwrap();
    let manifest_path = root.join(MANIFEST_FILE);
    let mut manifest: Manifest =
        serde_json::from_slice(&fs::read(&manifest_path).unwrap()).unwrap();
    manifest.modules.push("history.lkjscript".into());
    manifest.modules.sort();
    manifest.public.push("history.lkjscript".into());
    manifest.public.sort();
    fs::write(manifest_path, serde_json::to_vec_pretty(&manifest).unwrap()).unwrap();
    let lock = graph::build(&root).unwrap();
    let module = lock.packages[0]
        .modules
        .iter()
        .find(|module| module.id == "history.lkjscript")
        .unwrap();
    assert_eq!(module.exports, ["keep"]);
    assert_eq!(module.memory_interfaces.len(), 1);
    let interface = &module.memory_interfaces[0];
    assert_eq!(interface.name, "keep");
    assert_eq!(interface.type_parameters, ["t"]);
    assert_eq!(interface.trait_parameters.len(), 1);
    assert_eq!(interface.trait_parameters[0].parameter, "t");
    assert_eq!(interface.trait_parameters[0].trait_name, "copy");
    assert_eq!(interface.trait_parameters[0].trait_identity.len(), 64);
    assert_eq!(interface.memory_requirements.len(), 1);
    assert_eq!(interface.memory_requirements[0].parameter, "t");
    assert_eq!(interface.memory_requirements[0].operations, ["transport"]);
    assert_eq!(interface.parameter_modes, [LockedMemoryParameterMode::Copy]);
    assert_eq!(interface.result_mode, LockedMemoryResultMode::Trivial);
    assert_eq!(
        interface.equality_constraints[0].support,
        LockedConstraintSupport::CallerWitnessRequired
    );
    assert_eq!(
        interface.process_codec_constraints[0].support,
        LockedConstraintSupport::CallerWitnessRequired
    );
    assert_eq!(interface.package_memory_interface_sha256.len(), 64);
    assert_eq!(module.module_interface_sha256.len(), 64);
    assert_eq!(module.module_memory_interface_sha256.len(), 64);
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
fn metrics_collection_does_not_change_prepared_identity() {
    let root = fixture("metrics-identity");
    let (lock_path, lock) = create_lock(&root).unwrap();
    fs::write(lock_path, lock).unwrap();
    let entry = root.join("main.lkjscript");
    let plain = crate::compile_path(&entry, &lkjscript_core::Limits::default()).unwrap();
    let (measured, metrics) =
        crate::compile_path_with_metrics(&entry, &lkjscript_core::Limits::default()).unwrap();
    assert_eq!(plain.prepared_identity(), measured.prepared_identity());
    assert_eq!(metrics.source_files, 1);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn removed_resource_profile_manifest_field_is_rejected() {
    let root = fixture("removed-profile");
    let path = root.join(MANIFEST_FILE);
    let mut manifest: serde_json::Value =
        serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    manifest["resource_profile"] = serde_json::Value::String("default".into());
    fs::write(&path, serde_json::to_vec_pretty(&manifest).unwrap()).unwrap();
    let error = graph::build(&root).unwrap_err().to_string();
    assert!(error.contains("resource_profile") || error.contains("unknown field"));
    fs::remove_dir_all(root).unwrap();
}

mod forgery;
mod library;
