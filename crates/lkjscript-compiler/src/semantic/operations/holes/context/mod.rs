mod facts;

use crate::semantic::schema::*;
use crate::source::ValidatedSourceTree;
use facts::{constraints, expected_fact, identity};

pub(crate) fn build(
    tree: &ValidatedSourceTree,
    node: u32,
    profile: ResourceProfile,
) -> Result<HoleContextResult, ProtocolError> {
    let site = super::site::find(tree, node)?;
    let program = super::validate::completed_program(&site).ok();
    let scope = super::scope::entities(&site, program.as_ref());
    let (candidates, exploration, blockers) = if program.is_some() {
        super::candidates::enumerate(&site, &scope, profile)
    } else {
        let message =
            "no bounded completion typechecked; derived scope and ownership facts are unavailable";
        (
            Vec::new(),
            ExplorationRecord {
                supported: false,
                truncated: false,
                charged_category: "hole_candidates".into(),
                charged_count: 0,
                search_work: 0,
                omitted: Vec::new(),
                reason: Some(BoundedReason {
                    code: BlockerCode::CheckerRejected,
                    message: message.into(),
                }),
            },
            vec![ActionBlocker {
                code: BlockerCode::CheckerRejected,
                subject: "surrounding_program".into(),
                reason: message.into(),
            }],
        )
    };
    let expected_type = expected_fact(&site.expected);
    Ok(HoleContextResult {
        hole: identity(&site),
        goal: site.goal.clone(),
        containing_return_type: super::types::canonical(&site.return_type),
        expected_type,
        scope_entities: scope,
        constraints: constraints(&site, program.as_ref()),
        candidates,
        exploration,
        blockers,
    })
}
