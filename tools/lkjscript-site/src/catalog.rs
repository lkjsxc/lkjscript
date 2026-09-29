//! Explicit, compile-time publication allowlist. No directory is served at runtime.
pub(crate) struct Document {
    pub slug: &'static str,
    pub path: &'static str,
    pub title: &'static str,
    pub summary: &'static str,
    pub markdown: &'static str,
}

macro_rules! documents {
    ($(($slug:literal, $path:literal, $title:literal, $summary:literal)),* $(,)?) => {
        pub(crate) static DOCUMENTS: &[Document] = &[
            $(Document {
                slug: $slug,
                path: $path,
                title: $title,
                summary: $summary,
                markdown: include_str!(concat!("../../../", $path)),
            }),*
        ];
    };
}

include!("documents.rs");

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
