# Admit and lower exact scalar intrinsics from complete source

Date: 2026-10-24. Status: implemented in the maintained canonical compiler;
joined targeted verification and isolated reachability measurement passed.
Full publication acceptance remains required as recorded below.

## Decision and dependency

Extend the source-driven scalar E2/E3 path with eight exact pure I64/Bool
intrinsic implementations. The native source compiler checks their declarations
and applications in the complete supplied LKJSG001 carrier, constructs primitive
callable rows internally, and emits ELF through the unchanged backend.

The accepted predecessor is commit
`01c805a202b258a1a9241f1edafd5feaa7655ce1`, tree
`e5ee88ec3a01b3f5b79c19f9b444c6ab6d500bbc`. Its
[carrier-capacity slice](20261024-source-carrier-byte-capacity.md) removed the
aggregate source-byte quota before canonical owner replacement added more
retirement history. This slice closes a concrete missing semantic/lowering path
without expanding the scalar type family or changing any binary format.
The [source-input specification](../spec/selfhosting-source-input.md) owns the
admitted profile and diagnostics.

## Exact permission and whole-inventory checking

Permission derives from the complete implementation identifier, pure callable
kind, empty generic-binder list, exact result type and exactly two unrestricted
parameters. Declaration names and reachability grant no permission.

| Implementation | Parameter types | Result | Mode-0 primitive code |
| --- | --- | --- | ---: |
| `core.i64.add` | I64, I64 | I64 | 1 |
| `core.i64.subtract` | I64, I64 | I64 | 2 |
| `core.i64.multiply` | I64, I64 | I64 | 3 |
| `core.i64.divide` | I64, I64 | I64 | 4 |
| `core.i64.equal` | I64, I64 | Bool | 5 |
| `core.i64.less` | I64, I64 | Bool | 6 |
| `core.bool.and` | Bool, Bool | Bool | 9 |
| `core.bool.or` | Bool, Bool | Bool | 10 |

Every supplied External is checked, including unused declarations and aliases.
Unknown or other implementations, generic binders and borrow/consume parameter
modes reject with diagnostic 13. A recognized implementation with an incorrect
scalar signature or arity rejects with 19. Calls independently check arguments
against the admitted declaration. The existing whole-inventory pass still
checks uncalled function bodies, untaken branches, annotations and unused supplied
types. An empty-effect task Function or TaskFunction remains a task and rejects
at the unsupported scalar-profile boundary.

Structural admission precedes scalar permission. The intrinsic negative controls
use complete, freshly hashed carriers and first establish E1 success, so their
E2 failures are not digest or framing failures. Existing independently constructed
missing-endpoint and disconnected-cycle controls remain separate structural
checks. Rust transports raw carrier bytes and orchestrates the tests; it supplies
no target semantic table, normalized instruction stream or custody certificate
to this source-compilation route.

## E3 lowering and retained domains

The planner inventories all supplied pure Functions and admitted Externals in
canonical owner order. An External receives a zero-instruction callable row
containing its primitive code, exact signature and parameter-local slots. Calls
resolve through the same stable directory as ordinary functions. Trace and
emission skip nonexistent External bodies while preserving their parameter
layout. The unchanged backend independently validates the generated mode-0 input.

Argument evaluation remains eager and left-to-right, including Bool and/or.
Checked arithmetic preserves integer-overflow and division traps. A call in tail
position still uses ordinary Call/Return. The scalar profile contains no owned
values and establishes no ByteBuffer borrow, branch reconciliation or disposal
contract.

No work counter is removed or redefined. Inference retains its finite progress
measure: immutable resolved facts, a strictly decreasing unresolved count, or
deterministic rejection. Canonical directory scans and the existing containment
cursor remain bounded by the admitted finite input. Structural cycle and scope
checks remain mandatory.

LKJSG001 framing, LKJCI1 mode 0/profile 4, mode 1/profile 5, canonical .lkja
encoding and the native runtime are unchanged. Retained restrictions include
100,000 combined source-directory entries, individual object bounds, the
positive-I32 proof-allowance domain, an 8,388,608-byte generated mode-0 input,
1,024 callable rows, 128 parameters/locals per row and operand depth 128.
The mode-0 complete-layout bound remains
`2176 + 40*F + 32*N + B <= 8,388,608`; there is no separate node-count quota.
The independent 700-unit artifact guard still protects flat map emission.
This capability slice does not remove those representation or implementation
restrictions.

