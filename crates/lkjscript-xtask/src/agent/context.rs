use std::path::Path;

use super::model::ResumeContext;

pub fn build(root: &Path, task_id: &str, profile: &str) -> Result<ResumeContext, String> {
    let state = super::storage::load(root, task_id)?.ok_or("task state does not exist")?;
    let head = super::git::head(root)?;
    let revision_mismatch = (head != state.current_repository_revision).then(|| {
        format!(
            "state repository revision {}, current HEAD {head}",
            state.current_repository_revision
        )
    });
    let mut omissions = Vec::new();
    if let Err(error) = super::validate::validate(root, &state, false) {
        omissions.push(format!("state reference or revision staleness: {error}"));
    }
    let contexts = crate::structure::agent_context(root, &state.selected_capsule_scope, profile)?;
    let mut response = ResumeContext {
        schema: "lkjscript.agent-resume-context",
        version: 1,
        state,
        repository_context: Vec::new(),
        revision_mismatch,
        omissions,
        truncated: false,
    };
    let base_bytes = serde_json::to_vec_pretty(&response)
        .map_err(|error| format!("serialize resume context: {error}"))?
        .len();
    let context_bytes = if profile == "weak" {
        super::bounds::CONTEXT_BYTES_WEAK
    } else {
        super::bounds::CONTEXT_BYTES_STRONG
    };
    let limit = base_bytes
        .checked_add(context_bytes)
        .ok_or("resume output limit overflow")?
        .min(super::bounds::OUTPUT_BYTES);
    for context in contexts {
        include(&mut response, context, profile, limit)?;
    }
    let output_bytes = serde_json::to_vec_pretty(&response)
        .map_err(|error| format!("serialize resume context: {error}"))?
        .len();
    if output_bytes > super::bounds::OUTPUT_BYTES {
        return Err("resume output exceeds aggregate output byte limit".into());
    }
    Ok(response)
}

pub(crate) fn include(
    response: &mut ResumeContext,
    context: crate::model::QueryResult,
    profile: &str,
    limit: usize,
) -> Result<(), String> {
    if context.truncated {
        response.truncated = true;
        response.omissions.push(format!(
            "repository context for {} was truncated",
            context.target
        ));
    }
    for unsupported in &context.unsupported {
        response
            .omissions
            .push(format!("{}: {unsupported}", context.target));
    }
    response.repository_context.push(context);
    let bytes = serde_json::to_vec_pretty(response)
        .map_err(|error| format!("serialize resume context: {error}"))?;
    if bytes.len() > limit {
        let Some(omitted) = response.repository_context.pop() else {
            return Err("repository context disappeared during output accounting".into());
        };
        response.truncated = true;
        response.omissions.push(format!(
            "repository context for {} omitted by {profile} output limit {limit}",
            omitted.target
        ));
    }
    Ok(())
}
