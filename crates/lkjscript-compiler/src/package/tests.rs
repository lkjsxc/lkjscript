#![allow(clippy::unwrap_used)]

use std::fs;
use std::io::Cursor;
use std::path::{Path, PathBuf};
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
        interface.semantic_snapshot_constraints[0].support,
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
fn metrics_collection_does_not_change_verified_program() {
    let root = fixture("metrics-program");
    let (lock_path, lock) = create_lock(&root).unwrap();
    fs::write(lock_path, lock).unwrap();
    let entry = root.join("main.lkjscript");
    let plain = crate::compile_path(&entry).unwrap();
    let (measured, metrics) = crate::compile_path_with_metrics(&entry).unwrap();
    assert_eq!(plain.ssa(), measured.ssa());
    assert_eq!(metrics.source_files, 1);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn captured_locked_target_rejects_a_mismatched_completed_memory_plan() {
    let root = fixture("captured-target-plan");
    let (lock_path, lock) = create_lock(&root).unwrap();
    fs::write(lock_path, lock).unwrap();
    let entry = root.join("main.lkjscript");
    let program = crate::compile_path(&entry).unwrap();
    let verified = verify_for_compilation(&entry).unwrap().unwrap();
    let captured = capture_compilation(&verified).unwrap();
    assert!(captured.validate_memory_plan(program.memory_plan()).is_ok());

    let mut mismatched = program.memory_plan().clone();
    mismatched.id = crate::memory_plan::MemoryPlanId::from_bytes([0x5a; 32]);
    let result = captured.validate_memory_plan(&mismatched);
    assert!(result.is_err(), "mismatched completed plan must fail");
    let error = result.unwrap_err().to_string();
    assert!(error.contains("differs from locked target"), "{error}");
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

#[test]
fn package_reader_reaches_eof_and_rejects_metadata_growth_and_shrinkage() {
    let path = Path::new("lkjscript.package.json");
    let mut exact = Cursor::new(vec![b'x'; 4]);
    assert_eq!(
        read::bytes(&mut exact, 4, path, "package manifest").unwrap(),
        b"xxxx"
    );

    let mut grown = Cursor::new(vec![b'x'; 4]);
    let growth = read::bytes(&mut grown, 2, path, "package manifest").unwrap_err();
    assert_eq!(growth.class(), lkjscript_core::ErrorClass::Host);
    assert!(growth.to_string().contains("metadata=2; read=4"));

    let mut shortened = Cursor::new(vec![b'x']);
    let shrinkage = read::bytes(&mut shortened, 2, path, "package manifest").unwrap_err();
    assert_eq!(shrinkage.class(), lkjscript_core::ErrorClass::Host);
    assert!(shrinkage.to_string().contains("metadata=2; read=1"));

    let mut stale_host_width_hint = Cursor::new(vec![b'x']);
    let stale = read::bytes(
        &mut stale_host_width_hint,
        u64::MAX,
        path,
        "package manifest",
    )
    .unwrap_err()
    .to_string();
    assert!(stale.contains("metadata=18446744073709551615; read=1"));
}

#[test]
fn ordinary_package_check_and_compile_accept_large_manifest_and_lock() {
    const LARGE_NAME_BYTES: usize = 17 * 1024 * 1024;

    let root = fixture("large-files");
    let manifest_path = root.join(MANIFEST_FILE);
    let mut manifest: Manifest =
        serde_json::from_slice(&fs::read(&manifest_path).unwrap()).unwrap();
    manifest.name = "a".repeat(LARGE_NAME_BYTES);
    let manifest_bytes = serde_json::to_vec_pretty(&manifest).unwrap();
    assert!(manifest_bytes.len() > 1024 * 1024);
    fs::write(&manifest_path, &manifest_bytes).unwrap();

    let (lock_path, lock_bytes) = create_lock(&root).unwrap();
    assert!(lock_bytes.len() > 16 * 1024 * 1024);
    fs::write(&lock_path, &lock_bytes).unwrap();
    verify(&root).unwrap();
    crate::compile_path(&root.join("main.lkjscript")).unwrap();

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn dependency_graph_is_stack_safe_at_depth_and_reports_cycle_deterministically() {
    const DEPTH: usize = 512;

    let root = directory("deep-graph");
    let mut paths = Vec::new();
    paths.try_reserve(DEPTH).unwrap();
    paths.push(root.clone());
    for _ in 1..DEPTH {
        let child = paths.last().unwrap().join("d");
        fs::create_dir(&child).unwrap();
        paths.push(child);
    }

    let mut child_hash: Option<String> = None;
    for (index, path) in paths.iter().enumerate().rev() {
        let dependencies = child_hash
            .as_ref()
            .map(|hash| {
                vec![Dependency {
                    name: "next".into(),
                    path: "d".into(),
                    content_sha256: hash.clone(),
                }]
            })
            .unwrap_or_default();
        let manifest = empty_manifest(&format!("p{index}"), dependencies);
        write_manifest(path, &manifest);
        child_hash = Some(package_identity(&manifest));
    }

    let build_root = root.clone();
    let lock = std::thread::Builder::new()
        .name("deep-package-graph".into())
        .stack_size(256 * 1024)
        .spawn(move || graph::build(&build_root))
        .unwrap()
        .join()
        .unwrap()
        .unwrap();
    assert_eq!(lock.packages.len(), DEPTH);

    let leaf = paths.last().unwrap();
    let cycle_manifest = empty_manifest(
        "cycle",
        vec![Dependency {
            name: "self".into(),
            path: ".".into(),
            content_sha256: "0".repeat(64),
        }],
    );
    write_manifest(leaf, &cycle_manifest);
    let first = graph::build(&root).unwrap_err().to_string();
    let second = graph::build(&root).unwrap_err().to_string();
    assert_eq!(first, "local package dependency cycle");
    assert_eq!(second, first);

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn wide_dependency_graph_is_unrestricted_and_deterministic() {
    const WIDTH: usize = 512;

    let root = directory("wide-graph");
    let mut dependencies = Vec::new();
    dependencies.try_reserve(WIDTH).unwrap();
    for index in 0..WIDTH {
        let name = format!("d{index:05}");
        let child = root.join(&name);
        fs::create_dir(&child).unwrap();
        let manifest = empty_manifest(&name, Vec::new());
        write_manifest(&child, &manifest);
        dependencies.push(Dependency {
            name: name.clone(),
            path: name,
            content_sha256: package_identity(&manifest),
        });
    }
    write_manifest(&root, &empty_manifest("wide", dependencies));

    let first = graph::build(&root).unwrap();
    let second = graph::build(&root).unwrap();
    assert_eq!(first.packages.len(), WIDTH + 1);
    assert_eq!(second, first);

    fs::remove_dir_all(root).unwrap();
}

fn empty_manifest(name: &str, dependencies: Vec<Dependency>) -> Manifest {
    Manifest {
        schema: "lkjscript.package".into(),
        contract: contracts::expected(lkjscript_contracts::PACKAGE_MANIFEST)
            .unwrap()
            .to_hex(),
        name: name.into(),
        source_root: ".".into(),
        modules: Vec::new(),
        public: Vec::new(),
        dependencies,
        capabilities: Vec::new(),
        targets: Vec::new(),
    }
}

fn write_manifest(root: &Path, manifest: &Manifest) {
    fs::write(
        root.join(MANIFEST_FILE),
        serde_json::to_vec_pretty(manifest).unwrap(),
    )
    .unwrap();
}

fn package_identity(manifest: &Manifest) -> String {
    let manifest_bytes = serde_json::to_vec(manifest).unwrap();
    let manifest_hash = lkjscript_contracts::ContractDigest::from_bytes(
        lkjscript_contracts::sha256(&manifest_bytes),
    )
    .to_hex();
    let modules = Vec::<LockedModule>::new();
    let targets = Vec::<LockedTargetMemory>::new();
    let memory_hash = graph::package_memory_hash(&modules).unwrap();
    graph::package_hash(&manifest_hash, &memory_hash, &modules, &targets).unwrap()
}

mod forgery;
mod library;