## Public adoption and six-step bootstrap

The public transaction advances
`rev_2ea581ffa2b517be5b5decabc899000f11f46d54f543374c47ff283a844de098`
to
`rev_dd94b401eb30277e5289bbc9d651230337edcd6b1391e46fabcd92c0fe38d427`.
Transaction
`transaction_695480ddf4005e13c99f1a264e03cacc1b7793460f81c72287adf32d55265faf`
was accepted with differential equality, nine updates and 851 created canonical
owners. The created-owner count is distinct from the five new callable
declarations.

| Module | New functions | Existing body updates |
| --- | ---: | ---: |
| source-scalar-profile | 5 | 4 |
| source-scalar-infer | 0 | 1 |
| source-scalar-plan | 0 | 2 |
| source-scalar-trace | 0 | 1 |
| source-scalar-emit | 0 | 1 |
| Total | 5 | 9 |

The five helpers are `callable`, `external-types`, `parameters`,
`external-code` and `external-profile`. All updated declarations retain their
ordered non-body forms, including parameter identities, types, use modes and
result types. The proposal omits 66 unchanged functions. Authoritative meaning
remains the accepted semantic project; source files are authoring projections.

1. **Builder:** Use the public Rust CLI from accepted commit `01c805a202...`.
   Its actual executable SHA-256 is
   `2875b80183af2325a61ff5ea27a8e3f8cae19215b8a1007ecb895a42edcca426`;
   Cargo reported it fresh. It performs public authoring. The Rust native backend
   bootstraps the first source compiler.
2. **Consumed input:** Bootstrap and maintained Tier 1 reproduction use existing
   normalized inputs. Source target compilation receives complete raw LKJSG001.
   There is no transitional format or dual reader.
3. **Prerequisite capability:** The admitted backend already supports these eight
   primitive codes, exact signatures, eager arguments and integer traps. No new
   runtime or machine capability is introduced.
4. **Regeneration:** Public adoption regenerates the canonical project, complete
   carrier, native measurement and whole-inventory profile from the new revision.
   The actual formatted spec/test/source bytes are bound before targeted tests.
5. **Independent verification:** Run the whole-carrier E2 VM/reference/native
   matrix, native source E3 value/trap controls, existing node-capacity and
   structural controls, maintained normalized fixed points and complete profile.
   Publication separately requires every exact-tree gate below.
6. **Cleanup:** No temporary format bridge or compatibility shim is introduced.
   Reachability measurement uses an independent data-only project copy, validates
   its GC receipt and preservation checks, then removes only the owned copy.

## Targeted evidence

The shared scalar matrix adds 26 cases without adding a Rust test function:

| Added control family | Cases | Independent expectation |
| --- | ---: | --- |
| Arithmetic, truncation, false relations, I64 boundary results and recursion | 6 | Exact successful I64 output |
| Add/subtract/multiply overflow, zero/overflow division, eager and/or, left-before-right and operand-before-intrinsic order | 9 | Exact normalized trap, exit 70, empty stdout |
| Unused unknown/outside-profile implementation, forged signature/result/parameter/arity, borrow/consume mode, generic binder and invalid unused calls | 11 | Exact E2 diagnostic 13 or 19 and no executable output |

E2 runs the same complete carriers through the VM, independent reference
interpreter and native checker. E3 compiles every admitted case from raw source
and executes the resulting ELF against independent expectations. The baseline
and intrinsic-arithmetic cases additionally compare the full producer output
across the language engines. No Rust target-lowering oracle is supplied.
The existing balanced witness still lowers 32,770 instructions and executes to
42; both hash-consistent structural negatives remain selected.

All three targeted commands joined successfully, with zero failures:

| Command filter | Passed / ignored | Test-reported seconds | Whole command milliseconds |
| --- | ---: | ---: | ---: |
| `selfhost_source_scalar_` | 5 / 0 | 321.97 | 516,360.533 |
| `selfhost_source_measure_maintained_native_closure`, explicitly enabled | 1 / 0 | 23.72 | 23,873.855 |
| `selfhost_source_profile_inventories_every_owner_and_type` | 1 / 0 | 11.96 | 12,077.030 |

