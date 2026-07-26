mod identities;
mod model;
mod plan;
mod publication;
mod staging;

pub use model::{
    EditionMigrationChange, EditionMigrationDeclarationIdentity, EditionMigrationNodeIdentity,
    EditionMigrationPlan,
};
pub use publication::{diff_edition2_migration, publish_edition2_migration};
#[cfg(test)]
pub(crate) use publication::{simulate_checked_crash, simulate_checked_rollback};

use lkjscript_core::ResourceProfile;

use crate::source::{
    DiagnosticCategory, RevisionId, SourceDiagnostic, SourceEdition, SourceResult, SourceSpan,
    ValidatedSourceTree,
};

pub fn check_edition2_migration(
    tree: &ValidatedSourceTree,
    expected_revision: RevisionId,
    limits: &lkjscript_core::Limits,
    profile: ResourceProfile,
) -> SourceResult<EditionMigrationPlan> {
    if tree.revision() != expected_revision {
        return Err(diagnostic(
            tree,
            "LKJ-SRC-STALE-MIGRATION",
            format!(
                "stale migration revision {expected_revision}; expected {}",
                tree.revision()
            ),
        ));
    }
    let old_bytes = plan::total_bytes(tree)?;
    if tree.edition() == SourceEdition::Edition2 {
        return Ok(plan::idempotent(tree, old_bytes));
    }
    let conversions = staging::resolved_conversions(tree)?;
    staging::reserve(tree, &conversions, profile)?;
    let staged = plan::stage_sources(tree, &conversions);
    let migrated = crate::source::rebuild_staged_sources(
        &staged,
        tree.root_path().to_path_buf(),
        tree.root_origin().clone(),
        limits,
    )?;
    plan::validate_complete_semantics(tree, &migrated, limits, profile)?;
    Ok(EditionMigrationPlan {
        old_edition: SourceEdition::Edition1,
        new_edition: SourceEdition::Edition2,
        old_revision: tree.revision(),
        new_revision: migrated.revision(),
        old_tree_identity: tree.identity(),
        new_tree_identity: migrated.identity(),
        old_bytes,
        new_bytes: plan::total_bytes(&migrated)?,
        changes: plan::build_changes(tree, &migrated, staged, &conversions)?,
        declarations: identities::declaration_relations(tree, &migrated)?,
        nodes: identities::node_relations(tree, &migrated)?,
    })
}

pub(super) fn diagnostic(
    tree: &ValidatedSourceTree,
    code: &'static str,
    message: impl Into<String>,
) -> SourceDiagnostic {
    SourceDiagnostic::new(
        code,
        DiagnosticCategory::SourceLoading,
        message,
        tree.root_origin().clone(),
        SourceSpan::zero(),
    )
}
