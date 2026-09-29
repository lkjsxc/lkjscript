//! Explicit, compile-time publication allowlist. No directory is served at runtime.
pub(crate) struct Document {
    pub slug: &'static str,
    pub path: &'static str,
    pub title: &'static str,
    pub summary: &'static str,
    pub markdown: &'static str,
}

macro_rules! document {
    ($slug:literal, $path:literal, $title:literal, $summary:literal) => {
        Document {
            slug: $slug,
            path: $path,
            title: $title,
            summary: $summary,
            markdown: include_str!(concat!("../../../", $path)),
        }
    };
}

pub(crate) static DOCUMENTS: &[Document] = &[
    document!(
        "start",
        "docs/guides/native-command.md",
        "Your first program",
        "Create, inspect, change and run a typed command application."
    ),
    document!(
        "web",
        "docs/guides/native-web.md",
        "Build for the web",
        "An editable web application, from one installed executable."
    ),
    document!(
        "http",
        "docs/guides/native-http.md",
        "HTTP applications",
        "Compose routes, requests and standalone service deployments."
    ),
    document!(
        "forms",
        "docs/guides/native-forms.md",
        "Typed form handling",
        "Decode browser input through ordinary language libraries."
    ),
    document!(
        "direction",
        "docs/direction.md",
        "Language direction",
        "The graph, types, ownership, effects and long-term design."
    ),
    document!(
        "architecture",
        "docs/architecture.md",
        "Architecture",
        "Where meaning, validation, storage and execution belong."
    ),
    document!(
        "status",
        "docs/status.md",
        "Development status",
        "Implemented capabilities, publication boundaries and remaining work."
    ),
    document!(
        "security",
        "docs/security.md",
        "Trust and security",
        "Understand grants, trusted-code execution and security nonclaims."
    ),
];

pub(crate) fn revision() -> &'static str {
    match option_env!("LKJSCRIPT_SITE_REVISION") {
        Some(value) if value.len() == 40 && value.bytes().all(|byte| byte.is_ascii_hexdigit()) => {
            value
        }
        _ => "unversioned",
    }
}

pub(crate) fn source_url(path: &str) -> String {
    let revision = revision();
    let reference = if revision == "unversioned" {
        "main"
    } else {
        revision
    };
    format!("https://github.com/lkjsxc/lkjscript/blob/{reference}/{path}")
}
