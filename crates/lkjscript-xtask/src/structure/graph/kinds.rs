pub fn provenance(class: &str) -> &'static str {
    if class.starts_with("immutable-") {
        "immutable-evidence"
    } else if class.starts_with("generated-") {
        "generated"
    } else {
        "authored"
    }
}

pub fn file(path: &str, class: &str) -> &'static str {
    if class.starts_with("immutable-") {
        "evidence-file"
    } else if class.starts_with("generated-") {
        "generated-file"
    } else if path.starts_with("docs/decisions/") {
        "decision"
    } else if path.starts_with("docs/current-state") {
        "current-state"
    } else if path.ends_with(".schema.json") {
        "schema"
    } else if path.starts_with("docs/vision/experiments/") {
        "experiment"
    } else {
        "authored-file"
    }
}
