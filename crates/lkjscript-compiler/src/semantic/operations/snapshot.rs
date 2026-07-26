use crate::semantic::schema::SnapshotResult;
use crate::source::ValidatedSourceTree;

pub(crate) fn build(tree: &ValidatedSourceTree) -> SnapshotResult {
    SnapshotResult {
        repository_identity: tree.revision().to_hex(),
        tree_identity: tree.identity().to_hex(),
        source_units: crate::semantic::tree::source_units(tree),
        declarations: tree
            .declarations()
            .iter()
            .map(|declaration| crate::semantic::tree::declaration_record(tree, declaration))
            .collect(),
        nodes: crate::semantic::tree::node_records(tree),
    }
}
