use std::path::{Path, PathBuf};
use std::time::Instant;

use crate::source::{SourceDiagnostic, SourceOrigin, SourceResult};

use super::LoadState;

pub(super) fn load_files_depth_first(
    entry: &Path,
    state: &mut LoadState<'_>,
) -> SourceResult<(PathBuf, SourceOrigin)> {
    let root = super::frame::load_frame(entry, true, None, &[], state)?.ok_or_else(|| {
        SourceDiagnostic::generic(
            SourceOrigin::in_memory("source.lkjscript"),
            "root source was already loaded",
        )
    })?;
    let root_path = root.canonical.clone();
    let root_origin = root.parsed.origin.clone();
    let mut stack = vec![root];
    loop {
        let next = stack.last_mut().and_then(|frame| {
            let pending = frame.imports.get(frame.next_import)?.clone();
            frame.next_import += 1;
            Some((pending, frame.parent.clone(), frame.parsed.origin.clone()))
        });
        if let Some((pending, parent, origin)) = next {
            let loading_started = Instant::now();
            let next_path = super::imports::resolve_import(
                &pending.spec,
                &parent,
                state.package_root,
                state.installed_root,
                &origin,
                pending.span,
            )?;
            state.metrics.source_loading = state
                .metrics
                .source_loading
                .saturating_add(loading_started.elapsed());
            if let Some(frame) = super::frame::load_frame(
                &next_path,
                false,
                Some((origin, pending.span)),
                &stack,
                state,
            )? {
                stack.push(frame);
            }
            continue;
        }

        let Some(frame) = stack.pop() else {
            break;
        };
        state.loading.remove(&frame.canonical);
        state.done.insert(frame.canonical);
        state.files.push(frame.parsed);
    }
    Ok((root_path, root_origin))
}
