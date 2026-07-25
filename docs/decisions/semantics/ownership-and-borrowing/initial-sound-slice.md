# Ownership And Borrowing: Initial Sound Slice

[Authority](../ownership-and-borrowing.md)

## Status

**Mixed.** Current, Accepted Target, Deferred, Rejected, and historical evidence status follows the
explicit labels in this capsule and its authority; this capsule cannot promote a capability.

## Initial Sound Slice

**Current.** The first implementation slice is a complete safe island around
`Owned Buf`:

- `owned-buf-new` creates a fresh `Owned Buf` that cannot have a pre-existing
  alias;
- whole local/parameter places only;
- explicit `move` for ownership transfer;
- `borrow` and `borrow-mut` create non-escaping `Ref Buf` and `RefMut Buf`;
- owned-buffer read operations require `Ref Buf`, and writes require `RefMut
  Buf`;
- whole-program ownership analysis is charged against the deterministic
  `OWNERSHIP_ANALYSIS_MAX_EXPRESSION_NODES = 16_384` aggregate HIR-expression
  budget before place/loan analysis begins;
- a `borrow`/`borrow-mut` expression is accepted only as an exact direct
  reference argument or a direct `let` initializer; a direct argument's loan
  remains active through the complete call/runtime-operation expression;
- same-basic-block last-use dataflow ends a local borrow before lexical scope
  end where proved. Branch-local borrows are valid when each definition and all
  uses remain in its arm, but transporting one Borrow result across an SSA
  block edge is conservatively rejected;
- branch ownership states join exactly. Unsupported loop-carried moves/loans,
  reborrows, field/index places, return/storage of references, and partial moves
  are rejected;
- all generic instantiations whose signature or substitution contains `Owned`,
  `Ref`, or `RefMut` are rejected, including nested occurrences;
- SSA retains explicit place initialization/end, move, borrow, owner-transport,
  loan, and alias identities even though the VM representation remains the
  existing safe arena handle. Its public verifier uses bounded forward CFG
  dataflow under `OWNERSHIP_VERIFY_MAX_WORK = 131_072`, exact joins, explicit
  affine transfer, canonical current-owner facts, global LoanId uniqueness, and
  same-block loan checks after every pass. Work and retained ownership state are
  separately capped at 131,072 charged units/cells. The public CFG verifier
  additionally requires dense block storage, caps a function at 4,096 blocks,
  uses bounded bitset dominators, and caps charged CFG work at 4,194,304 word
  operations. Affine values crossing a basic-block boundary must use explicit
  typed block arguments; direct predecessor-value carry is deliberately
  rejected.

The initial owned-buffer operations consume typed references directly; general
`ref-read`/`ref-write` syntax is rejected until place projection is implemented.
This slice does not silently make legacy `Buf`, `Handle`, product, or collection
uses affine; those remain worker-local GC/capability values until migrated by a
later breaking contract. It establishes a sound ownership path without
modifying canonical Brainfuck source. Deterministic source `Drop`, resource
RAII, named regions, arbitrary `Owned T`, and borrow-aware existing host
operations remain **Accepted Targets**. Runtime session cleanup remains the
Current final backstop and is not called language `Drop`.

Native GC references are added as a separate ownership category rather than
pretending they are lexical borrows.

Current source accepts only exact `Owned Buf`, `Ref Buf`, and `RefMut Buf`.
`Owned Buf` may occur in local annotations, parameters, and function/main
returns. References may occur only as function parameters or inferred local
borrow bindings. Product fields and List/Option/Result elements reject direct
or nested ownership/reference types. Generic ownership/reference
instantiation, named regions, nested ownership/reference constructors,
reference returns or user-call results, reborrows, projected places,
closures/capture, partial moves, cross-block Borrow results, Move/Borrow in
loop cycles, changed loop-carried owner/loan state, and unequal loop/branch
ownership states are rejected.
`RefMut` user-call forwarding is rejected in this slice. Semantic frame states
retain a reference through its consuming runtime call and omit a consumed
`RefMut` from later safepoints; these bytecode-era frame facts are not native GC
stack maps. Same-block NLL ends a
local loan after its last argument use; temporary argument loans end only after
the complete call/runtime-operation expression. Runtime/frame cleanup and SSA
`PlaceEnd` are lexical/root cleanup facts, not deterministic user `Drop`.
## Deferred And Rejected

Full partial moves, closure values, pinned source APIs, cross-worker transfer,
and collection element borrows are **Deferred** until their matrices pass.
Lexical-to-function-end approximation as the final model, implicit copies of
non-`Copy` values, raw safe pointers, conservative lifetime extension, and
source-asserted unsafe thread transfer are **Rejected**.
