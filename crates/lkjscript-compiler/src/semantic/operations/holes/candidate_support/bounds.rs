use crate::semantic::schema::*;

pub(crate) fn unsupported(
    reason: TypeUnavailableReason,
) -> (Vec<HoleCandidate>, ExplorationRecord, Vec<ActionBlocker>) {
    let message = format!("expected instantiated type unavailable: {reason:?}");
    (
        Vec::new(),
        unsupported_exploration(BoundedReason {
            code: BlockerCode::ExpectedTypeUnavailable,
            message: message.clone(),
        }),
        vec![ActionBlocker {
            code: BlockerCode::ExpectedTypeUnavailable,
            subject: "expected_type".into(),
            reason: message,
        }],
    )
}

pub(crate) fn bounded_failure(
    message: String,
) -> (Vec<HoleCandidate>, ExplorationRecord, Vec<ActionBlocker>) {
    (
        Vec::new(),
        ExplorationRecord {
            supported: false,
            truncated: true,
            charged_category: "hole_candidates".into(),
            charged_count: 0,
            search_work: 0,
            omitted: Vec::new(),
            reason: Some(BoundedReason {
                code: BlockerCode::CandidateBudgetExhausted,
                message: message.clone(),
            }),
        },
        vec![ActionBlocker {
            code: BlockerCode::CandidateBudgetExhausted,
            subject: "candidate_exploration".into(),
            reason: message,
        }],
    )
}

pub(crate) fn omitted_categories(
    rejected: std::collections::BTreeMap<CandidateCategory, u64>,
) -> Vec<OmittedCategory> {
    let mut result = Vec::new();
    result.extend(
        rejected
            .into_iter()
            .map(|(category, count)| OmittedCategory {
                category,
                known_count: Some(count),
                blocker: BlockerCode::CheckerRejected,
            }),
    );
    result
}

pub(crate) fn unsupported_blockers() -> Vec<ActionBlocker> {
    vec![ActionBlocker {
        code: BlockerCode::QualificationUnsupported,
        subject: "imports_and_qualification".into(),
        reason: "no qualified reference form is available".into(),
    }]
}
