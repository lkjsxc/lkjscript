//! Toolchain sources are embedded by the same checked source loader.
pub fn library(path: &str) -> Option<Vec<u8>> {
    let text = match path {
        "$std/lib.rs" => include_str!("../library/lib.rs"),
        "$std/prelude.rs" => include_str!("../library/prelude.rs"),
        "$std/sequence.rs" => include_str!("../library/sequence.rs"),
        "$std/bytes.rs" => include_str!("../library/bytes.rs"),
        "$std/string.rs" => include_str!("../library/string.rs"),
        "$std/str.rs" => include_str!("../library/str.rs"),
        "$std/char.rs" => include_str!("../library/char.rs"),
        "$std/fmt.rs" => include_str!("../library/fmt.rs"),
        "$std/iter.rs" => include_str!("../library/iter.rs"),
        "$std/integer.rs" => include_str!("../library/integer.rs"),
        "$std/io.rs" => include_str!("../library/io.rs"),
        "$std/ordered_map.rs" => include_str!("../library/ordered_map.rs"),
        "$std/ordered_set.rs" => include_str!("../library/ordered_set.rs"),
        _ => return None,
    };
    Some(text.as_bytes().to_vec())
}
pub fn runtime() -> Option<String> {
    Some(include_str!("../library/runtime.rs").to_string())
}