Each command uses `cargo test --release --locked --lib`, one test thread and
`--nocapture`; the maintained measurement also selects `--ignored`. Command
time includes work outside the test-reported interval and is not interchangeable
with compiler execution time. Public adoption took 16,158.288 ms and reported
zero selected differential tests; its accepted/equal result is separate from
the subsequent executable controls.

Evidence under `.artifacts/selfhost/source-scalar-intrinsics` binds the actual
source, accepted builder, canonical revision and complete carrier:

| Receipt | SHA-256 |
| --- | --- |
| Joined targeted continuation | `53f4129a45ca1263d956a8e83b9c2b09c3e2d83b218274205c46c41ac0aaacf4` |
| Maintained native measurement | `50e84bbb57dd015250c7658d564326d7c289839b1e27cec2af5289511dafa173` |
| Complete profile | `8c63ea88fc644292cab88eff8ab50bb2e4b11ac0d20d66c35e28d53dbbb1fcfd` |
| Targeted source-input specification | `4191b337998d51cce1bcc124550b3f0e65470bfc86618bbf385e82bc7efd1367` |
| Isolated reachability report | `63bc08049adb0e0654f8a9f20beffc8666017664c34515388a3a6011b15616a0` |
| Reviewed reachability script | `b74a3adaec7bf16c1a500d7d7d7996f3a0b1679a48af27c2bbb3fa9c9b429877` |

## Separated maintained-closure measurements

The columns compare accepted carrier revision `rev_2ea581ff...` with intrinsic
revision `rev_dd94b401...`. They are different maintained closures. No same-input
old/new performance experiment was run, no test pruning or counter redefinition
occurred, and no speedup is claimed.

| Observation | Before | After |
| --- | ---: | ---: |
| Modules / callable declarations | 72 / 577 | 72 / 582 |
| Pure functions / externals | 554 / 23 | 559 / 23 |
| Live canonical owners | 33,655 | 33,919 |
| Retirement records | 11,064 | 11,651 |
| Supplied type objects | 35 | 35 |
| Complete LKJSG001 carrier bytes | 7,941,194 | 8,110,321 |
| Mode-1 LKJCI1 bytes / units | 9,086,555 / 581 | 9,155,057 / 586 |
| Emitted .lkja bytes / units | 9,603,621 / 581 | 9,675,994 / 586 |
| Backend mode-0 bytes / functions / nodes | 832,576 / 244 / 16,456 | 832,576 / 244 / 16,456 |
| Combined source frontend mode-0 bytes / functions / nodes | 1,661,372 / 561 / 34,201 | 1,672,927 / 566 / 34,469 |
| Combined source frontend normalized type entries | 31 | 31 |
| Direct local full-kernel work | 3,094,141 | 3,120,526 |
| Composed callable telemetry work | 7,995,020 | 8,076,365 |
| Composed callable modeled metadata bytes | 11,401,876 | 11,499,321 |
| Physical source packs / bytes | 297 / 406,592,267 | 346 / 419,166,252 |
| Reachable historical objects | 227,308 | 231,731 |
| Reachable historical canonical bytes | 485,420,559 | 497,593,841 |
| Isolated GC retained pack bytes | 387,778,025 | 397,154,190 |
| Isolated GC modeled preparation retention bytes | 543,611,407 | 556,916,977 |
| Maintained native measurement wall time, ms | 23,085 | 23,710 |
| Native requested allocation, live retention, mapped capacity and RSS | Unmeasured | Unmeasured |

The slice adds 264 live owners and 587 retirements; the full carrier grows by
169,127 bytes. Its 49 new physical packs occupy 12,573,985 bytes, and every old
pack is preserved. Physical storage, reachable canonical historical payload,
modeled metadata and actual native memory are separate observations.

The isolated public GC run joined with exit 0 in 181,036.967397 ms. Its data-only
copy used reflink-or-full-copy with no hardlinks. The copy contained 232,515
objects before GC and retained 231,731, reclaiming 784; its 346 packs became
16, reclaiming 22,012,062 physical pack bytes. The original project retained
all 346 packs and 419,166,252 bytes. The report verifies the original project,
source/index/branch, history, builder and bound evidence unchanged, validates
the GC receipt, and records successful cleanup with no errors. No source project
compaction or reseed occurred. Modeled GC retention is neither requested
allocation nor resident memory; native lifetime measurements remain unmeasured.

