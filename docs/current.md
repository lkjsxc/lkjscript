# Current implementation

This document describes implemented behavior in the checkout. It is not a promise of backward
compatibility.

## User-visible capability

- `.lkjscript` is the only accepted source suffix. Its current one-marker-per-line notation is a
  bootstrap projection, not the permanent semantic schema.
- Packages and modules use checked manifests, lock data, deterministic import resolution, path
  containment, and cycle rejection.
- The implemented language includes typed functions and calls, local bindings and mutation,
  conditionals and loops, products, enums, exhaustive matching, generic `Option` and `Result`,
  numeric conversions, bytes, byte vectors, lists, typed errors, and explicit capabilities.
- The command-line runtime supports a default automatic path and diagnostic `vm`, `baseline-jit`,
  and `optimizing-jit` selections. Forced native selections preflight support and report failure
  rather than claiming generated execution after fallback.
- Host adapters cover standard I/O and selected filesystem, TCP, hashing, terminal, and SQLite
  operations behind typed capability checks.
- Ordinary runtime memory is collector-free. Unique storage, invocation regions, structural
  values, and explicit host-resource ownership are implemented for the supported value families.

The executable examples under `src/examples/` and compiler/runtime tests own the exact supported
surface.

## Compiler and runtime foundations

Current text compilation follows this path:

```text
line-oriented text and package files
    -> source tree and resolved typed HIR
    -> ownership, effect, and memory-plan checks
    -> independently verified and normalized SSA
    -> validated bytecode
    -> default VM/native execution path
```

Trusted compiler entry points compile directly without selecting a compiler resource profile or
charging source, HIR, or SSA shape to a budget ledger. The lexer-token, children-per-form,
top-level-form, source nesting, 16 MiB per-file source, 256 MiB aggregate-source, and 65,536
source-unit validity ceilings have been removed. The parser uses fallibly grown explicit frames;
source projection, identity flattening, formatting, module rewriting, clone, and destruction use
iterative work stacks. Remaining recursive expression analysis, HIR memory planning, and SSA
lowering call sites use localized repeatable heap-backed stack segments rather than a finite depth
admission rule. Trusted validation, loading, package analysis, and compilation select an explicit
unrestricted source-byte policy. Source files are read to EOF in checked, fallibly reserved chunks;
metadata is a capacity and change-detection hint, not admission. Ownership analysis no longer
pre-scans HIR merely to reject an aggregate expression count. HIR memory-plan expression work is
checked observational `u64` telemetry rather than admission. SSA ownership verification likewise
has no ownership-work or retained-state-cell admission ceiling. Its worklist moves single-source
states and shares copy-on-write state across joins, while exact predecessor-state equality and all
ownership cleanup rules remain enforced. Generated SSA coverage verifies 44,000 owned resource
parameters and places with 132,000 active-place, owner, and affine state cells per block, 264,000
cells under the former aggregate retained-state accounting across a two-block propagation, one
44,000-argument consuming call, and exact cleanup of every place. Generated coverage compiles, creates
a verified HIR memory plan, creates verified and normalized SSA, validates bytecode, executes
through the VM, and destroys a program with 20,000 nested `do` expressions on a 256 KiB native
thread stack. That fixture records exactly 20,001 memory-plan expressions, 20,003 entries, and
40,045 verifier steps. A nested product-match fixture reaches a physical marker depth above 50,
and malformed 8,192-deep mismatched and unclosed input produces deterministic diagnostics and
drops partial trees on the same small stack. Other generated coverage compiles and executes a
source 1,024 bytes beyond the former 16 MiB boundary and exercises the source authority with 65,537
in-memory units. Checked accounting crosses 256 MiB; the exact 258 MiB compile-and-execute geometry
is retained as an opt-in stress test and has not been run as part of normal verification. Compile
metrics are observational phase timings and source-file counts only. Package manifests and
prepared-program identities likewise do not carry compiler-profile identity. Canonical
verified-SSA, validated-bytecode, and prepared-program identity bytes stream directly into one
incremental SHA-256 implementation owned by `lkjscript-contracts` and re-exported by core. Identity
construction has no canonical-byte, append-count, prepared-descriptor-byte, or ordered-closure-entry
admission ceiling and retains a fixed-size hashing working set. Prepared closures still require a
nonempty, nonzero, strictly ordered unique sequence, and canonical sequence lengths remain checked
against their `u64` wire representation.

