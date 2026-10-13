# Retained native repository-policy predecessor

Since the 2026-09-29 owner decision, the required `lkjscript-dev policy no-python`
gate makes extension and bounded raw-byte shebang decisions directly in Rust.
Production policy classification no longer loads or executes this bundle, and does
not serialize observations through JSON or Base64. Filesystem admission, ordering,
final-symlink nonfollowing, extension precedence and the 512-byte prefix remain.

This ordinary lkjscript program is retained as an independent executable regression
oracle and a language example, not as a runtime fallback. The Rust unit tests load
its unchanged derived bundle under `cfg(test)` and compare ASCII case variants,
single-byte mutations and the read boundary. The public-CLI reproduction tests still
exercise real native authorship and execution. The retained native functions inspect
Bytes through general `bytes-get`; these fixtures do not establish compiler self-hosting.

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

Compiler or artifact contract changes require rebuilding this retained artifact
through public check/build while preserving accepted HEAD and its exact standard
supplier. Rebuild the contributor test executable after replacing the artifact.
The [current release notes](../../docs/releases/v0.1.92.md) identify the selected
compatibility boundary; [status](../../docs/status.md) owns actual acceptance.

The preceding derived bundle used compiler 33, bytecode 27 and artifact 40:
`artifact_bundle_6efb7a414dc41054c09602618b9b7d2754ce78d53590ddc0846fc0e9eef2b7cf`,
758,094 bytes. Public check/build preserves the accepted native HEAD and its exact
supplier; all 62 native and supplier tests agree between the two evaluators.

The .96 compilation refresh selects artifact_bundle_80fc7c3a1aa35da4a96b3480dd55fc31e10ce70d1a93e472d072faa40ac1f958 (758,710 bytes, SHA-256 16b76270e8e38d8758e84e63adfb8c85a598f796f36b5850d83e0bd618e1bef3), using compiler 34, bytecode 28 and artifact 41. Its two public builds are byte-identical and all 62 source/supplier checks report zero failures and differential equality; the [separate rebuild receipt](../../.artifacts/associated-read-views/consumer-assets-installation-proposals/sourcef87c8b-four-families-first/receipt.json) binds their original source f87c8b7fa80e03bb8e3901746aa14a845fc509161916ca07eef92c55d32ed550. Accepted HEAD and exact suppliers remain unchanged. This compilation refresh supplies no final source, release or additional runtime acceptance.
