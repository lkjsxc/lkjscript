//! Rust-owned policy decisions over bounded, host-observed raw bytes.
//! The predecessor language bundle is used only by differential tests.

pub(super) fn extensions(values: &[&str]) -> Vec<bool> {
    values
        .iter()
        .map(|value| value.eq_ignore_ascii_case("py"))
        .collect()
}

pub(super) fn shebangs(values: &[Vec<u8>]) -> Vec<bool> {
    values.iter().map(|value| python_shebang(value)).collect()
}

fn python_shebang(prefix: &[u8]) -> bool {
    let observed = &prefix[..prefix.len().min(super::MAXIMUM_SHEBANG_BYTES as usize)];
    if !observed.starts_with(b"#!") {
        return false;
    }
    let end = observed
        .iter()
        .position(|byte| *byte == b'\n')
        .unwrap_or(observed.len());
    observed[..end]
        .windows(b"python".len())
        .any(|word| word.eq_ignore_ascii_case(b"python"))
}

#[cfg(test)]
mod tests;
