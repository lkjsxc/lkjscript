//! Read-only recent publication history; no replay, revalidation or rollback.
use super::{
    append_compact_record, compact_response_writer, ensure_options, extract_global_project,
    open_normalized_repository, option_value, usage_error,
};
use crate::platform::diagnostic::{Diagnostic, DiagnosticClass};
use crate::platform::publication::{
    DEFAULT_HISTORY_ITEMS, MAXIMUM_HISTORY_ITEMS, PublicationStatus,
};

pub(super) fn execute(arguments: Vec<String>) -> Result<Vec<u8>, Diagnostic> {
    let (arguments, project) = extract_global_project(arguments)?;
    ensure_options(&arguments[2..], &["--limit"], &[])?;
    let limit = match option_value(&arguments[2..], "--limit")? {
        None => DEFAULT_HISTORY_ITEMS,
        Some(value) => {
            if value.is_empty() || !value.bytes().all(|byte| byte.is_ascii_digit()) {
                return Err(usage_error(
                    "history --limit requires an unsigned decimal integer",
                ));
            }
            value
                .parse::<usize>()
                .map_err(|_| usage_error("history --limit is too large"))?
        }
    };
    if !(1..=MAXIMUM_HISTORY_ITEMS).contains(&limit) {
        return Err(Diagnostic::new(
            DiagnosticClass::Resource,
            "publication_history_limit",
            format!("history limit must be 1 through {MAXIMUM_HISTORY_ITEMS}"),
        ));
    }
    let repository = open_normalized_repository(project)?;
    let history = repository.history(limit)?;
    let mut output = compact_response_writer()?;
    append_compact_record(
        &mut output,
        "result",
        &[
            ("status", "success".to_owned()),
            ("command", "inspect.history".to_owned()),
        ],
    )?;
    append_compact_record(
        &mut output,
        "history",
        &[
            ("contract", "lkjscript-recent-history-1".to_owned()),
            ("repository", history.head.repository_id.to_string()),
            ("observed", history.head.revision.to_string()),
            ("record", history.head.record.to_string()),
            ("ordering", "newest-first".to_owned()),
            ("items", history.entries.len().to_string()),
            ("limit", limit.to_string()),
            ("truncated", history.next.is_some().to_string()),
            (
                "next-revision",
                history
                    .next
                    .map_or_else(|| "none".to_owned(), |p| p.revision.to_string()),
            ),
            (
                "next-record",
                history
                    .next
                    .map_or_else(|| "none".to_owned(), |p| p.record.to_string()),
            ),
            ("evidence", "recorded-acceptance".to_owned()),
            ("current-validation", "not-run".to_owned()),
            (
                "read-work-scope",
                "record-receipt-traversal-excludes-open".to_owned(),
            ),
            ("objects-read", history.work.objects_read.to_string()),
            ("bytes-read", history.work.bytes_read.to_string()),
        ],
    )?;
    for (ordinal, entry) in history.entries.iter().enumerate() {
        let receipt = &entry.receipt;
        let parent = entry.revision.publication.parents.first();
        append_compact_record(
            &mut output,
            "history.entry",
            &[
                ("ordinal", ordinal.to_string()),
                ("revision", entry.revision.revision.to_string()),
                ("record", entry.record.to_string()),
                (
                    "parent",
                    parent.map_or_else(|| "none".to_owned(), |p| p.revision.to_string()),
                ),
                (
                    "parent-record",
                    parent.map_or_else(|| "none".to_owned(), |p| p.record.to_string()),
                ),
                (
                    "status",
                    match receipt.status {
                        PublicationStatus::AcceptedChange => "accepted-change",
                        PublicationStatus::ProjectCreated => "project-created",
                        PublicationStatus::MergeAccepted => "merge-accepted",
                    }
                    .to_owned(),
                ),
                ("intent-present", receipt.intent.is_some().to_string()),
                ("intent", receipt.intent.clone().unwrap_or_default()),
                ("owners-created", receipt.counts.owners_created.to_string()),
                ("owners-updated", receipt.counts.owners_updated.to_string()),
                ("owners-deleted", receipt.counts.owners_deleted.to_string()),
                (
                    "dependencies-changed",
                    receipt.counts.dependencies_changed.to_string(),
                ),
                (
                    "tests-selected",
                    receipt.validation.tests_selected.to_string(),
                ),
                (
                    "tests-executed",
                    receipt.validation.tests_executed.to_string(),
                ),
                ("tests-passed", receipt.validation.tests_passed.to_string()),
            ],
        )?;
    }
    Ok(output.finish())
}
