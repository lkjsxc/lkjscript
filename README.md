# lkjscript

`lkjscript` is an experimental, AI-primary, statically typed, memory-safe language and runtime
implemented in Rust. The project is in an architectural reset toward a typed semantic workspace,
scale-safe compilation, and one measured production execution path.

## Prerequisites

- a current stable Rust toolchain with Cargo, Rustfmt, and Clippy;
- the SQLite runtime library (`libsqlite3` on Linux);
- Linux x86-64 for the currently tested native path.

## Build, check, and run

```sh
cargo build --locked -p lkjscript-app --bin lkjscript
./target/debug/lkjscript check src/examples/hello/main.lkjscript
./target/debug/lkjscript run src/examples/hello/main.lkjscript
```

A successful `check` is silent. It validates the required package and uses the production compiler
through validated bytecode, then drops the compiled result without entering the program. `run` is
the intentional effectful operation; the example prints `3628800`. It attempts one eligible
baseline-native group before effects, otherwise executing the unchanged validated program in the VM.
Use `check <entry> --json` for one deterministic machine document. A represented invalid package or
program still exits nonzero and keeps stderr empty; command misuse reports on stderr.
The VM has no JIT dependency or native-transition state. There is no engine, threshold, or forced
native CLI selection, and native entry is a commit point with no VM retry. The discarded
optimizing runtime and forced execution helpers are deleted. The SSA evaluator is available
only as the opt-in `lkjscript-ir/test-oracle` feature used by development and differential tests;
production dependencies do not enable it.

The active product scope is local package check/run plus the compiler's in-process semantic
workspace library. Text and paths are importer conveniences, not program authority.
`Workspace::empty` creates a source-free incomplete program; revision-checked transactions can
create products, enums, generic or non-generic functions, and an entry point with ordered explicit
capability parameters, then fill real typed holes with flat non-recursive drafts. Function creation
uses declaration-local binder handles only until atomic publication assigns stable type-parameter
entities. Main capability parameters and ordinary value parameters receive stable entities. Drafts
construct exact generic calls with structured `SemanticType` arguments and compiler-derived trait
witnesses; source inference and semantic edits share the same exact instantiation validator.
Implemented drafts also include immutable and mutable lexical locals, ordered sequence, assignment,
`while`, explicitly typed `loop`, nearest-lexical `break` and `continue`, early `return`, every
canonical operation supported by the ordinary runtime-operation lowering class, byte-vector
move/borrow, product/enum construction and observation, and source-free exhaustive enum payload
matches. Match
arms use flat `PatternDraft` trees; named payload bindings receive stable workspace entities while
compiler-only scrutinee and projection storage remains hidden. Transactions can delete callables and
whole user product/enum declarations atomically; owned members and target implementations cascade,
independent surviving dependents block, private
dense identities compact, and public survivor identities remain stable. A narrow
`MoveSequenceChild` transaction changes one sequence's evaluation order by stable child and sibling
identities while preserving every surviving subtree identity; it does not rebuild source or expose a
path/index editing API. The complete immutable `WorkspaceSnapshot` derives compiler HIR directly,
without rendering or parsing source. Imported and
programmatically constructed programs use the same canonical match
checker/lowering, semantic authority, query/index model, completeness gate, ownership checker, and
compiler boundary.

The `.lkjscript` extension is the only fixed source-format property. The current line-oriented
encoding is provisional and non-authoritative; no textual, binary, or compatibility promise follows
from it.

A minimal source-free authorship sequence is:

```rust
use lkjscript_compiler::{
    BuiltinTrait, DeclarationType, DraftTypeParameterId, Edit, ParameterDraft,
    SemanticTrait, SemanticType, Transaction, TypeParameterDraft, Workspace,
};
use lkjscript_contracts::CapabilityKind;

let mut workspace = Workspace::empty().expect("empty semantic workspace");
let revision = workspace.current().revision();
let parameter = DraftTypeParameterId::new(0);
let created = workspace.apply(Transaction {
    base_revision: revision,
    edits: vec![
        Edit::CreateFunction {
            name: "identity".into(),
            type_parameters: vec![TypeParameterDraft {
                id: parameter,
                name: "t".into(),
                bounds: vec![SemanticTrait::Builtin(BuiltinTrait::Copy)],
            }],
            parameters: vec![ParameterDraft {
                name: "value".into(),
                ty: DeclarationType::DraftTypeParameter(parameter),
            }],
            return_type: DeclarationType::DraftTypeParameter(parameter),
        },
        Edit::CreateMain {
            parameters: vec![ParameterDraft {
                name: "stdio".into(),
                ty: DeclarationType::Capability(CapabilityKind::Stdio),
            }],
            return_type: SemanticType::Unit,
        },
    ],
}).expect("atomic generic declaration creation");
assert!(!created.snapshot.completeness_blockers().is_empty());
```

The public transaction diff returns the function, stable type-parameter, ordinary and main
value-parameter, and body-hole identities; the declaration-local handle does not escape publication.
A focused end-to-end test constructs recursive factorial plus `main(stdio: capability stdio)`,
formats and prints `fact(10)` through canonical operations, compiles with `compile_snapshot`, and
matches the imported hello output with zero source-loading and parser invocations.

The workspace has one application binary, `lkjscript`; there is no semantic wire service pending a
measured consumer. Daemon, process-cell, session, scheduler, resource-topology, service-database,
and Linux-observation products are intentionally absent. The language's SQLite capability remains
implemented directly by the VM and `lkjscript-sys`, alongside stdio, clock, filesystem, network,
and terminal operations.

## Verification

```sh
cargo fmt --all -- --check
cargo clippy --quiet --workspace --all-targets --all-features --locked -- -D warnings
cargo test --quiet --workspace --all-targets --all-features --locked
cargo build --quiet --workspace --release --locked
```

Rustfmt is already silent on success; the other native quiet forms remove routine Cargo progress
while preserving failures. Use focused tests during iteration and reserve this full boundary for the
final relevant state. The retained container
verification builds the workspace release once, checks the effectful hello example without entering
it, and runs hello, Mandelbrot, and all seven local shell smoke/check scripts, including SQLite:

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
