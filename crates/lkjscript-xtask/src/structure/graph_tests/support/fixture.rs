pub fn public_facts(root: &Path) {
    let directory = root.join("meta/config/public-facts");
    assert!(fs::create_dir_all(&directory).is_ok());
    let contract = lkjscript_contracts::current_contracts()
        .ok()
        .and_then(|contracts| {
            contracts
                .get(lkjscript_contracts::PUBLIC_FACTS)
                .map(lkjscript_contracts::RegisteredContract::digest)
        })
        .map_or_else(String::new, |digest| digest.to_hex());
    let manifest = format!(
        concat!(
            "{{\"schema\":\"lkjscript.public-facts\",\"contract\":\"{}\",",
            "\"platform_revision\":{},\"shards\":[{{\"path\":\"facts.json\",",
            "\"first\":\"test-fact\",\"last\":\"test-fact\"}}]}}"
        ),
        contract,
        lkjscript_contracts::PLATFORM_REVISION
    );
    let shard = format!(
        concat!(
            "{{\"schema\":\"lkjscript.public-fact-shard\",\"contract\":\"{}\",",
            "\"facts\":[{{\"id\":\"test-fact\",\"kind\":\"capability\",",
            "\"status\":\"current\",\"scope\":[\"repository\"],",
            "\"interface\":\"test fact\",\"exclusions\":[{{\"id\":\"outside-scope\",",
            "\"interface\":\"outside scope is excluded\"}}],",
            "\"authority\":{{\"kind\":\"repository-path\",\"path\":\"docs/decision.md\"}},",
            "\"implementation_anchors\":[\"crates/x/src/lib.rs\"],",
            "\"evidence\":[{{\"path\":\"docs/decision.md\",\"class\":\"implementation-test\"}}],",
            "\"projections\":[\"docs/decision.md\"],\"dependencies\":[],",
            "\"invalidated_by\":[],\"platform_revision\":{},\"contracts\":[]}}]}}"
        ),
        contract,
        lkjscript_contracts::PLATFORM_REVISION
    );
    assert!(fs::write(directory.join("manifest.json"), manifest).is_ok());
    assert!(fs::write(directory.join("facts.json"), shard).is_ok());
}

pub fn fixture(root: &Path, revision: &str) -> Audit {
    let content = [
        ("crates/lkjscript-core/Cargo.toml", "[package]\nname=\"lkjscript-core\"\n"),
        ("crates/lkjscript-core/src/lib.rs", "pub fn core() {}\n"),
        ("crates/x/Cargo.toml", "[package]\nname=\"x\"\n[dependencies]\nlkjscript-core={}\n"),
        ("crates/x/src/lib.rs", "pub fn api() {}\n"),
        ("crates/x/tests/sample.rs", "#[test]\nfn works() {}\n"),
        ("docs/decision.md", "[source](../src/main.lkjscript)\n"),
        ("src/main.lkjscript", concat!(
            "imports/\nimport/\nmodule/\nsrc/part.lkjscript\n/module\n",
            "declarations/\npart\n/declarations\n/import\n/imports\nmain/\n",
            "sig/\ninputs/\n/inputs\noutput/\nunit\n/output\n/sig\nunit\n/main\n")),
        ("src/part.lkjscript", concat!(
            "def/\nname/\npart\n/name\npublic\nfn/\nsig/\ninputs/\n/inputs\n",
            "output/\nunit\n/output\n/sig\nparams/\n/params\nunit\n/fn\n/def\n")),
    ];
    let mut files = Vec::new();
    for (path, text) in content {
        let absolute = root.join(path);
        assert!(fs::create_dir_all(absolute.parent().unwrap_or(root)).is_ok());
        assert!(fs::write(&absolute, text).is_ok());
        files.push(FileRecord { path: path.into(), bytes: text.len() as u64,
            lines: text.lines().count() as u64, max_physical_line_scalars: 80,
            max_ordinary_line_scalars: 80, exact_data_lines: 0, class: "authored".into(),
            capsule: path.starts_with("crates/x/").then(|| "x".into()) });
    }
    for path in ["meta/config/public-facts/manifest.json", "meta/config/public-facts/facts.json"] {
        if let Ok(text) = fs::read_to_string(root.join(path)) {
            files.push(FileRecord { path: path.into(), bytes: text.len() as u64,
                lines: text.lines().count() as u64, max_physical_line_scalars: 80,
                max_ordinary_line_scalars: 80, exact_data_lines: 0, class: "authored".into(),
                capsule: None });
        }
    }
    let mut core = capsule();
    core.id = "core".into();
    core.root = "crates/lkjscript-core".into();
    core.facade = vec!["crates/lkjscript-core/src/lib.rs".into()];
    core.allowed_dependencies.clear();
    Audit {
        schema: "lkjscript.repository-audit".into(),
        contract: lkjscript_contracts::REPOSITORY_GRAPH_DIGEST.to_hex(),
        revision: revision.into(),
        policy_identity: lkjscript_contracts::REPOSITORY_GRAPH_DIGEST.to_hex(),
        files,
        directories: [".", "crates", "crates/lkjscript-core", "crates/lkjscript-core/src",
            "crates/x", "crates/x/src", "crates/x/tests", "docs", "meta", "meta/config",
            "meta/config/public-facts", "src"].into_iter().map(|path| DirectoryRecord {
                path: path.into(), entries: 1, depth: path.matches('/').count() as u64,
            }).collect(),
        classifications: vec![], capsules: vec![core, capsule()], findings: vec![],
        provenance: vec![], unsupported: vec![],
    }
}