Executable function arity and local slots now remain `usize` through HIR and code generation; SSA
frame-state slots use `u64`, and bytecode prototypes, owner-place metadata, and failure-cleanup
locals/places use host indexes. The one active bytecode format encodes branch targets, local indexes, call argument counts,
memory-witness ordinals, and single place indexes as fixed little-endian `u64`; instructions that
name both a place and local encode two separate `u64` operands. Bytecode function, block, and
instruction links, call-witness instruction offsets, cleanup node identities, and cleanup range
offsets are also `u64`, with checked conversion at host-index boundaries. Existing constant,
global, product, enum, and structural table, descriptor, and field operands remain `u16` or `u8`.
Decoding classifies operands as no operand, `u16`, checked host index, or checked place/local pair,
proves the complete operand is present, and rejects host-width overflow before validation or
indexing. The VM
uses checked frame arithmetic and fallible reservations, rejects a low stack host policy before
wide frame allocation, tracks instruction starts independently of encoded length, and tail-forwards high local slots without two- or three-byte instruction assumptions.
Generated production coverage compiles and executes 300 parameters, 300 arguments, and more than
255 simultaneously live lexical locals, reads slot 299, agrees with the SSA evaluator, and proves
that automatic execution remains on the generic VM route while forced native mode reports an
unsupported signature. A larger generated stress case executes 1,024 parameters, arguments, and
lexical locals through the VM and reads slot 1,023. A separate 1,024-owned-parameter and argument
source emits 115,754 bytes in main, validates and prepares that executable, executes exact cleanup
through the VM, and confirms that automatic execution retains the generic VM fallback when native
cleanup expansion is ineligible. A generated owned parameter in slot 299 also executes and cleans
up. Another generated source emits a function larger than 65,535 bytes with a branch target beyond
that former boundary; calls taking both branch paths execute through the VM. Malformed fixed-width
jumps are rejected when truncated, aimed into operand bytes, outside the function, or unrepresentable
as a host index. Failure cleanup is represented in SSA and bytecode by deterministic
hash-consed backward-only node chains. Ordinary cleanup has separate loan, unplaced-owner, and
place roots so independently changing segments do not copy one another; call-unentered cleanup has
its own root. The production 300-owned-byte-vector parameter/argument fixture publishes and runs in
the VM with 315,450 logically expanded cleanup actions represented by 1,200 physical nodes, while
exercising local and place indexes above 255. Validation rejects duplicate nodes, empty root sets,
self, forward, and out-of-range links before indexing and checks aligned, sorted, nonoverlapping
ranges plus exact live-owner coverage whenever cleanup metadata is present. The native machine-plan
backend still materializes per-instruction cleanup calls, so preflight explicitly declines a shared
shape whose expansion exceeds its private 65,535-call eligibility envelope; automatic execution
keeps the validated generic VM path rather than failing publication. Direct
validated-bytecode coverage executes unique local and place 299 and rejects equal-to-count,
truncated, misaligned-jump, and malformed cleanup references. Prepared identity now records native
transport specialization as an optional identity rather than substituting the semantic SSA identity
when specialization is unavailable.

A Semantic Source service already exposes snapshots, stable node queries, typed holes,
diagnostics, transactions, and a local stdio session. It supplies an explicit limited aggregate
source-byte policy at its untrusted boundary; the same policy checks staged transaction source
bytes before publication. It has no source-unit, token, node, or work admission quota. Other
boundary-local byte and request-count policies remain for untrusted framing and persistence. It
currently mirrors the text-oriented source tree and the compiler still recompiles from text, so it
is a bootstrap editing service, not yet the intended semantic program authority.

## Tested platform

The broad local suite and native paths are exercised on Linux x86-64. Portable Rust components
may build elsewhere, but no other host or native target is currently claimed as tested.

## Known gaps

- Source spans, positions, and snapshot-local node indexes remain `u32`, so an individual source
  or source tree beyond those addressable ranges fails at a representation boundary. HIR, SSA,
  recursive type/trait/enum, and structural-value paths retain other arbitrary count or recursion
  ceilings. HIR memory planning still has independently triggered quotas for functions, entries,
  uses, loans, constants, calls, obligations, type nodes and edges, witnesses, aggregate shape,
  destinations, borrow scopes, drop paths, and deterministic verifier/SCC work. The
  20,001-expression fixture does not cross those tables: it has one function, 20,003 entries, one
  constant and type fact, no uses, loans, calls, obligations, destinations, or borrow scopes, and
  40,045 verifier steps. Bytecode `ValidationLimits` still cap total encoded chunk bytes, physical
  table entries, metadata bytes, and constant data during validation; there is no separate
  per-function code-byte validity limit. The 300- and 1,024-owner production geometries stay below
  the remaining physical cleanup-node/range limits after sharing. Constants, globals,
  product/enum/structural tables, product fields, enum substitutions, and several structural
  descriptors retain `u16` or `u8` representation ceilings. HIR and SSA place identities remain
  `u32`, a separate above-`u32` representation gap rather than the removed executable byte width.
  Validator-synthetic ownership identity no longer narrows bytecode positions or parameter indexes
  to `u32`; it uses tagged instruction offsets and parameter indexes at host width. Where the
  remaining bounds constrain trusted compiler output rather than an untrusted serialized boundary,
  they remain follow-up validity and representation gaps, not host policy.
- Recursive compiler paths not exercised by the ordinary deep-expression production vertical,
  including parts of type, trait, enum, semantic-schema, and transaction processing, still need
  explicit work-stack conversion or equivalent evidence. Some analyses retain poor large-input
  complexity.
- The compiler cannot yet consume a syntax-independent semantic snapshot directly.
- Semantic edits still publish text files, and stable identity remains coupled to the current
  source representation in important paths.
- The evaluator, VM, baseline native path, and proof-oriented optimizing path still multiply the
  implementation surface. Their long-term roles have not yet been selected by representative
  measurement.
- Some host-resource cleanup obligations remain explicit rather than compiler-inserted on every
  implemented outcome.
- The daemon, process-cell, scheduler, and database foundations exist, but they are not required
  to validate the local language and compiler architecture.
