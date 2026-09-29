//! Compile-time input projection of the site's publication catalog.
//! No filesystem scan, duplicated path list or HTTP/runtime dependency.
macro_rules! documents {
    ($(($slug:literal, $path:literal, $title:literal, $summary:literal)),* $(,)?) => {
        pub(super) const PATHS: &[&str] = &[$($path),*];
    };
}

include!("../../../../lkjscript-site/src/documents.rs");

pub(super) fn embeds(path: &str) -> bool {
    PATHS.contains(&path)
}
