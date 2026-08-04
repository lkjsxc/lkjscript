pub(super) fn graph_identity(
    revision: &str,
    nodes: &[crate::model::Node],
    edges: &[crate::model::Edge],
    work: u64,
    charged_bytes: u64,
) -> String {
    let mut bytes = Vec::new();
    append(&mut bytes, revision);
    append(
        &mut bytes,
        &lkjscript_contracts::REPOSITORY_GRAPH_DIGEST.to_hex(),
    );
    bytes.extend_from_slice(&work.to_be_bytes());
    bytes.extend_from_slice(&charged_bytes.to_be_bytes());
    for item in nodes {
        for value in [
            &item.id,
            &item.kind,
            &item.label,
            &item.provenance,
            &item.authority,
            item.span.as_deref().unwrap_or(""),
            &item.confidence,
        ] {
            append(&mut bytes, value);
        }
    }
    for item in edges {
        for value in [
            &item.from,
            &item.to,
            &item.kind,
            &item.evidence,
            &item.confidence,
        ] {
            append(&mut bytes, value);
        }
    }
    crate::sha256::digest(&bytes)
}

fn append(bytes: &mut Vec<u8>, value: &str) {
    bytes.extend_from_slice(&(value.len() as u128).to_be_bytes());
    bytes.extend_from_slice(value.as_bytes());
}
