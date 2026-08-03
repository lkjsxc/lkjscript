use std::fs;
use std::path::{Path, PathBuf};

use lkjscript_core::Limits;

#[test]
fn all_101_sources_project_to_closed_schema_without_byte_changes() -> std::io::Result<()> {
    let workspace = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut files = Vec::new();
    collect_sources(&workspace.join("src"), &mut files)?;
    collect_sources(
        &workspace.join("crates/lkjscript-app/tests/fixtures"),
        &mut files,
    )?;
    collect_sources(
        &workspace.join("meta/benchmarks/jit/pre-jit-workload"),
        &mut files,
    )?;
    files.sort();
    assert_eq!(files.len(), 101, "tracked source corpus changed");
    for path in files {
        project_source(&workspace, &path)?;
    }
    Ok(())
}

fn project_source(workspace: &Path, path: &Path) -> std::io::Result<()> {
    let source = fs::read_to_string(path)?;
    let logical = path
        .strip_prefix(workspace)
        .expect("workspace source")
        .to_str()
        .expect("workspace source path must be UTF-8");
    let tree = crate::source::validate(&source, logical, &Limits::default())
        .unwrap_or_else(|error| panic!("{}: {error}", path.display()));
    assert_eq!(
        tree.format_single_source().as_deref(),
        Some(source.as_str())
    );
    let units = crate::semantic::tree::source_units(&tree);
    let nodes = crate::semantic::tree::node_records(&tree);
    assert_eq!(nodes.len(), tree.nodes().len());
    let declarations = tree
        .declarations()
        .iter()
        .map(|item| crate::semantic::tree::declaration_record(&tree, item))
        .collect::<Vec<_>>();
    let source_nodes = crate::semantic::tree::source_nodes(&tree);
    for declaration in tree.declarations() {
        let index = declaration.node().index();
        let subtree =
            crate::semantic::tree::subtree_record(&tree, index).expect("exact declaration subtree");
        let encoded = serde_json::to_vec(&subtree).expect("encode closed subtree");
        let decoded: crate::semantic::schema::SemanticSubtreeRecord =
            serde_json::from_slice(&encoded).expect("decode closed subtree");
        assert_eq!(decoded.node.index, index);
        let rebuilt = decoded.to_source().expect("validate closed subtree");
        assert_eq!(
            crate::source::format_node_source(&rebuilt),
            crate::source::format_node_source(source_nodes[index as usize])
        );
    }
    let snapshot = crate::semantic::schema::SnapshotResult {
        repository_identity: "corpus-fixture".to_string(),
        tree_identity: tree.identity().to_hex(),
        source_units: units,
        declarations,
        nodes,
    };
    let first = serde_json::to_vec(&snapshot).expect("encode closed snapshot");
    assert!(!String::from_utf8_lossy(&first).contains("edition"));
    let second = serde_json::to_vec(&snapshot).expect("re-encode closed snapshot");
    assert_eq!(first, second, "{}", path.display());
    Ok(())
}

fn collect_sources(directory: &Path, output: &mut Vec<PathBuf>) -> std::io::Result<()> {
    for entry in fs::read_dir(directory)? {
        let entry = entry?;
        if entry.file_type()?.is_dir() {
            collect_sources(&entry.path(), output)?;
        } else if entry.path().extension().and_then(|value| value.to_str()) == Some("lkjscript") {
            output.push(entry.path());
        }
    }
    Ok(())
}
