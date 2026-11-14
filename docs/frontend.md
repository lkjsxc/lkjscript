# Source producer

The source producer accepts declared source/library bytes and returns checked
lkjscript IR. [src/frontend_lib.rs](../src/frontend_lib.rs) shares the production
loader, parser, resolver, elaborator, checker and writer. It has no filesystem or
native-emission entry. [src/frontend.rs](../src/frontend.rs) owns the pipeline.

## Pure API

```rust
pub fn source_to_ir(bundle: &SourceBundle) -> Result<String, Diagnostic>;
```

`SourceBundle` contains `root: String`, `library: Option<String>` and
`files: Vec<SourceInput>`. Each input has a logical `path: String` and exact
UTF-8 `text: String`. `frontend::check_bundle` returns checked IR in memory;
`frontend::check_source` checks an already parsed source program.

The loader resolves inline/file modules, chooses between `child.rs` and
`child/mod.rs`, and rejects missing, ambiguous or duplicate inputs.
Logical paths are exact lookup keys and diagnostic names. Preserve them when
comparing IR bytes, including relative source paths and `$std/...` names.

`library: None` supplies no source library. `Some(path)` selects a declared
library root and its supplied module files. The pure API and World driver do not
substitute library bytes for missing inputs.

## Native World driver

[src/frontend_main.rs](../src/frontend_main.rs) accepts a bundle or directory
capabilities:

```sh
./frontend.elf program.lkjsb > program.lkjir
./frontend.elf --check program.lkjsb
./frontend.elf --source src/main.rs --source-dir /work/project \
  --library-dir /work/library > program.lkjir
./frontend.elf --source small.rs --no-library --check
./frontend.elf --source src/main.rs --library-dir library --pack > program.lkjsb
```

Bundle paths open beneath the current-directory capability. Direct source mode
opens the chosen source directory (default `.`) and the selected library.
Both routes call the same production loader. Library paths use `$std/...`.

`--pack` records exact UTF-8 reads, including both ambiguous module candidates.
It caches missing-file results. Syntax/module rejection inputs can be packed;
I/O failures remain errors. `--check` and `--pack` are mutually exclusive.

Normal mode writes checked IR beginning with `lkjscript-ir 1`; check mode is
silent. Errors write one diagnostic and select status 1. Diagnostics retain
logical path, line, column and code. Bundle framing errors use E_BUNDLE;
file/UTF-8 read errors use E_IO. Compiler arguments must be UTF-8.

The installed [native workflow](native.md) supplies the directories explicitly,
prepares runtime IR, builds programs and decodes scalar results.

## Bundle format

[src/frontend_bundle.rs](../src/frontend_bundle.rs) defines LKJSB1.
All integer fields are unsigned 64-bit little-endian. Lengths count bytes.

```text
bundle  = magic string library u64 file{file_count}
magic   = 4c 4b 4a 53 42 31 0a             # ASCII "LKJSB1\n"
string  = u64 UTF8{byte_length}
library = 00 | 01 string
file    = string string                    # logical path, exact source text
```

The first string is the root path. The following library field is absent or
contains the library root. The next u64 counts files; file order is retained.
There is no padding or terminator. The decoder requires complete consumption,
valid UTF-8, valid tags, representable lengths and enough remaining bytes.
The loader checks duplicate and missing logical inputs.

Native `lkjscript pack` uses the production loader.
[tools/source-bundle.mjs](../tools/source-bundle.mjs) also packs explicitly listed
files for bootstrap work:

```sh
node tools/source-bundle.mjs --root app/main.rs \
  --file app/main.rs --file app/helper.rs --output target/program.lkjsb
```

The optional `--library PATH` selects a listed library root; it does not discover
or provide files. For distinct logical and physical names, `--inputs FILE.json`
accepts `{root, library, files: [{path, file}]}`; physical paths are relative to
the current directory. The codec API exposes
`encodeBundle({root, library, files: [{path, text}]})`.

## Self-build

[tools/frontend.mjs](../tools/frontend.mjs) compares source results and exact IR.
[tools/frontend-selfhost.mjs](../tools/frontend-selfhost.mjs) prepares inputs,
emits the native producer and compares its own-source IR and executable bytes.
Current fixed-point results and the next step are in [status](status.md).
