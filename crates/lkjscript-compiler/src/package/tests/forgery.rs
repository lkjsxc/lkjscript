use super::*;

fn add_public_module(root: &std::path::Path, id: &str, source: &str) {
    fs::write(root.join(id), source).unwrap();
    let path = root.join(MANIFEST_FILE);
    let mut manifest: Manifest = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    manifest.modules.push(id.into());
    manifest.modules.sort();
    manifest.public.push(id.into());
    manifest.public.sort();
    fs::write(path, serde_json::to_vec_pretty(&manifest).unwrap()).unwrap();
}

fn generic_fixture(label: &str) -> PathBuf {
    let root = fixture(label);
    add_public_module(
        &root,
        "generic.lkjscript",
        concat!(
            "def/\nname/\npair-first\n/name\npublic\nfn/\nforall/\nt\nu\n/forall\n",
            "sig/\ninputs/\nt\nu\n/inputs\noutput/\nt\n/output\n/sig\n",
            "params/\nfirst\nt\nsecond\nu\n/params\nfirst\n/fn\n/def\n"
        ),
    );
    root
}

fn structural_target_fixture(label: &str) -> PathBuf {
    let root = fixture(label);
    fs::write(
        root.join("main.lkjscript"),
        concat!(
            "product/\nname/\nbox\n/name\npublic\nfields/\nfield/\nname/\nvalue\n/name\n",
            "type/\ni64\n/type\n/field\n/fields\n/product\n",
            "main/\nsig/\ninputs/\n/inputs\noutput/\nproduct/\nbox\n/product\n/output\n/sig\n",
            "product-value/\nbox\nfield/\nvalue\n1\n/field\n/product-value\n/main\n"
        ),
    )
    .unwrap();
    root
}

fn assert_rejected(root: &std::path::Path, mutate: impl FnOnce(&mut LockFile)) {
    let mut lock = graph::build(root).unwrap();
    mutate(&mut lock);
    fs::write(root.join(LOCK_FILE), encoding::encode(&lock).unwrap()).unwrap();
    assert!(verify(root).unwrap_err().to_string().contains("stale"));
}

fn interface(lock: &mut LockFile) -> &mut PackageMemoryInterface {
    lock.packages
        .iter_mut()
        .find(|package| package.origin == ".")
        .unwrap()
        .modules
        .iter_mut()
        .find(|module| module.id == "generic.lkjscript")
        .unwrap()
        .memory_interfaces
        .first_mut()
        .unwrap()
}

#[test]
fn exact_hir_memory_interface_rejects_missing_extra_and_reordered_facts() {
    let root = generic_fixture("interface-forgery");
    assert_rejected(&root, |lock| {
        interface(lock).memory_requirements[0].operations.clear();
    });
    assert_rejected(&root, |lock| {
        interface(lock).memory_requirements[0]
            .operations
            .push("dispose".into());
    });
    assert_rejected(&root, |lock| {
        interface(lock).memory_requirements[0].operations =
            vec!["dispose".into(), "transport".into()];
    });
    assert_rejected(&root, |lock| {
        interface(lock).memory_requirements.reverse();
    });
    assert_rejected(&root, |lock| {
        interface(lock).type_parameters.remove(0);
    });
    assert_rejected(&root, |lock| {
        interface(lock).type_parameters.push("v".into());
    });
    assert_rejected(&root, |lock| {
        interface(lock).type_parameters.reverse();
    });
    assert_rejected(&root, |lock| {
        interface(lock).parameter_modes[0] = LockedMemoryParameterMode::Consume;
    });
    assert_rejected(&root, |lock| {
        interface(lock).result_mode = LockedMemoryResultMode::Owned;
    });
    assert_rejected(&root, |lock| {
        interface(lock).package_memory_interface_sha256 = "00".repeat(32);
    });
    assert_rejected(&root, |lock| {
        let module = lock.packages[0]
            .modules
            .iter_mut()
            .find(|module| module.id == "generic.lkjscript")
            .unwrap();
        module.module_interface_sha256 = "00".repeat(32);
    });
    assert_rejected(&root, |lock| {
        let module = lock.packages[0]
            .modules
            .iter_mut()
            .find(|module| module.id == "generic.lkjscript")
            .unwrap();
        module.module_memory_interface_sha256 = "00".repeat(32);
    });
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn target_plan_witness_member_and_dependency_closures_are_exact() {
    let root = fixture("target-forgery");
    assert_rejected(&root, |lock| {
        lock.packages[0].targets[0].memory_plan_id = "00".repeat(32)
    });
    assert_rejected(&root, |lock| {
        lock.packages[0].targets[0].witness_groups.remove(0);
    });
    assert_rejected(&root, |lock| {
        let mut forged = lock.packages[0].targets[0].witness_groups[0].clone();
        forged.group = "ff".repeat(32);
        lock.packages[0].targets[0].witness_groups.push(forged);
    });
    assert_rejected(&root, |lock| {
        lock.packages[0].targets[0].witness_groups[0].members[0].member = "11".repeat(32);
    });
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn external_witness_dependency_closure_is_exact() {
    let root = structural_target_fixture("dependency-forgery");
    let lock = graph::build(&root).unwrap();
    assert!(!lock.packages[0].targets[0]
        .external_witness_dependencies
        .is_empty());
    assert_rejected(&root, |lock| {
        lock.packages[0].targets[0]
            .external_witness_dependencies
            .remove(0);
    });
    assert_rejected(&root, |lock| {
        let mut forged = lock.packages[0].targets[0].external_witness_dependencies[0].clone();
        forged.target_group = "ff".repeat(32);
        lock.packages[0].targets[0]
            .external_witness_dependencies
            .push(forged);
    });
    assert_rejected(&root, |lock| {
        lock.packages[0].targets[0].external_witness_dependencies[0].target_member =
            "11".repeat(32);
    });
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn scanner_shaped_legacy_record_is_not_decodable() {
    let root = generic_fixture("scanner-record");
    let (_, bytes) = create_lock(&root).unwrap();
    let mut value: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    value["packages"][0]["modules"][0]["witness_requirements"] = serde_json::json!([]);
    fs::write(root.join(LOCK_FILE), serde_json::to_vec(&value).unwrap()).unwrap();
    let error = verify(&root).unwrap_err().to_string();
    assert!(error.contains("unknown field") || error.contains("canonically encoded"));
    fs::remove_dir_all(root).unwrap();
}
