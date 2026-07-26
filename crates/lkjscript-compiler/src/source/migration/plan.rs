use lkjscript_core::BudgetLedger;

use crate::source::{SourceEdition, SourceResult, ValidatedSourceTree};

use super::{
    diagnostic, staging, EditionMigrationChange, EditionMigrationDeclarationIdentity,
    EditionMigrationNodeIdentity, EditionMigrationPlan,
};

pub(super) fn stage_sources(
    tree: &ValidatedSourceTree,
    conversions: &[staging::ConversionInsertion],
) -> Vec<(std::path::PathBuf, crate::source::SourceOrigin, String)> {
    tree.files()
        .iter()
        .enumerate()
        .map(|(index, file)| {
            (
                file.path.clone(),
                file.origin.clone(),
                staging::replacement(index, file, conversions),
            )
        })
        .collect()
}

pub(super) fn validate_complete_semantics(
    old: &ValidatedSourceTree,
    migrated: &ValidatedSourceTree,
    limits: &lkjscript_core::Limits,
    ledger: &mut BudgetLedger,
) -> SourceResult<()> {
    if migrated.edition() != SourceEdition::Edition2 {
        return Err(diagnostic(
            old,
            "LKJ-SRC-MIGRATION",
            "migration did not produce an Edition 2 closure",
        ));
    }
    let analyzed = crate::analyze::analyze_program(migrated)
        .map_err(|error| diagnostic(old, "LKJ-SRC-MIGRATION-SEMANTICS", error.to_string()))?;
    let ssa = crate::ssa::lower_program_with_budget(&analyzed, ledger)
        .map_err(|error| diagnostic(old, "LKJ-SRC-MIGRATION-SEMANTICS", error.to_string()))?;
    let (chunk, _) = crate::codegen::compile_program(&ssa)
        .map_err(|error| diagnostic(old, "LKJ-SRC-MIGRATION-SEMANTICS", error.to_string()))?;
    lkjscript_core::validate_chunk(chunk, &limits.validation)
        .map_err(|error| diagnostic(old, "LKJ-SRC-MIGRATION-SEMANTICS", error.to_string()))?;
    Ok(())
}

pub(super) fn build_changes(
    tree: &ValidatedSourceTree,
    migrated: &ValidatedSourceTree,
    staged: Vec<(std::path::PathBuf, crate::source::SourceOrigin, String)>,
    conversions: &[staging::ConversionInsertion],
) -> SourceResult<Vec<EditionMigrationChange>> {
    let mut changes = Vec::with_capacity(staged.len());
    for (index, ((_, origin, source), old)) in staged.into_iter().zip(tree.files()).enumerate() {
        let new = migrated
            .files()
            .iter()
            .find(|file| file.origin == origin)
            .ok_or_else(|| diagnostic(tree, "LKJ-SRC-MIGRATION", "migrated source is missing"))?;
        let (offset, inserted) = staging::insertion(old);
        changes.push(EditionMigrationChange {
            path: origin.logical_path,
            insertion_byte: u64::try_from(offset).map_err(|_| {
                diagnostic(tree, "LKJ-SRC-MIGRATION-LIMIT", "insertion offset overflow")
            })?,
            inserted_bytes: inserted.to_string(),
            conversion_count: u64::try_from(
                conversions.iter().filter(|site| site.file == index).count(),
            )
            .map_err(|_| diagnostic(tree, "LKJ-SRC-MIGRATION-LIMIT", "conversion overflow"))?,
            old_bytes: old.exact_source_len,
            new_bytes: new.exact_source_len,
            old_identity: old.identity,
            new_identity: new.identity,
            replacement_source: source,
        });
    }
    changes.sort_by(|left, right| left.path.cmp(&right.path));
    Ok(changes)
}

pub(super) fn idempotent(tree: &ValidatedSourceTree, bytes: u64) -> EditionMigrationPlan {
    EditionMigrationPlan {
        old_edition: SourceEdition::Edition2,
        new_edition: SourceEdition::Edition2,
        old_revision: tree.revision(),
        new_revision: tree.revision(),
        old_tree_identity: tree.identity(),
        new_tree_identity: tree.identity(),
        old_bytes: bytes,
        new_bytes: bytes,
        changes: Vec::new(),
        declarations: tree
            .declarations()
            .iter()
            .map(|item| EditionMigrationDeclarationIdentity {
                old_key: item.key().clone(),
                new_key: item.key().clone(),
                old_node: item.node(),
                new_node: item.node(),
            })
            .collect(),
        nodes: tree
            .nodes()
            .iter()
            .map(|item| EditionMigrationNodeIdentity {
                old: item.id(),
                new: item.id(),
            })
            .collect(),
    }
}

pub(super) fn total_bytes(tree: &ValidatedSourceTree) -> SourceResult<u64> {
    tree.files().iter().try_fold(0_u64, |total, file| {
        total.checked_add(file.exact_source_len).ok_or_else(|| {
            diagnostic(
                tree,
                "LKJ-SRC-MIGRATION-LIMIT",
                "migration source-byte total overflow",
            )
        })
    })
}
