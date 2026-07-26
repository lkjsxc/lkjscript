mod model;
mod staging;

pub use model::{EditionMigrationChange, EditionMigrationPlan};

use lkjscript_core::ResourceProfile;

use crate::source::{
    api, DiagnosticCategory, RevisionId, SourceDiagnostic, SourceEdition, SourceResult, SourceSpan,
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
    let old_bytes = total_bytes(tree)?;
    if tree.edition() == SourceEdition::Edition2 {
        return Ok(EditionMigrationPlan {
            old_edition: SourceEdition::Edition2,
            new_edition: SourceEdition::Edition2,
            old_revision: tree.revision(),
            new_revision: tree.revision(),
            old_tree_identity: tree.identity(),
            new_tree_identity: tree.identity(),
            old_bytes,
            new_bytes: old_bytes,
            changes: Vec::new(),
        });
    }
    staging::reserve(tree, profile)?;
    let staged = stage_sources(tree);
    let migrated = api::rebuild_staged_sources(
        &staged,
        tree.root_path().to_path_buf(),
        tree.root_origin().clone(),
        limits,
    )?;
    if migrated.edition() != SourceEdition::Edition2 {
        return Err(diagnostic(
            tree,
            "LKJ-SRC-MIGRATION",
            "migration did not produce an Edition 2 closure",
        ));
    }
    let new_bytes = total_bytes(&migrated)?;
    let changes = build_changes(tree, &migrated, staged)?;
    Ok(EditionMigrationPlan {
        old_edition: SourceEdition::Edition1,
        new_edition: SourceEdition::Edition2,
        old_revision: tree.revision(),
        new_revision: migrated.revision(),
        old_tree_identity: tree.identity(),
        new_tree_identity: migrated.identity(),
        old_bytes,
        new_bytes,
        changes,
    })
}

fn stage_sources(
    tree: &ValidatedSourceTree,
) -> Vec<(std::path::PathBuf, crate::source::SourceOrigin, String)> {
    let mut staged = Vec::with_capacity(tree.files().len());
    for file in tree.files() {
        let (offset, inserted) = staging::insertion(file);
        let mut source = String::with_capacity(file.exact_source.len() + inserted.len());
        source.push_str(&file.exact_source[..offset]);
        source.push_str(inserted);
        source.push_str(&file.exact_source[offset..]);
        staged.push((file.path.clone(), file.origin.clone(), source));
    }
    staged
}

fn build_changes(
    tree: &ValidatedSourceTree,
    migrated: &ValidatedSourceTree,
    staged: Vec<(std::path::PathBuf, crate::source::SourceOrigin, String)>,
) -> SourceResult<Vec<EditionMigrationChange>> {
    let mut changes = Vec::with_capacity(staged.len());
    for ((_, origin, source), old) in staged.into_iter().zip(tree.files()) {
        let new = migrated
            .files()
            .iter()
            .find(|file| file.origin == origin)
            .ok_or_else(|| {
                diagnostic(
                    tree,
                    "LKJ-SRC-MIGRATION",
                    "migrated source identity is missing",
                )
            })?;
        let (offset, inserted) = staging::insertion(old);
        let insertion_byte = u64::try_from(offset).map_err(|_| {
            diagnostic(
                tree,
                "LKJ-SRC-MIGRATION-LIMIT",
                "migration insertion offset overflow",
            )
        })?;
        changes.push(EditionMigrationChange {
            path: origin.logical_path,
            insertion_byte,
            inserted_bytes: inserted.to_string(),
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

fn total_bytes(tree: &ValidatedSourceTree) -> SourceResult<u64> {
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

fn diagnostic(
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
