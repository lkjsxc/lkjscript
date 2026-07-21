//! Language-spec source budgets. Numbers are fixed for this language version.

/// Maximum XML nest depth in one file.
pub const MAX_NEST_DEPTH: u32 = 8;
/// Maximum children under one element.
pub const MAX_CHILDREN: u32 = 8;
/// Maximum lexer tokens per source file.
pub const MAX_TOKENS_PER_FILE: u32 = 256;
/// Maximum visible children in one directory (`check-tree`).
pub const MAX_DIR_CHILDREN: u32 = 8;
/// Maximum top-level `def` / `do` / `import` forms per file.
pub const MAX_TOPLEVEL_FORMS: u32 = 8;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Limits {
    pub max_nest_depth: u32,
    pub max_children: u32,
    pub max_tokens_per_file: u32,
    pub max_dir_children: u32,
    pub max_toplevel_forms: u32,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            max_nest_depth: MAX_NEST_DEPTH,
            max_children: MAX_CHILDREN,
            max_tokens_per_file: MAX_TOKENS_PER_FILE,
            max_dir_children: MAX_DIR_CHILDREN,
            max_toplevel_forms: MAX_TOPLEVEL_FORMS,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_match_spec_consts() {
        let lim = Limits::default();
        assert_eq!(lim.max_nest_depth, MAX_NEST_DEPTH);
        assert_eq!(lim.max_toplevel_forms, MAX_TOPLEVEL_FORMS);
    }
}
