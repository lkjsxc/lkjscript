// The live proof must not accept a shortened, repeated or non-search query list.
pub fn validate(paths: &[String]) -> Result<(), String> {
    if paths.len() != 51usize {
        return Err(format!("live site parity requires 51 search query paths in paths.txt, found {}", paths.len()));
    }
    let mut seen = std::collections::BTreeSet::<String>::new();
    for path in paths {
        if !path.starts_with("/search?") {
            return Err(format!("paths.txt contains a non-search query target: {path}"));
        }
        if !seen.insert(path.clone()) {
            return Err(format!("paths.txt repeats search query target: {path}"));
        }
    }
    Ok(())
}
