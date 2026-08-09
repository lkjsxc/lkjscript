# lkjscript

`lkjscript` is an experimental, AI-primary, statically typed, memory-safe language and runtime
implemented in Rust. The project is in an architectural reset toward a typed semantic workspace,
scale-safe compilation, and one measured production execution path.

## Prerequisites

- a current stable Rust toolchain with Cargo, Rustfmt, and Clippy;
- the SQLite runtime library (`libsqlite3` on Linux);
- Linux x86-64 for the currently tested native path.

## Build and first run

```sh
cargo build --locked -p lkjscript-app --bin lkjscript
cargo run --locked -p lkjscript-app --bin lkjscript -- \
  run src/examples/hello/main.lkjscript
```

A successful run prints `3628800`. `run` has one product execution policy: it attempts one eligible
baseline-native group before effects, otherwise executes the unchanged validated program in the VM.
The VM has no JIT dependency or native-transition state. There is no engine, threshold, or forced
native CLI selection, and native entry is a commit point with no VM retry. The discarded
optimizing runtime and forced execution helpers are deleted. The SSA evaluator is available
only as the opt-in `lkjscript-ir/test-oracle` feature used by development and differential tests;
production dependencies do not enable it.

The active product scope is local package compile/run plus the compiler's in-process semantic
workspace library. Text and paths are importer conveniences, not program authority.
`Workspace::empty` creates a source-free incomplete program; revision-checked transactions can
create non-generic products, enums, functions, and `main`, then fill real typed holes with flat
non-recursive drafts. Implemented drafts include immutable lexical locals, selected canonical
built-in operations, byte-vector move/borrow, product/enum construction and observation, and
source-free exhaustive enum payload matches. Match arms use flat `PatternDraft` trees; named payload
bindings receive stable workspace entities while compiler-only scrutinee/projection storage remains
hidden. The complete immutable `WorkspaceSnapshot` derives compiler HIR directly, without rendering
or parsing source. Imported and programmatically constructed programs use the same canonical match
checker/lowering, semantic authority, query/index model, completeness gate, ownership checker, and
compiler boundary.

The `.lkjscript` extension is the only fixed source-format property. The current line-oriented
encoding is provisional and non-authoritative; no textual, binary, or compatibility promise follows
from it.

A minimal source-free authorship sequence is:

```rust
use lkjscript_compiler::{Edit, ParameterDraft, SemanticTypeRef, Transaction, Workspace};

let mut workspace = Workspace::empty().expect("empty semantic workspace");
let revision = workspace.current().revision();
let created = workspace.apply(Transaction {
    base_revision: revision,
    edits: vec![
        Edit::CreateFunction {
            name: "identity".into(),
            parameters: vec![ParameterDraft {
                name: "value".into(),
                ty: SemanticTypeRef::I64,
            }],
            return_type: SemanticTypeRef::I64,
        },
        Edit::CreateMain {
            return_type: SemanticTypeRef::I64,
        },
    ],
}).expect("atomic declaration creation");
assert!(!created.snapshot.completeness_blockers().is_empty());
```

The public transaction diff returns the created entity and body-hole identities; subsequent
`FillHole` transactions complete `identity(value) = value` and `main() = identity(42)`. The focused
compiler test executes that source-free snapshot through the production bytecode/VM route and
returns `42` with zero source-loading and parser invocations.

The workspace has one application binary, `lkjscript`; there is no semantic wire service pending a
measured consumer. Daemon, process-cell, session, scheduler, resource-topology, service-database,
and Linux-observation products are intentionally absent. The language's SQLite capability remains
implemented directly by the VM and `lkjscript-sys`, alongside stdio, clock, filesystem, network,
and terminal operations.

## Verification

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --workspace --all-targets --all-features --locked
cargo build --workspace --release --locked
```

The retained container verification builds only `--bin lkjscript` and runs hello, Mandelbrot, and
all seven local shell smoke/check scripts, including the direct SQLite path:

```sh
docker compose -f meta/docker-compose.yml --profile verify run --build --rm verify
```

## Documentation

- [Authority and conflict rules](docs/authority.md)
- [Normative language semantics](docs/spec/language.md)
- [Semantic workspace contract](docs/spec/workspace.md)
- [Current implementation status](docs/status.md)
- [Current architecture and target deltas](docs/architecture.md)
- [Performance method and baselines](docs/performance.md)
- [Roadmap](docs/roadmap.md)
- [Engineering policy](AGENTS.md)
