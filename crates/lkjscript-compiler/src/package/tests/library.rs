use super::*;

fn library(root: &std::path::Path) {
    fs::create_dir_all(root).unwrap();
    fs::write(
        root.join("history.lkjscript"),
        concat!(
            "def/\nname/\nkeep\n/name\npublic\nfn/\nforall/\nt\n/forall\n",
            "sig/\ninputs/\nt\n/inputs\noutput/\nt\n/output\n/sig\n",
            "params/\nvalue\nt\n/params\nvalue\n/fn\n/def\n"
        ),
    )
    .unwrap();
    let manifest = Manifest {
        schema: "lkjscript.package".into(),
        contract: contracts::expected(lkjscript_contracts::PACKAGE_MANIFEST)
            .unwrap()
            .to_hex(),
        name: "history".into(),
        source_root: ".".into(),
        modules: vec!["history.lkjscript".into()],
        public: vec!["history.lkjscript".into()],
        dependencies: Vec::new(),
        capabilities: Vec::new(),
        targets: Vec::new(),
    };
    fs::write(
        root.join(MANIFEST_FILE),
        serde_json::to_vec_pretty(&manifest).unwrap(),
    )
    .unwrap();
}

#[test]
fn no_main_public_library_is_hir_derived_in_cross_package_lock() {
    let root = fixture("cross-package");
    let dependency_root = root.join("history");
    library(&dependency_root);
    let dependency_hash = graph::build(&dependency_root).unwrap().root;
    let manifest_path = root.join(MANIFEST_FILE);
    let mut manifest: Manifest =
        serde_json::from_slice(&fs::read(&manifest_path).unwrap()).unwrap();
    manifest.dependencies.push(Dependency {
        name: "history".into(),
        path: "history".into(),
        content_sha256: dependency_hash,
    });
    fs::write(
        &manifest_path,
        serde_json::to_vec_pretty(&manifest).unwrap(),
    )
    .unwrap();
    let (path, bytes) = create_lock(&root).unwrap();
    fs::write(path, bytes).unwrap();
    assert!(verify(&root).is_ok());
    let (lock, _) = encoding::read(&root).unwrap();
    assert_eq!(lock.packages.len(), 2);
    let dependency = lock
        .packages
        .iter()
        .find(|package| package.name == "history")
        .unwrap();
    assert!(dependency.targets.is_empty());
    let interface = &dependency.modules[0].memory_interfaces[0];
    assert_eq!(interface.name, "keep");
    assert_eq!(interface.type_parameters, ["t"]);
    assert_eq!(interface.memory_requirements[0].operations, ["transport"]);
    assert_eq!(interface.parameter_modes, [LockedMemoryParameterMode::Copy]);
    assert_eq!(interface.result_mode, LockedMemoryResultMode::Trivial);
    fs::remove_dir_all(root).unwrap();
}
