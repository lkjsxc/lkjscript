//! Keep first-open recovery distinct from steady-state observation.
use super::*;

pub(super) fn copy_cold_repository(source: &Path, destination: &Path) {
    // Only the root's disposable files are omitted. Nested application files are not caches.
    copy_regular_tree_excluding(source, destination, &["LOCK", "catalog"]);
    assert!(!destination.join("LOCK").exists());
    assert!(!destination.join("catalog").exists());
}

pub(super) fn first_status(executable: &Path, directory: &Path, project: &Path) -> String {
    let before = content_inventory(project);
    let revision = current_revision_at(executable, directory, project);
    assert_recovery_delta(project, &before);
    revision
}

fn is_catalog_segment(relative: &str) -> bool {
    let path = Path::new(relative);
    let parent = Path::new("catalog").join("segments");
    let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
        return false;
    };
    let Some(digest) = name
        .strip_prefix("segment_")
        .and_then(|name| name.strip_suffix(".lkjs"))
    else {
        return false;
    };
    path.parent() == Some(parent.as_path())
        && digest.len() == 64
        && digest
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn assert_recovery_delta(project: &Path, before: &BTreeMap<String, [u8; 32]>) {
    let after = content_inventory(project);
    for (name, digest) in before {
        assert_eq!(
            after.get(name),
            Some(digest),
            "first open must preserve every existing file: {name}"
        );
    }
    let added: Vec<_> = after
        .keys()
        .filter(|name| !before.contains_key(*name))
        .collect();
    let manifest = Path::new("catalog").join("current.lkjc");
    for name in &added {
        let relative = Path::new(name);
        assert!(
            relative == Path::new("LOCK") || relative == manifest || is_catalog_segment(name),
            "first open added an unexpected file: {name}"
        );
        let metadata = std::fs::symlink_metadata(project.join(relative)).unwrap();
        assert!(metadata.is_file() && !metadata.file_type().is_symlink());
    }
    assert!(
        added
            .iter()
            .any(|name| Path::new(name) == Path::new("LOCK"))
    );
    assert!(added.iter().any(|name| Path::new(name) == manifest));
    assert!(added.iter().any(|name| is_catalog_segment(name)));
    assert!(std::fs::read(project.join("LOCK")).unwrap().is_empty());
    println!(
        "cold-open preserved-files={} added-operational-files={}",
        before.len(),
        added.len()
    );
}

#[test]
fn cold_status_query_and_inspection_preserve_authority_then_all_files() {
    let temporary = tempfile::tempdir().unwrap();
    let copied = temporary.path().join("lkjscript");
    copy_executable(&binary(), &copied);
    let source = temporary.path().join("source");
    let created = compact_success_at(
        &copied,
        temporary.path(),
        &[
            "new",
            path(&source),
            "--template",
            "command",
            "--name",
            "cold-open",
        ],
    );
    let expected = compact_field(compact_record(&created, "revision"), "id").unwrap();
    let found = compact_success_at(
        &copied,
        temporary.path(),
        &[
            "--project",
            path(&source),
            "query",
            "find",
            "module",
            "application",
        ],
    );
    let module = compact_field(compact_record(&found, "owner"), "id").unwrap();
    let observations = [
        vec!["status"],
        vec!["query", "owners", "--kind", "module"],
        vec!["inspect", "owner", "module", module],
    ];
    let source_before = content_inventory(&source);
    for (index, observation) in observations.iter().enumerate() {
        let project = temporary.path().join(format!("cold-{index}"));
        copy_cold_repository(&source, &project);
        if index == 0 {
            assert_eq!(first_status(&copied, temporary.path(), &project), expected);
        } else {
            let before = content_inventory(&project);
            let mut arguments = vec!["--project", path(&project)];
            arguments.extend_from_slice(observation);
            compact_success_at(&copied, temporary.path(), &arguments);
            assert_recovery_delta(&project, &before);
        }
        let warm = content_inventory(&project);
        assert_eq!(
            current_revision_at(&copied, temporary.path(), &project),
            expected
        );
        for observation in &observations {
            let mut arguments = vec!["--project", path(&project)];
            arguments.extend_from_slice(observation);
            compact_success_at(&copied, temporary.path(), &arguments);
            assert_eq!(content_inventory(&project), warm);
        }
    }
    assert_eq!(content_inventory(&source), source_before);
}

#[test]
fn cold_copy_omits_only_root_operational_state() {
    let temporary = tempfile::tempdir().unwrap();
    let source = temporary.path().join("source");
    std::fs::create_dir_all(source.join("catalog")).unwrap();
    std::fs::create_dir_all(source.join("nested/catalog")).unwrap();
    for name in [
        "HEAD",
        "LOCK",
        "catalog/current.lkjc",
        "nested/LOCK",
        "nested/catalog/application-data",
    ] {
        std::fs::write(source.join(name), name.as_bytes()).unwrap();
    }
    let original = content_inventory(&source);
    let complete = temporary.path().join("complete");
    copy_regular_tree(&source, &complete);
    assert_eq!(content_inventory(&complete), original);

    let cold = temporary.path().join("cold");
    copy_cold_repository(&source, &cold);
    let expected: BTreeMap<_, _> = original
        .iter()
        .filter(|(name, _)| {
            Path::new(name) != Path::new("LOCK")
                && Path::new(name) != Path::new("catalog").join("current.lkjc")
        })
        .map(|(name, digest)| (name.clone(), *digest))
        .collect();
    assert_eq!(content_inventory(&cold), expected);
    assert_eq!(content_inventory(&source), original);
}
