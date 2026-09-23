# Native repository-policy tool

The required `lkjscript-dev policy no-python` gate uses this ordinary lkjscript
program for extension and raw-byte shebang decisions. There is no Rust decision
fallback. The contributor executable embeds its generated bundle and calls the
same grant-free `PureTool` adapter as the native reference guides.

This is maintained native-tool adoption, not a self-hosted compiler or a native
filesystem/Git implementation. Rust observes paths, validates relative paths,
reads bounded regular-file prefixes, encodes the existing typed Bytes boundary,
reports decisions and returns exit codes. Native functions inspect Bytes directly
through general `bytes-get`; no production List<I64> expansion remains.

## Authority and reproduction

`project/` is accepted editable meaning. `requests/create.lkjc` retains original
integer-list authorship. `requests/20260924-bytes.lkjc` retains the actual canonical
edit and exact dependency replacement. `requests/create-bytes.lkjc` is the literal
fresh-authoring witness for the current program. None is a synchronized source mirror.
`generated/policy.lkja` is derived through ordinary public `build`, not a privileged
storage writer or external semantic generator. The selected standard now supplies
`bytes-get`; unchanged guide/application packages keep their own exact suppliers.

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
The current check runs 11 policy tests plus 51 exact standard tests, differentially.
The public CLI tests independently rebuild from copied accepted meaning and exercise
fresh creation, canonical editing and old/new detached runs with no host tools.

The Bytes program requires development source after public v0.1.44. That frozen
runtime lacks `core.bytes.get` and must reject the new closure, including when the
selected target does not itself use byte indexing. The earlier integer-list program
and its exact dependency remain executable on newer compatible runtimes.

## Typed boundary

| Pure Command target | One positional argument | Result |
| --- | --- | --- |
| `extensions` | List of Text extension observations | List of Bool |
| `shebangs` | List of Bytes raw prefixes | List of Bool |

An extension is the host's `Path::extension` observation without the dot; missing
is empty Text. Native code recognizes exactly `py`, `pY`, `Py`, and `PY`. This
has precedence even for a missing path, directory or symlink. A filename `.py`
alone has no extension on the supported Linux host. Path admission is separate.

For a non-Python extension the host observes at most 512 regular-file bytes,
skips a final symlink/non-file and preserves existing read errors. It passes typed
Bytes, never UTF-8 conversion or integer-list expansion. Native code requires `#!`
at offsets zero and one, then finds ASCII-case-insensitive `python` anywhere before
the first LF and within the first 512 bytes. CR, NUL and invalid UTF-8 do not become
a newline or undergo replacement. Native code also caps inspection at 512. The
original substring rule remains: this is not an interpreter-token parser or proof
that Python cannot be used.

The host calls batches of at most 64 observations, checks one Boolean per input,
and preserves original path order and the full violation count. The report shows
at most 64 violations. Preparation, execution, typed-input or output failure is an
infrastructure error, never an empty successful policy result.

```sh
lkjscript --project tools/native-policy/project run extensions --arguments '[["py","rs","PY"]]'
lkjscript --project tools/native-policy/project run shebangs --arguments '[[{"$bytes":"IyFweXRob24="}]]'
```

The second invocation returns `[true]` for raw `#!python`. Bytes use the existing
canonical padded Base64 JSON object, with exactly one `$bytes` field; integer arrays,
plain text, extra members and noncanonical Base64 reject. A normal bundle and grant-free
Command descriptor suffice after deleting the authoring project. There is no new
syntax, policy-specific intrinsic, ambient filesystem grant or application dependency.

## Verification and cost

An independent byte-window oracle remains only in host tests. It compares all 64
ASCII case combinations, all 1,536 one-byte mutations of `python`, raw-byte/first-line
cases and adjacent 512-byte boundaries. Filesystem tests retain extension precedence,
non-followed final links and multiple batches. The reusable embedding boundary retains
strict loading, cancellation/recovery, typed values, output limits and task rejection.

The complete exact standard closure and Base64 boundary have real costs. This is not
zero-copy I/O. Matched complete-command measurements, failures and source acceptance
belong to the [byte campaign](../../docs/campaigns/202609240603.md); the
[predecessor campaign](../../docs/campaigns/202609240414.md) retains the original costs.
