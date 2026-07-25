use std::path::Path;

use crate::source::{SourceDiagnostic, SourceOrigin, SourceResult};

pub(crate) fn validate_logical_source_path(logical_path: &str) -> SourceResult<SourceOrigin> {
    let raw_origin = SourceOrigin {
        logical_path: logical_path.to_string(),
        host_containment_path: None,
    };
    let path = Path::new(logical_path);
    let mut pieces = Vec::new();
    let canonical_components = path.components().all(|component| {
        let std::path::Component::Normal(piece) = component else {
            return false;
        };
        let Some(piece) = piece.to_str() else {
            return false;
        };
        if piece.starts_with('.') {
            return false;
        }
        pieces.push(piece);
        true
    });
    if logical_path.is_empty()
        || logical_path.contains('\\')
        || path.is_absolute()
        || path.extension().and_then(|extension| extension.to_str())
            != Some(crate::SOURCE_EXTENSION)
        || !canonical_components
        || pieces.join("/") != logical_path
    {
        return Err(SourceDiagnostic::loading(
            raw_origin,
            format!(
                "logical source path must be a canonical relative non-dot .{} path: {logical_path}",
                crate::SOURCE_EXTENSION
            ),
        ));
    }
    Ok(SourceOrigin {
        logical_path: logical_path.to_string(),
        host_containment_path: None,
    })
}

pub(crate) fn canonical_logical_path(path: &Path) -> String {
    let mut pieces = Vec::new();
    for component in path.components() {
        match component {
            std::path::Component::Normal(piece) => {
                if let Some(piece) = piece.to_str() {
                    pieces.push(piece);
                }
            }
            std::path::Component::ParentDir => {
                pieces.pop();
            }
            std::path::Component::CurDir
            | std::path::Component::RootDir
            | std::path::Component::Prefix(_) => {}
        }
    }
    if pieces.is_empty() {
        "source.lkjscript".into()
    } else {
        pieces.join("/")
    }
}
