# Native repository-policy tool

The required `lkjscript-dev policy no-python` gate uses this ordinary lkjscript
program for its extension and shebang decisions. There is no Rust decision
fallback. The contributor executable embeds the generated bundle and calls the
same grant-free `PureTool` adapter used by the native reference guides.

This is maintained native-tool adoption, not a self-hosted compiler or a native
filesystem/Git implementation. Rust observes repository paths, validates relative
paths, reads bounded regular-file prefixes, reports results and returns exit codes.
The gate still runs under its existing name, scope and verification owner.

## Authority and reproduction

`project/` is accepted editable meaning. `requests/create.lkjc` preserves literal
initial authorship; it is not synchronized source authority. The exact standard
supplier is unchanged. `generated/policy.lkja` is derived through ordinary public
`build`, not a privileged storage writer or an external semantic generator.

```sh
lkjscript --project tools/native-policy/project check
lkjscript --project tools/native-policy/project build --output rebuilt-policy.lkja
lkjscript --project tools/native-policy/project query find module policy
lkjscript --project tools/native-policy/project change draft --owner MODULE --output edit.lkjc
lkjscript --project tools/native-policy/project change plan --input-file edit.lkjc
```

Use the observed module identity for `MODULE`. An untouched draft is unchanged.
After editing, review a new plan, apply its exact token and check the accepted
program. Replace the derived bundle with the new public build before rebuilding
the contributor executable. Never edit storage or compiled bytes directly.
The current check runs 11 policy tests plus 45 exact standard tests, differentially.
The public CLI tests independently rebuild the maintained bundle from a copied
project and exercise fresh creation, canonical editing and old/new detached runs.

## Typed boundary

The ordinary pure Command targets are:

| Target | One positional argument | Result |
| --- | --- | --- |
| `extensions` | List of Text extension observations | List of Bool |
| `shebangs` | List of raw-byte prefixes, each represented as List of I64 | List of Bool |

An extension is the host platform's `Path::extension` observation, without the
dot; a missing extension is empty Text. Native code recognizes exactly `py`,
`pY`, `Py`, and `PY`. This decision has precedence even for a missing path,
directory or symlink. A filename `.py` alone has no extension on the supported
Linux host. Host path admission remains separate from this lexical policy.

For a non-Python extension, the host observes at most 512 bytes of a regular
file, skips a final symlink/non-file and preserves existing read errors. It passes
bytes as integer values, without UTF-8 conversion. Native code requires `#!`
at positions zero and one, then finds ASCII-case-insensitive `python` anywhere
before the first LF and within the first 512 bytes. CR, NUL and invalid UTF-8 do
not become a newline or cause lossy replacement. The native function also caps
its own inspection at 512. This intentionally preserves the original substring
rule: it is not an interpreter-token parser or proof that Python cannot be used.

The host calls batches of at most 64 observations, checks one Boolean per input,
and preserves original path order and the complete violation count. The existing
report shows at most 64 violations. Preparation, execution, typed input or output
failure is an infrastructure error, never an empty successful policy result.

For an independent binary-only invocation:

```sh
lkjscript --project tools/native-policy/project run extensions --arguments '[["py","rs","PY"]]'
lkjscript --project tools/native-policy/project run shebangs --arguments '[[[35,33,112,121,116,104,111,110]]]'
```

A normal artifact and grant-free Command descriptor suffice after deleting the
authoring project. The tool uses existing functions and collections; it adds no
intrinsic, language syntax, ambient filesystem grant or application dependency.

## Verification and cost

The maintained host tests retain an independent byte-window oracle only in test
code. They compare all 64 ASCII case combinations, all 1,536 one-byte mutations
of `python`, raw-byte/first-line cases and adjacent 512-byte boundaries. Filesystem
tests cover extension precedence, non-followed final links and several batches.
The reusable embedding boundary is separately checked for strict loading,
cancellation/recovery, typed values, output limits and rejection of tasks even
with an empty effect row or unused component requirements.

The bundle includes its complete exact standard closure. Its size and the native
VM cost are real overhead, not a speedup claim. Matched complete-policy timings
and final source acceptance belong to the [campaign](../../docs/campaigns/202609240414.md).
