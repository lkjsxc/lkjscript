//! Canonical declaration names used by resolution, generic validation, and elaboration.
/// Join one module path and declaration spelling without changing either input.
pub fn qualified(module: &str, name: &str) -> String {
    let mut result = module.to_string();
    if !module.is_empty() {
        result.push_str("::");
    }
    result.push_str(name);
    result
}
