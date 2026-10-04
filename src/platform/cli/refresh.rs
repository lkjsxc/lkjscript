//! Explicit refresh of an original reviewed proposal onto one accepted revision.

use super::*;

pub(super) fn parse_onto(
    action: ChangeAction,
    options: &[String],
) -> Result<Option<RevisionId>, Diagnostic> {
    if action != ChangeAction::Refresh {
        return Ok(None);
    }
    required_option(options, "--onto")?
        .parse::<RevisionId>()
        .map(Some)
        .map_err(|diagnostic| direct_option_error("--onto", diagnostic))
}

pub(super) fn require_target(view: &RepositoryView, onto: RevisionId) -> Result<(), Diagnostic> {
    if view.revision() != onto {
        return Err(Diagnostic::new(
            DiagnosticClass::Semantic,
            "change_refresh_stale_target",
            format!(
                "refresh target {onto} is not the observed HEAD {}; no change was published",
                view.revision()
            ),
        ));
    }
    Ok(())
}

pub(super) fn execute(
    project: Option<PathBuf>,
    action: ChangeAction,
    request: ChangeCommandRequest,
) -> Result<Vec<u8>, Vec<Diagnostic>> {
    let ChangeCommandRequest {
        normalized,
        reviewed,
        input_file,
        output_file,
        onto,
    } = request;
    let reviewed = reviewed
        .ok_or_else(|| single_diagnostic(usage_error("refresh requires an exact reviewed plan")))?;
    let (onto, original) = match (action, reviewed.refresh, onto) {
        (ChangeAction::Refresh, None, Some(onto)) => (onto, reviewed),
        (ChangeAction::Apply, Some(refresh), None) => (
            refresh.onto,
            ChangePlanToken {
                request: reviewed.request,
                prepared: refresh.original_prepared,
                refresh: None,
            },
        ),
        _ => {
            return Err(single_diagnostic(usage_error(
                "refresh uses the original plan and --onto; apply uses its refreshed token",
            )));
        }
    };
    let repository = open_normalized_repository(project).map_err(single_diagnostic)?;
    let current = repository.view_current().map_err(single_diagnostic)?;
    // A key already accepted at another parent cannot authorize another refreshed candidate.
    let replay = normalized
        .options
        .idempotency_key
        .as_deref()
        .map(|key| repository.view_idempotency_base(key, onto))
        .transpose()
        .map_err(single_diagnostic)?
        .flatten();
    let target = match replay {
        Some(view) if action == ChangeAction::Apply => view,
        _ => {
            require_target(&current, onto).map_err(single_diagnostic)?;
            current
        }
    };
    let origin = repository
        .view_ancestor_from(
            &target,
            normalized.semantic.base,
            &ExecutionControl::uncancelled(),
        )
        .map_err(single_diagnostic)?;
    let mut source_owners = std::collections::BTreeMap::new();
    let mut original_prepared = origin
        .prepare_authored_change_with_source_owners(
            &normalized.semantic,
            normalized.options.clone(),
            Some(&mut source_owners),
        )
        .map_err(|mut errors| {
            for error in &mut errors {
                if !matches!(
                    error.class,
                    DiagnosticClass::Resource | DiagnosticClass::Cancelled
                ) {
                    normalized.origins.locate(error, &source_owners);
                }
            }
            errors
        })?;
    finish_authored_review(&mut original_prepared, &origin, &normalized)?;
    let origin_plan = LogicalChangePlan::new(normalized.request_commitment, &original_prepared)
        .map_err(single_diagnostic)?;
    let origin_encoding =
        encode_logical_change_plan(&origin_plan, |_| Ok(())).map_err(single_diagnostic)?;
    require_same_plan(original, origin_encoding.token, "original").map_err(single_diagnostic)?;

    let mut prepared = target.prepare_refreshed_authored_change(
        &normalized.semantic,
        normalized.options.clone(),
        &original_prepared,
        &normalized.native_reads,
    )?;
    finish_authored_review(&mut prepared, &target, &normalized)?;
    let logical_plan =
        LogicalChangePlan::refreshed(normalized.request_commitment, &prepared, original)
            .map_err(single_diagnostic)?;
    if action == ChangeAction::Refresh {
        let (encoding, plan_output) = match output_file.as_deref() {
            Some(path) => {
                let (encoding, output) =
                    write_logical_plan_output(repository.root(), Path::new(path), &logical_plan)
                        .map_err(single_diagnostic)?;
                (encoding, Some(output))
            }
            None => (
                encode_logical_change_plan(&logical_plan, |_| Ok(())).map_err(single_diagnostic)?,
                None,
            ),
        };
        return compact_change_response(
            &repository,
            &prepared,
            "prepared",
            encoding,
            input_file.as_deref(),
            plan_output.as_ref(),
            None,
        )
        .map_err(single_diagnostic);
    }
    let encoding =
        encode_logical_change_plan(&logical_plan, |_| Ok(())).map_err(single_diagnostic)?;
    require_same_plan(reviewed, encoding.token, "refreshed").map_err(single_diagnostic)?;
    apply_prepared_change(&repository, &prepared, encoding)
}

fn require_same_plan(
    reviewed: ChangePlanToken,
    reconstructed: ChangePlanToken,
    stage: &str,
) -> Result<(), Diagnostic> {
    if reviewed != reconstructed {
        return Err(Diagnostic::new(
            DiagnosticClass::Semantic,
            "change_prepared_plan_mismatch",
            format!(
                "{stage} reviewed plan does not match its complete reconstruction; no change was published"
            ),
        ));
    }
    Ok(())
}