The raw report is bound to the joined continuation at
`.artifacts/selfhost/source-scalar-intrinsics/reachability-53f4129a45ca1263d956a8e83b9c2b09c3e2d83b218274205c46c41ac0aaacf4/report.json`.
Its scope is versioned semantic HEAD ancestry, receipts, transactions and source
objects, excluding derived cache and operational selection roots. It is a
historical-storage measurement, not same-process native compiler reclamation.

The complete carrier SHA-256 is
`b800b61e8d90d6c507fa9918130bc629bdbad491b9d9ec38f79b28beeaa64177`;
semantic state is
`semantic_state_706b6db4ae98c6adb7511c56da0e893974a45585b91fbaf48e8e38593870fa19`.
Mode-1 input SHA-256 is
`c868e4d5a65ff3cebbe2bd5f44aaff593686c656535fd44ced12da8b68070493`.
The complete carrier identity, not semantic state alone, binds these results.

The whole-inventory profile still records 24 structural record types, 146 Text
expressions, 91 List expressions, 14 generic application sites, one borrow
parameter and 159 consume parameters. These observations identify remaining
dependencies; they are not admission or cleanup certificates. Composed work is
passive telemetry under the accepted unmetered composed policy, not remaining
headroom below a 10-million quota. Other validation/preparation limits remain
distinct.

## Fixed-point scope and publication gates

Maintained Tier 1 native C2=C3 remains 2,590,880 bytes, SHA-256
`55eaea3fcc2604a386f68a5dffd5bb9c2ca8e936ca0d2bf94235e1770c2487d5`.
Artifact output from C1/C2/C3 agrees at 9,675,994 bytes, SHA-256
`6dab0ce912baa2306325d2f74c5e341f12264797213fd9bf8f67fe0cdd62840a`.
Both chains consume Rust-normalized maintained inputs. They do not establish
maintained-source C2=C3 or reference/C1/C2 canonical A1=A2=A3.

Backend C1 and C2 reproduce the same 5,312,463-byte combined source frontend,
SHA-256 `70a6e0ab7183962f1d27d39a2eff4e589658bd56fe2554165418097164d31d54`.
Both and the 705,162-byte Rust-bootstrap frontend compile the pinned complete
raw scalar carrier to an independently executed program returning 42, and reject
the hash-consistent unused Bytes-type carrier with diagnostic 13. The complete
maintained compiler source remains outside E2/E3 permission.

The maintained measurement records Linux x86_64, 64-bit pointers, empty native
environment, 4,093-byte input chunks, a ten-second child deadline and a 32 MiB
harness output policy. Source-control logs additionally bind each compiler,
complete input carrier, generated program and exact execution expectation.
Those harness policies are not source admission quotas.

No full publication gate has run for this intrinsic candidate at the time of
this decision. Before commit, root must freeze the intended staged tree and run:

1. `bash tools/bootstrap-verifier.sh --expected-source <actual accepted base> --expected-tree <actual staged tree> --output <owned evidence directory>`,
   recording its stdout-derived verifier root and using that root's
   `lkjscript-dev`.
2. Literal `bash tools/selfhost/verify.sh`: expected 127 selfhost passes with
   two ignored tests, and ten native passes with none ignored.
3. That bootstrapped verifier's
   `check full --fresh --machine --jobs 4`: all 29 gates fresh, zero reuse,
   stable source/evidence identities and a joined terminal receipt.
4. The same verifier's `release target`, then review and publish the exact
   verified tree through normal main integration and remote confirmation.

These are required outcomes, not observed results. There is no root `./verify`
executable. Existing gate owners coordinate one checkout and one canonical target.
Git and terminal publication receipts own the future intrinsic commit identity.
Native same-process reclamation and full P_self Tier 2 acceptance remain unproven.

## Next frontier

Implement explicit Target selection next, retaining complete supplied-inventory
checking before entry selection. Then extend Bytes/flat records, Text, closed
List substitutions and complete ByteBuffer custody/cleanup to cover the maintained
profile. Reconcile and measure each actual successor; prepared local drafts do not
establish adoption or runtime acceptance.

At 586 measured mode-1 units, the independent 700-unit flat-map restriction
remains relevant. Replace that emission path before a measured successor reaches
the unit blocker. Reassess complete carrier size, retirement count and the retained
proof domain separately. Source fixed points, independent canonical artifact
parity, same-process native reclamation and the remaining nominal, closure,
witness, read-view and Region families remain required follow-on work.
