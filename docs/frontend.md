# Source producer

The source producer accepts an explicit source bundle and returns checked K
text. [src/frontend_lib.rs](../src/frontend_lib.rs) is the pure consumer root:
it shares the production parser, loader, resolver, elaborator, K checker and K
writer with the complete compiler. It has no filesystem or native-emission
entry point. [src/frontend.rs](../src/frontend.rs) owns the shared pipeline.

## Pure API

The root re-exports this operation:

```rust
pub fn source_to_k(bundle: &SourceBundle) -> Result<String, Diagnostic>;
```

`SourceBundle`, defined in [src/source.rs](../src/source.rs), contains
`root: String`, `library: Option<String>` and `files: Vec<SourceInput>`.
Each input has a logical `path: String` and exact UTF-8 `text: String`.
`frontend::check_bundle` returns checked K without serialization;
`frontend::check_source` checks an already parsed `Program`.

The loader alone interprets inline and file modules, chooses between
`child.rs` and `child/mod.rs`, and reports missing, ambiguous or duplicate
inputs. Logical paths are exact lookup keys and diagnostic names; the bundle
codec does not canonicalize them or inspect the host filesystem. Preserve the
same logical paths when comparing K bytes, including relative source paths and
`$std/...` names used by bootstrap observations.

`library: None` supplies no source library. `Some(path)` selects an explicitly
supplied library root and its declared module files. Neither the pure API nor
the World driver substitutes embedded library bytes for missing inputs.

## Native World driver

[src/frontend_main.rs](../src/frontend_main.rs) declares the production modules
and a World entry. It accepts either an explicit bundle or directory capabilities:

```sh
./frontend.elf program.lkjsb > program.k
./frontend.elf --check program.lkjsb
./frontend.elf --source src/main.rs --source-dir /work/project \
  --library-dir /work/library > program.k
./frontend.elf --source small.rs --no-library --check
./frontend.elf --source src/main.rs --library-dir library --pack > program.lkjsb
```

Bundle paths are opened beneath the current-directory capability. Stored paths
are logical names. Direct source mode opens the explicitly selected source
directory (default `.`), plus the explicit library directory or no library.
It calls the same `source::load_report` module loader, then checks the already
parsed program through `frontend::check_source`. There is no second parse or
host-side module resolver. Library inputs use stable `$std/...` logical paths.

`--pack` encodes the exact UTF-8 reads made by the loader, including both
ambiguous module candidates. Missing-file results are cached too. Syntax/module
rejection inputs can be packed; I/O errors are reported instead of being
misrepresented as missing bytes. `--check` and `--pack` are mutually exclusive.

On success normal mode writes exactly checked K with no added newline; check
mode is silent. Errors write one diagnostic and select status 1. Source errors
retain logical path, line, column and code. Bundle framing errors use
`BUNDLE:1:1: E_BUNDLE ...`; file/UTF-8 read errors use `E_IO`. Compiler arguments
must be UTF-8. Stream failures keep ordinary World trap behavior.

The installed [native workflow](native.md) supplies source, library and runtime
paths explicitly, publishes outputs atomically, and adds standalone run and
scalar evaluation commands. No Node input-packing step is required.

## LKJSB1 wire format

[src/frontend_bundle.rs](../src/frontend_bundle.rs) decodes this grammar. All
integer fields are unsigned, 64-bit, little-endian values. Lengths count bytes,
not Unicode characters. There is no padding, terminator or implicit field.

```text
bundle  = magic string library u64 file{file_count}
magic   = 4c 4b 4a 53 42 31 0a             # ASCII "LKJSB1\n"
string  = u64 UTF8{byte_length}
library = 00 | 01 string
file    = string string                    # logical path, exact source text
```

The first string is the root path. The `u64` following `library` is
`file_count`; each string's `u64` is its `byte_length`. Library tag zero
means absent; tag one is followed by its root path. File order is retained.

