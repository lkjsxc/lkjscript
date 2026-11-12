# Reduced ownership counterexample

`verdict-601-c1.rs` is a statement-deletion reduction of verdict generator seed
601, case 1. rustc accepts it; lkjscript rejects the later mutable borrow with
`E_BORROW`. The earlier shared reference is stored in a `View` local whose
reference is never used again and whose type has no destructor. This is a
remaining holder-liveness restriction. It is not evidence of unsound acceptance.

Two trailing spaces were removed from the retained source to pass the repository
whitespace check. The original reduction observation remains unchanged in
`verdict-601-c1.rs.jsonl`; its `retained` field records the normalized source hash
and bytes, and a repeated evaluator run confirms the same failure and rustc
acceptance. Diagnostic line numbers are unchanged. The record names
the frozen compiler binary, host runtime and reducer hashes, toolchains, flags,
input and output hashes, and both observations. The compiler was built from a
dirty checkout based on the recorded commit; its binary hash identifies the
actual tested artifact.

The original program returns 12 under rustc. Statement deletion removes the
vector insertions, so the reduced program traps with `R0003` before reaching
the conflicting borrow. This is permitted by the selected failure category:
`rejects-valid:E_BORROW` preserves rustc acceptance and lkjscript's ownership
rejection, not normal execution or the original value. The reducer separately
preserves exact categories for value, trap, and output mismatches.

Reproduce the original generated case, then reduce it:

```sh
node tools/generators/verdict.mjs target/debug/lkjscript 601 2 target/generators
node tools/generators/reduce.mjs target/debug/lkjscript \
  target/generators/verdict-601/c1.rs target/reduced/verdict-601-c1.rs \
  --engine node --entry probe --category rejects-valid:E_BORROW --timeout 2000
```

The reducer removes balanced statements or items, trying larger deletions first,
and rechecks every candidate with rustc and the selected lkjscript engine. It
stops when no remaining candidate deletion preserves the category and Rust
validity assumption. This is a local minimum for those deletion spans. It does
not claim globally minimal source. `--profile value` also compares trap source
lines; `--profile output --entry main` uses the World shim and compares raw bytes
and the manner of ending. `--engine k` selects the independent K machine.

`--timeout` is an operational timeout for each child process, not a language
execution limit. The shared process runner kills the whole child process group.
Every accepted reduction is saved before the next candidate runs. Runtime edits
abort a run so a single record cannot mix different host implementations.

Run the reducer's focused checks with:

```sh
node --test tools/generators/reduce.test.mjs
```
