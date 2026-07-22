//! Per-file nest, child, and token budget checks.

use crate::lex::Token;
use lkjscript_core::{Error, Limits, Result};

pub fn check_file_limits(tokens: &[Token], limits: &Limits, path: &str) -> Result<()> {
    let token_count = tokens.len() as u32;
    if token_count > limits.max_tokens_per_file {
        return Err(Error::msg(format!(
            "{path}: token budget exceeded ({token_count} > {}); split via import",
            limits.max_tokens_per_file
        )));
    }
    check_nest_tokens(tokens, limits, path)?;
    Ok(())
}

fn check_nest_tokens(tokens: &[Token], limits: &Limits, path: &str) -> Result<()> {
    let mut depth: u32 = 0;
    let mut child_stack: Vec<u32> = Vec::new();
    for tok in tokens {
        match tok {
            Token::Open(_) => {
                if let Some(c) = child_stack.last_mut() {
                    *c += 1;
                    if *c > limits.max_children {
                        return Err(Error::msg(format!(
                            "{path}: too many children (>{}); split args / extract helper",
                            limits.max_children
                        )));
                    }
                }
                depth += 1;
                if depth > limits.max_nest_depth {
                    return Err(Error::msg(format!(
                        "{path}: nest depth exceeded (>{}); extract a def",
                        limits.max_nest_depth
                    )));
                }
                child_stack.push(0);
            }
            Token::Close(_) => {
                child_stack.pop();
                depth = depth.saturating_sub(1);
            }
            Token::Atom(_) | Token::Str(_) => {
                if let Some(c) = child_stack.last_mut() {
                    *c += 1;
                    if *c > limits.max_children {
                        return Err(Error::msg(format!(
                            "{path}: too many children (>{}); split args / extract helper",
                            limits.max_children
                        )));
                    }
                }
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lex::lex;

    #[test]
    fn rejects_token_over_budget() {
        let lim = Limits {
            max_tokens_per_file: 2,
            ..Limits::default()
        };
        let tokens = lex("a\nb\nc\n").expect("lex");
        let err = check_file_limits(&tokens, &lim, "t.lkjml").expect_err("budget");
        assert!(err.as_str().contains("split via import"));
    }
}