The decoder requires valid UTF-8 in every string and complete consumption of
the input. It rejects unknown headers/tags, truncated fields, lengths outside
the address space or remaining bytes, an impossible file count, and trailing
bytes. It validates lengths before slicing or allocating. The structural count
check requires enough bytes for two length fields per file; it is not a source
quota. Duplicate and missing logical inputs are production loader diagnostics.
BOMs, line endings and other valid UTF-8 bytes are preserved. Execution resource
controls remain separate, as described in [limits.md](limits.md).

## Packing declared inputs

Native `lkj pack` uses the loader directly, as described above.
[tools/source-bundle.mjs](../tools/source-bundle.mjs) remains a host-side packer
for explicit bootstrap observations. It
reads only listed physical files; module selection remains the loader's job.
Run commands from the directory against which physical input paths should be
resolved. With identical physical and logical paths and no source library:

```sh
node tools/source-bundle.mjs --root app/main.rs \
  --file app/main.rs --file app/helper.rs \
  --output target/program.lkjsb
```

For different logical and physical names, use an explicit JSON specification.
This example supplies one source file and all current bootstrap library files:

```json
{
  "root": "app/main.rs",
  "library": "$std/lib.rs",
  "files": [
    {"path": "app/main.rs", "file": "app/main.rs"},
    {"path": "$std/lib.rs", "file": "library/lib.rs"},
    {"path": "$std/prelude.rs", "file": "library/prelude.rs"},
    {"path": "$std/sequence.rs", "file": "library/sequence.rs"},
    {"path": "$std/bytes.rs", "file": "library/bytes.rs"},
    {"path": "$std/string.rs", "file": "library/string.rs"},
    {"path": "$std/str.rs", "file": "library/str.rs"},
    {"path": "$std/char.rs", "file": "library/char.rs"},
    {"path": "$std/fmt.rs", "file": "library/fmt.rs"},
    {"path": "$std/iter.rs", "file": "library/iter.rs"},
    {"path": "$std/integer.rs", "file": "library/integer.rs"},
    {"path": "$std/io.rs", "file": "library/io.rs"},
    {"path": "$std/ordered_map.rs", "file": "library/ordered_map.rs"},
    {"path": "$std/ordered_set.rs", "file": "library/ordered_set.rs"}
  ]
}
```

```sh
node tools/source-bundle.mjs --inputs program-inputs.json \
  --output target/program.lkjsb --manifest target/program-inputs-record.json
```

The specification's physical paths are relative to the command's current
directory, not the JSON file. Add every source module the program needs. The
`--root`/`--file` form also accepts an explicit `--library PATH`; it does not
supply that file or any other library file automatically. Use fresh output and
record paths: exclusive creation rejects existing files, symlinks and hardlink
aliases. A failed write removes only artifacts created by that invocation. The
default record path is the bundle path plus `.json`.

The record contains `format`, `root`, `library`, the physical-path base
`base`, and an ordered `files` array. Each file records logical `path`,
physical `file`, byte count and SHA256 of the original bytes. `bundle`
records output path, byte count and SHA256. Invalid UTF-8 inputs are rejected
before output. The hashes describe the exact inputs and wire bytes; they do not
assert successful compilation.

For other tools, `encodeBundle({root, library, files: [{path, text}]})` returns
a Buffer. Async `packFiles({root, library, files: [{path, file}]})` returns
`{bytes, manifest}`; its record omits an output path until a caller chooses one.

## Bootstrap observations

The boundary implementation and codec tests are separate from native self-build
evidence. [tools/frontend.mjs](../tools/frontend.mjs) records B0 source results
and compares exact K, including a `compare --world` route using bundles.
[tools/frontend-selfhost.mjs](../tools/frontend-selfhost.mjs) separates input
preparation, native emission, and own-source/fixed-point runs. Preparation alone
executes no compiler. Actual bootstrap results, controls, hashes and outstanding
work belong in [status.md](status.md) and its linked observations. This document
specifies the interface and does not establish a native self-build milestone.
