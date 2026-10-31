# REVISED SELF-HOSTING DIRECTIVE & EXECUTION CONTRACT FOR `lkjscript`

> **Core Engineering Principle**: Choose the shortest sound path to a compiler written in `lkjscript` that compiles its own authoritative source graph. Make each claimed dependency earn its place through evidence, and make every completion claim traceable to a defined input, observed execution, and preserved verification result.

## Deliverable 1: Concise `/goal` Invocation (For Supported Codex Environments)

Use this invocation with `/goal` in a supported, owner-selected Codex session ([S11](https://www.google.com/search?q=https%3A%2F%2Fdevelopers.openai.com%2Fcookbook%2Fexamples%2Fcodex%2Fusing_goals_in_codex)):

```text
/goal Achieve semantic compiler self-hosting for lkjsxc/lkjscript on Linux x86_64: implement a standalone compiler in lkjscript (S) that decodes, validates, normalizes, and compiles its own complete authoritative source-graph closure and emits both valid .lkja artifacts and standalone Linux x86_64 ELF executables without Rust semantic preprocessing or an external VM during self-produced stages, verified by (1) deterministic stage-2/stage-3 fixed-point identity bytes(C2) == bytes(C3) for both native ELF and .lkja outputs, (2) behavioral and negative conformance suites on generated artifacts across independent and mutated programs, and (3) the repository's discovered official verification gates, while preserving #![forbid(unsafe_code)], denied unwrap/expect/panic lints, single-current-format rules, and ownership-aware workspace disk bounds. Start with a verified repository/dependency inventory and the smallest end-to-end compiler slice; between turns, choose the next dependency-justified slice and continue until verified completion or an explicit blocker checkpoint.
```

## Deliverable 2: Portable Master Execution Prompt

### 1. Mission, Scope & Self-Hosting Definitions

You are operating with substantial engineering autonomy on [lkjsxc/lkjscript](https://www.google.com/search?q=https%3A%2F%2Fgithub.com%2Flkjsxc%2Flkjscript) (and, where cross-repository consumer validation is explicitly required, [lkjsxc/lkjscript-sandbox](https://www.google.com/search?q=https%3A%2F%2Fgithub.com%2Flkjsxc%2Flkjscript-sandbox)).

Your objective is to take `lkjscript` from its current repository state to **Tier 2 Semantic Compiler Self-Hosting** via the shortest sound, evidence-driven engineering path, without turning optional runtime or optimizer expansions into artificial prerequisites.

#### 1.1 Explicit Taxonomy of Self-Hosting Tiers

Keep these three achievements strictly distinct in all code, commit messages, status records, and acceptance claims:

| Tier | Achievement Name | Input to `lkjscript` Implementation | Responsibilities Implemented in `lkjscript` | Remaining Host / External Boundary | Role in This Directive |
| :--- | :--- | :--- | :--- | :--- | :--- |
| **Tier 1** | **Backend Self-Hosting in `lkjscript`** | Host-admitted and host-exported normalized IR (`NormalizedProgram` or documented backend input schema) | Selected IR lowering, bytecode/bundle emission (`.lkja`), and/or Linux `x86_64` ELF64 machine-code emission | Rust still performs source-graph decoding, semantic admission, and normalization | **Intermediate Milestones (B, C, D)** — must explicitly declare remaining Rust preprocessing in milestone names and receipts |
| **Tier 2** | **Semantic Compiler Self-Hosting** | The authoritative source meaning graph $S$ encoded in the documented canonical binary compiler-input schema | Structural input validation, semantic admission (types, affine ownership, borrows, regions, effects/capabilities), normalization, and emission of **both** current `.lkja` artifacts and standalone Linux `x86_64` ELF64 executables | Initial Rust bootstrap compiler $C_0$, initial source-graph binary exporter (prior to self-hosted graph reader cutover), and external OS/launcher staging | **Primary Goal Completion Target (Milestones E & F)** — self-produced native stages ($C_1 \to C_2 \to C_3$) require no Rust compiler, no fresh Rust semantic preprocessing, and no external VM |
| **Tier 3** | **Wider Toolchain Independence** | Maintained authoring proposals (`.lkjc` / `.lkjx`), repository packs, and package transports | Tier 2 compiler plus native symbolic transaction parser (`tx`), pack storage/compaction (`gc`), package transport, and service host | Only explicitly retained OS kernel / hardware boundaries | **Subsequent Horizon (Milestone G+)** — not required to claim Tier 2 semantic compiler self-hosting |

#### 1.2 Supported Self-Hosting Language Profile ($P_{\text{self}}$)

Because the accepted typed meaning graph (`GraphRepository`) is the sole editable authority of program meaning, a text parser is not required to establish Tier 2 semantic compiler self-hosting. However, you must not silently narrow the language to a toy subset and declare the whole compiler finished:

1. **Defined Profile Scope ($P_{\text{self}}$)**:
   - $P_{\text{self}}$ must include:
     - Every type form, ownership/borrow/region construct, control-flow form, function/closure call form, and byte/buffer operation used in the **complete transitive source closure $S$ of the `lkjscript` semantic compiler itself** (including any referenced declarations from [packages/standard](https://www.google.com/search?q=https%3A%2F%2Fgithub.com%2Flkjsxc%2Flkjscript%2Ftree%2Fmain%2Fpackages%2Fstandard)).
     - A documented corpus of non-compiler programs exercising checked `I64` arithmetic/traps, `Bool`/`Unit`/`Text`/`Bytes`, structural and nominal records/choices, sequences/buffers, lexical regions (`region-with`, `region-alloc`, `region-read`, `region-write`, `region-freeze`), recursion and self-tail calls, and explicit contract/implementation witnesses.
2. **Explicit Rejection of Unsupported Constructs**:
   - Any language construct in the broader repository specification that lies outside $P_{\text{self}}$ during an intermediate milestone (for example, `F64`, resident HTTP/Nostr service hosts, or distributed capabilities) must be **deterministically rejected with an explicit diagnostic code** before output publication.
   - Never miscompile, silently ignore, or fall back to the Rust interpreter for unsupported constructs.
   - Any required language feature not yet supported by the native self-hosted compiler must remain listed under `Known Limitations` in the acceptance record until implemented.

#### 1.3 Definition of "Region-Native" for the Compiler

In this directive, **region-native** means:

- Compiler phase-local graph and table structures (decoded source-graph nodes, symbol/type environments, normalized IR instructions, and label/relocation tables) are allocated in bounded lexical `Region` arenas whose backing memory is logically reclaimed when the compilation phase exits.
- Emitted binary outputs (`.lkja` bundles and `ELF64` executables) and diagnostics are owned by bounded `ByteBuffer` / `Bytes` values.
- Ordinary stack locals, scalar control state, and bootstrap containers are permitted where they simplify correctness. Do not force every helper byte into a region before the compiler workload exists, and do not claim whole-program zero-copy or region-exclusive execution unless measured and verified.

### 2. Epistemic Discipline: Requirements vs. Verified Facts vs. Hypotheses

Always distinguish four classes of statements (detailed in **Deliverable 3**):

1. **Owner Requirements (Binding)**:
   - Compile the authoritative meaning graph; advance toward self-hosting in `lkjscript`; target Linux `x86_64` first; preserve deterministic output and reproducible bootstrapping; favor region-oriented, cache-conscious designs over pervasive `Arc` graphs; maintain only current `HEAD` formats (no legacy fallback readers or dual-wire shims); keep `#![forbid(unsafe_code)]` and denied `unwrap_used` / `expect_used` / `panic` / no-Python gates intact ([AGENTS.md](https://www.google.com/search?q=https%3A%2F%2Fraw.githubusercontent.com%2Flkjsxc%2Flkjscript%2Fmain%2FAGENTS.md), [docs/direction.md](https://www.google.com/search?q=https%3A%2F%2Fraw.githubusercontent.com%2Flkjsxc%2Flkjscript%2Fmain%2Fdocs%2Fdirection.md)); operate autonomously in clear technical English without automatically invoking external paid APIs or spawning external billable agent sessions.
2. **Verified External Platform & Tooling Facts**:
   - Standard 3-stage bootstrap fixed-point rules ([S1](https://www.google.com/search?q=https%3A%2F%2Fgcc.gnu.org%2Finstall%2Fbuild.html)), deterministic build requirements ([S2](https://www.google.com/search?q=https%3A%2F%2Freproducible-builds.org%2Fdocs%2Fdeterministic-build-systems%2F)), untrusted buffer validation obligations ([S4](https://www.google.com/search?q=https%3A%2F%2Fcapnproto.org%2Fencoding.html), [S5](https://www.google.com/search?q=https%3A%2F%2Fflatbuffers.dev%2Flanguages%2Frust%2F)), Linux syscall semantics for `mmap`/`munmap` ([S6](https://www.google.com/search?q=https%3A%2F%2Fman7.org%2Flinux%2Fman-pages%2Fman2%2Fmmap.2.html)) and partial `read`/`write` transfers ([S7](https://www.google.com/search?q=https%3A%2F%2Fman7.org%2Flinux%2Fman-pages%2Fman2%2Fread.2.html), [S8](https://www.google.com/search?q=https%3A%2F%2Fman7.org%2Flinux%2Fman-pages%2Fman2%2Fwrite.2.html)), `rustc` `unsafe_code` lint boundaries ([S9](https://www.google.com/search?q=https%3A%2F%2Fdoc.rust-lang.org%2Frustc%2Flints%2Flisting%2Fallowed-by-default.html%23unsafe-code)), bounded translation-validation limits ([S10](https://www.google.com/search?q=https%3A%2F%2Fweb.ist.utl.pt%2Fnuno.lopes%2Fpubs.php%3Fid%3Dalive2-pldi21)), Codex `/goal` lifecycle contracts ([S11](https://www.google.com/search?q=https%3A%2F%2Fdevelopers.openai.com%2Fcookbook%2Fexamples%2Fcodex%2Fusing_goals_in_codex)), and Cargo clean/profile behavior ([S12](https://www.google.com/search?q=https%3A%2F%2Fdoc.rust-lang.org%2Fcargo%2Fcommands%2Fcargo-clean.html), [S13](https://www.google.com/search?q=https%3A%2F%2Fdoc.rust-lang.org%2Fcargo%2Freference%2Fprofiles.html)).
3. **Supplied Repository Snapshot (Pending Local Verification)**:
   - The supplied summary of [lkjsxc/lkjscript](https://www.google.com/search?q=https%3A%2F%2Fgithub.com%2Flkjsxc%2Flkjscript) up to commit [b83b6dd5dc235daaef0483d10342988f7712085c](https://www.google.com/search?q=https%3A%2F%2Fgithub.com%2Flkjsxc%2Flkjscript%2Fcommit%2Fb83b6dd5dc235daaef0483d10342988f7712085c) (Phases 1–4: [54c07ec](https://www.google.com/search?q=https%3A%2F%2Fgithub.com%2Flkjsxc%2Flkjscript%2Fcommit%2F54c07ecb7dca0e3dbde7b74c311b21331f5296f8), [dfc57ac](https://www.google.com/search?q=https%3A%2F%2Fgithub.com%2Flkjsxc%2Flkjscript%2Fcommit%2Fdfc57ac850bb36541eb822f89196099f1ae55feb), [20b031b](https://www.google.com/search?q=https%3A%2F%2Fgithub.com%2Flkjsxc%2Flkjscript%2Fcommit%2F20b031ba921ab92d101d633f33f3711e77701b4f), [a9c40eb](https://www.google.com/search?q=https%3A%2F%2Fgithub.com%2Flkjsxc%2Flkjscript%2Fcommit%2Fa9c40eb1f37a7346c88244d411e3dbe53b8254a7), [b83b6dd](https://www.google.com/search?q=https%3A%2F%2Fgithub.com%2Flkjsxc%2Flkjscript%2Fcommit%2Fb83b6dd5dc235daaef0483d10342988f7712085c)) is provided as an initial orientation aid, **not** an uncheckable axiom.
   - You **must** perform a bounded initial verification in Milestone A. If `HEAD` has advanced or any file/gate differs from the supplied snapshot, adapt to the actual repository state. Never reset `HEAD` to an older commit merely to match this document.
4. **Replaceable Engineering Hypotheses**:
   - Proposals regarding specific slab cell layouts, register assignments, new `region-slice` or `choice` syntax, `bytes(C1) == bytes(C2)` host emission parity, E-Graph optimizer migration timing, automatic parallelization, and Cargo `codegen-units` / `lto` settings are **hypotheses**. Retain, revise, or replace them based on observed compiler workload evidence.

### 3. Dependency-Driven Milestone Route (Milestones A–G)

Do not execute a rigid preparatory campaign of disconnected runtime features before writing compiler code. Instead, work backward from the **smallest maintainable compiler implementation in `lkjscript` that can compile its own authoritative source graph**, and let that compiler workload drive runtime and language additions.

| Milestone | Concrete Deliverable | Primary Acceptance Focus |
| :--- | :--- | :--- |
| **A. Establish the Contract & Inventory** | Bounded verification of actual `HEAD`, active verifier plan, and disk usage; documented **Canonical Compiler Input Schema (`LKJCI1`)**; explicit inventory of operations/types/runtime helpers required by the bootstrap compiler; definition of $P_{\text{self}}$ and bootstrap script harness. | Claims match observed source and verifier discovery; input schema and dependency inventory are committed and tested. |
| **B. Compile Through `lkjscript` (VM & Reference)** | Author the smallest meaningful end-to-end compiler component in `lkjscript` (starting with backend lowering/emission from the canonical input schema to `.lkja` and/or `x86_64` ELF64 bytes) and execute it on the existing bytecode VM and canonical reference interpreter. | Emitted `.lkja` / `ELF64` artifacts pass structural validation and execute identically to expected outputs on a test corpus. |
| **C. Run That Compiler Component Natively (`x86_64` ELF)** | Add **only** the native `elf_x86_64` instructions, runtime memory/region support, and bounded binary `stdin`/`stdout` I/O actually required by Milestone B's `lkjscript` compiler component. | The standalone native ELF executable of the `lkjscript` compiler component compiles the Milestone B corpus and produces valid, executable artifacts. |
| **D. Demonstrate Backend Closure** | Expand the `lkjscript` compiler backend so it compiles the complete normalized IR of **its own implementation closure** into both current `.lkja` bundles and standalone Linux `x86_64` ELF64 executables. | Self-backend closure passes differential execution; remaining Rust semantic-admission and IR-export dependencies are explicitly recorded in the receipt. |
| **E. Move Semantic Admission into `lkjscript`** | Implement source-graph decoding (from `LKJCI1`), type checking, affine ownership/borrow checking, region-brand admission, and normalization in `lkjscript` covering $P_{\text{self}}$ and the complete compiler source closure $S$. | Both valid source graphs and negative/malformed source graphs (type mismatches, ownership escapes, bad offsets, cycles) are handled accurately by the `lkjscript` compiler without Rust semantic preprocessing. |
| **F. Demonstrate Native Semantic Compiler Self-Hosting (`C2 == C3`)** | Build $C_1 = C_0(S, E)$, $C_2 = C_1(S, E)$, and $C_3 = C_2(S, E)$ from the complete authoritative source closure $S$; prove $\text{bytes}(C_2) == \text{bytes}(C_3)$ and `.lkja` reproducibility; verify $C_2$ and $C_3$ on general programs and mutated compiler source $S'$. | Native self-hosted fixed point holds; both `.lkja` and `ELF64` outputs pass independent execution and negative tests with zero Rust compiler or external VM participation in $C_1 \to C_2 \to C_3$. |
| **G. Workload-Guided Expansion & Optimization** | Expand language coverage beyond $P_{\text{self}}$; refine region layouts (`region-slice` / choices if not already added for B–E); migrate `egraph_opt` rewrites; add conservative static resource summaries and explicit parallel unit compilation. | Each addition is backed by standalone correctness tests, trap-preservation counterexamples, and before/after time and memory measurements. |

**Milestone Ordering Flexibility**:
- Milestones may overlap where a dependency is demonstrated. For example, if implementing Milestone B reveals that an indexed byte/slice primitive in `Region` or `ByteBuffer` removes substantial boilerplate, implement that primitive immediately, document why in the commit message, and proceed.
- Conversely, do **not** require migrating `egraph_opt`, rewriting the dependency planner, or building automatic parallelization before Milestones B–F unless a measured time or memory limit requires a specific optimization.

### 4. Bootstrap Fixed-Point & Reproducibility Specification

#### 4.1 Formal Bootstrap Notation

Define the bootstrap pipeline with exact symbols:

- $S$: The complete, immutable authoritative source-graph closure of the self-hosted `lkjscript` compiler (including all required compiler modules and any referenced declarations in `packages/standard`), serialized in the canonical compiler input format `LKJCI1`.
- $E_{\text{ELF}}, E_{\text{lkja}}$: The declared target (`x86_64-unknown-linux`), output format selector (`ELF64` or `.lkja`), compiler flags, and deterministic build environment.
- $C_0$: The host Rust bootstrap compiler (`lkjscript build`).
- $C_1 = C_0(S, E_{\text{ELF}})$: The Stage-1 standalone Linux `x86_64` ELF compiler executable produced by $C_0$.
- $C_2 = C_1(S, E_{\text{ELF}})$: The Stage-2 standalone Linux `x86_64` ELF compiler executable produced by running $C_1$ on $S$ in a clean process with no Rust compiler or VM in the loop.
- $C_3 = C_2(S, E_{\text{ELF}})$: The Stage-3 standalone Linux `x86_64` ELF compiler executable produced by running $C_2$ on $S$ in the same clean environment.
- $A_1 = C_1(S, E_{\text{lkja}})$, $A_2 = C_2(S, E_{\text{lkja}})$, $A_3 = C_3(S, E_{\text{lkja}})$: The `.lkja` artifact bundles emitted for $S$ by $C_1, C_2, C_3$ respectively.

#### 4.2 Required Fixed-Point Criteria vs. Optional Host Parity

1. **Primary Native Fixed-Point Criterion (Mandatory for Milestone F)** ([S1](https://www.google.com/search?q=https%3A%2F%2Fgcc.gnu.org%2Finstall%2Fbuild.html)):
   $$\text{sha256}(C_2) == \text{sha256}(C_3) \quad (\text{i.e., } \text{bytes}(C_2) == \text{bytes}(C_3))$$
2. **Primary `.lkja` Emission Reproducibility Criterion (Mandatory for Milestone F)**:
   $$\text{sha256}(A_2) == \text{sha256}(A_3) \quad (\text{and } \text{sha256}(A_1) == \text{sha256}(A_2) \text{ when } C_1 \text{ and } C_2 \text{ execute the same } S)$$
   *Note on `.lkja` equivalence*: Distinguish canonical `.lkja` container framing from the instruction stream inside it. Where $C_0$ (Rust) and $C_2$ (`lkjscript`) use different lowering algorithms (for example, $C_0$ runs an optional rewrite that $C_2$ omits in its baseline), $\text{bytes}(C_0(P, E_{\text{lkja}}))$ and $\text{bytes}(C_2(P, E_{\text{lkja}}))$ may legitimately differ as long as both pass `.lkja` admission validation and execute to identical observable results and trap codes. Where a pass is designed as an exact canonical port, assert byte equality explicitly.
3. **Optional Host-to-Self Parity ($\text{bytes}(C_1) == \text{bytes}(C_2)$)**:
   - Requiring $\text{bytes}(C_1) == \text{bytes}(C_2)$ demands that the Rust host backend ($C_0$) and the `lkjscript`-authored backend ($S$) use identical instruction selection, register allocation, layout ordering, and runtime stubs.
   - Treat $\text{bytes}(C_1) == \text{bytes}(C_2)$ as an **optional diagnostic check** during early backend porting (Milestones B–D), **not** as a permanent constraint or the definition of self-hosting. Once $C_1$ is compiled from $S$, $C_1$ and $C_2$ already run the same `lkjscript` code generator, so $\text{bytes}(C_2) == \text{bytes}(C_3)$ is the natural self-hosted fixed point.
4. **Determinism & Reproducibility Controls ([S2](https://www.google.com/search?q=https%3A%2F%2Freproducible-builds.org%2Fdocs%2Fdeterministic-build-systems%2F))**:
   - Every emitted `.lkja` and `ELF64` image must be a pure deterministic function of $(S, E)$:
     - Use deterministic ordered maps/sets (`BTreeMap`/`BTreeSet` or canonical sorted tables); never iterate over randomized hash tables when assigning IDs, offsets, or relocations.
     - Zero-initialize all alignment and struct/slab padding bytes explicitly.
     - Exclude host filesystem paths, wall-clock timestamps, process IDs, and uninitialized allocator capacity from emitted bytes.
5. **Epistemic Scope of Fixed-Point Proof ([S3](https://www.google.com/search?q=https%3A%2F%2Fdwheeler.com%2Ftrusting-trust%2F))**:
   - Document accurately that $\text{bytes}(C_2) == \text{bytes}(C_3)$ proves deterministic bootstrap closure and repeatability for $(S, E)$, **not** universal compiler correctness or absence of a Trusting-Trust defect. General compilation capability must be demonstrated by compiling:
     - An independent corpus of positive and negative non-compiler programs $P_1, \dots, P_k$, and
     - A modified compiler source closure $S'$ (containing a deliberate, observable behavioral change or new diagnostic) through $C_2$ and verifying that $C_2(S', E)$ exhibits the new behavior.

### 5. Normative Technical Contracts

#### 5.1 Graph-to-Compiler Input Boundary (`LKJCI1` Schema)

Do not conflate persistent repository packs (`packs/pack_*.lkjp`), runtime memory arenas, and the compiler's input format:

1. **Canonical Compiler Input Schema (`LKJCI1`)**:
   - Define a compact, deterministic binary input format `LKJCI1` representing an immutable source-graph closure (or, in Milestones B–D, a normalized IR module closure):
     - **Header**: 8-byte magic (`"LKJCI001"`), 4-byte flags/profile ID, 4-byte root declaration index, 8-byte total byte length, 8-byte declaration count, 8-byte maximum traversal fuel limit, and 32-byte BLAKE3/SHA-256 content digest.
     - **Tables**: Canonically ordered, bounds-checked tables of module descriptors, type descriptors, declaration headers, and expression/instruction nodes encoded with explicit little-endian fixed-width integers, explicit variant tags, and 0-based table indices (or validated relative byte offsets).
     - **Structural Import Checks (Mandatory Before Semantic Checking)** ([S4](https://www.google.com/search?q=https%3A%2F%2Fcapnproto.org%2Fencoding.html), [S5](https://www.google.com/search?q=https%3A%2F%2Fflatbuffers.dev%2Flanguages%2Frust%2F)):
       - Verify magic, total byte length against actual input buffer size, digest integrity, alignment, table count bounds, and index/offset ranges using checked arithmetic (`checked_mul`, `checked_add`).
       - Enforce a strict **traversal work limit** (maximum visited nodes/edges) so malformed or cyclic inputs cannot cause unbounded loops or stack overflow during decoding.
2. **Role & Sunset of the Rust Exporter**:
   - In Milestones A–E, a small, audited Rust exporter command or test helper may project an accepted `GraphRepository` revision (after pack decompression) into `LKJCI1` bytes.
   - Record this exporter explicitly in the dependency manifest. Once `lkjscript` can emit `LKJCI1` from authored changes or read repository packs directly (Tier 3), the Rust exporter can be retired.

#### 5.2 Region & Memory Model Correctness Rules

Before adding new region operations or claiming performance gains, enforce these soundness invariants:

1. **Representing Compiler Graphs (Ergonomics vs. Necessity)**:
   - Finite compiler graphs, ASTs, and IRs *can* be represented using existing records, integer variant tags, relative pointers, or parallel arrays.
   - New language constructs such as contiguous `region-slice` operations or tagged `choice` cells inside regions should be introduced **when justified by the compiler workload** (e.g., to eliminate verbose worklist/table indexing in Milestones B–E), with explicit soundness tests.
2. **Ownership Semantics for Slice Allocation & Indexed Access**:
   - If implementing `region-alloc-slice(CAP, INIT_VALUE, COUNT)`, `region-slice-get`, and `region-slice-set`:
     - **Element Restriction**: Restrict region slice elements in the initial profile to **duplicable (`Copy`-like) region cell values** (`Unit`, `Bool`, `I64`, same-region `rel-ptr`, `region-slice` descriptors, and flat records/tagged choices composed solely of those types).
     - **No Affine Duplication**: Reject `region-alloc-slice` or `region-slice-get` on affine/unique resource handles (`CapabilityResource`, unique `RegionCap`, or owning heap containers) so repeated initialization or indexed reads can never duplicate an affine resource or bypass drop obligations.
     - **Constant-Time Bulk Reclamation Compatibility**: Because region cells contain only trivial-drop scalars and same-region relative offsets/slices, dropping or resetting the enclosing `region-with` arena remains a sound $O(1)$ bulk operation with no per-element destructor traversal.
3. **Region-to-`Bytes` Bridge & Lifetime Separation**:
   - An immutable `Bytes` value escaping a `region-with` block **must never** borrow raw memory that will be invalidated when the lexical region exits.
   - Use **explicit bounded copying** into an independent owning `ByteBuffer` / `Bytes` allocation when exporting compiled artifacts from a region, or consume/transfer an owned `ByteBuffer`.
   - Distinguish packed byte sequences (`u8` elements in `Bytes` / `ByteBuffer`) from 8-byte `I64` word slices. Specify exact byte length and endianness whenever encoding multi-byte integers into `ByteBuffer`.
4. **Provenance & Stale-Offset Protection**:
   - A generation number stored only inside an arena header cannot detect a stale raw integer offset (`u32`) after the arena is reset and reused, because the raw offset carries no expected generation.
   - Therefore:
     - **Static Guarantee**: Rely primarily on lexical region brand checking (`%R`) so `rel-ptr<%R, T>` and `region-slice<%R, T>` cannot escape their `(region-with (as %R) ...)` scope or be dereferenced with a different region's capability.
     - **Runtime Custody Check**: Where runtime arena slots or capabilities are checked dynamically, store the expected `(arena_id, generation)` inside the **capability token (`RegionCap`)** (and/or tagged pointer representation in the evaluator) and verify `cap.arena_id == slot.arena_id && cap.generation == slot.generation` on every dereference, rejecting wraparound reuse cleanly.
5. **Checked Layout Arithmetic & Addressing Limits**:
   - All slab offset and extent calculations (`header_size`, `count.checked_mul(stride)`, `offset.checked_add(byte_len)`, alignment rounding) must use overflow-checked arithmetic and reject negative counts/indices or extents exceeding the region's reserved capacity (and the `u32` 4 GiB offset ceiling).
   - Note that a 64-byte-aligned slab base does not require 64-byte alignment for every individual cell; align cells to their natural scalar/record alignment (1, 4, or 8 bytes) with explicit zero padding.
6. **Accurate Native Arena Reclamation Claims ([S6](https://www.google.com/search?q=https%3A%2F%2Fman7.org%2Flinux%2Fman-pages%2Fman2%2Fmmap.2.html))**:
   - In the native `x86_64` ELF runtime, restoring an arena bump cursor on `(region-with ...)` exit provides **$O(1)$ logical reuse of scratch virtual memory** within an already mapped arena for non-escaping allocations.
   - Do not claim that cursor restoration immediately reduces OS resident set size (RSS) or eliminates all heap growth for escaping return values (`ByteBuffer`, `List`, `Record`). For a single-shot compiler invocation, a bounded process-lifetime heap for escaping values combined with reusable scratch region arenas for phase-local tables is a clean, honest architecture.

#### 5.3 Minimal Native Compiler Binary Interface

Equip the standalone native `x86_64` ELF compiler executable with a simple, unambiguous Unix filter contract before considering complex filesystem or daemon interfaces:

1. **Standard Stream & Exit Status Contract**:
   - **Input (`stdin`, fd `0`)**: Reads a single bounded binary payload (up to a configured maximum, e.g., 16 MiB) using a loop over Linux `read(0, buf, rem)` that properly handles partial reads (`0 < n < rem`) and `EINTR` retries until EOF (`n == 0`) or buffer-limit overflow ([S7](https://www.google.com/search?q=https%3A%2F%2Fman7.org%2Flinux%2Fman-pages%2Fman2%2Fread.2.html)).
   - **Mode Selection**: Controlled explicitly by entry target or a small fixed command-line selector (e.g., mode `0` = emit `.lkja` raw bytes, mode `1` = emit `ELF64` raw bytes), never inferred ambiguously by sniffing output bytes.
   - **Output (`stdout`, fd `1`)**: Buffers the complete validated output artifact in memory first; only after compilation succeeds with zero errors, writes the exact artifact bytes to `stdout` using a loop over `write(1, buf, rem)` handling partial writes and `EINTR`, and exiting non-zero on `EPIPE` / write error ([S8](https://www.google.com/search?q=https%3A%2F%2Fman7.org%2Flinux%2Fman-pages%2Fman2%2Fwrite.2.html)). Never emit partial artifact bytes before failure.
   - **Diagnostics (`stderr`, fd `2`)**: On any input, validation, semantic, or resource error, writes a deterministic diagnostic code/message to `stderr` (with empty `stdout`) and exits with a documented non-zero code (`64` for usage/argument error, `65` for input/validation/semantic rejection, `70` for runtime trap/quota exhaustion).
2. **Launcher & Atomic File Publication**:
   - During Milestones C–F, the host test harness or `lkjscript` CLI launcher may feed the input stream, capture `stdout` into a sibling staging file, `fsync` the file, set mode `0755` (for ELF outputs), atomically rename to the destination path, and `fsync` the parent directory.
   - Keep this staging/launcher role explicit in the toolchain documentation rather than adding raw path `openat` calls to the compiled program without capability checks.
3. **Scope of `#![forbid(unsafe_code)]` ([S9](https://www.google.com/search?q=https%3A%2F%2Fdoc.rust-lang.org%2Frustc%2Flints%2Flisting%2Fallowed-by-default.html%23unsafe-code))**:
   - Preserve `#![forbid(unsafe_code)]` across all Rust crates. Recognize that safe Rust emitting machine code into an ELF file does not automatically make the emitted machine code memory-safe: the emitted `x86_64` instructions must be verified via 16-byte System V ABI stack alignment checks, bounds checks, trap tests, and multi-engine execution equivalence.

#### 5.4 Effect Summaries, Quota Semantics & Optimization Legality

Keep these concerns properly separated and sequenced:

1. **Conservative Static Effect/Resource Summaries (Optional Until Milestone G)**:
   - Any static resource analysis must use a sound abstract domain with an explicit **`Unknown` / `DynamicMetered`** top element for general recursion or data-dependent loops. Do not claim exact polynomial bounds for arbitrary programs.
2. **Quota Semantics & Error Precedence**:
   - Clearly document whether native ELF execution runs un-metered (bounded by OS process limits and input-size caps, as currently specified in [docs/spec/native-elf.md](https://www.google.com/search?q=https%3A%2F%2Fraw.githubusercontent.com%2Flkjsxc%2Flkjscript%2Fmain%2Fdocs%2Fspec%2Fnative-elf.md)) or under an explicit step/byte counter.
   - If lowering quota checks into native code, do not hoist whole-block charges across potentially trapping instructions in a way that changes observable error precedence relative to the reference evaluator unless the specification explicitly permits block-granular metering.
3. **Sequential First; Parallelism Only With Full Interference Proofs**:
   - Keep compilation and execution strictly sequential for the self-hosting path (Milestones A–F).
   - Note that disjoint region write sets alone do not establish equivalence with sequential evaluation when expressions can trap, diverge, consume shared quota budgets, or raise peak memory. Defer parallel compilation to Milestone G (and prefer explicit coarse-grained parallel compilation of independent units with deterministic result ordering over automatic expression-level parallelization).
4. **Trap-Preserving Optimizer Legality ([S10](https://www.google.com/search?q=https%3A%2F%2Fweb.ist.utl.pt%2Fnuno.lopes%2Fpubs.php%3Fid%3Dalive2-pldi21))**:
   - Because `lkjscript` `I64` arithmetic traps on signed overflow and division/modulo by zero, general ring identities such as $(a + b) + c \to a + (b + c)$ or $0 \times e \to 0$ are **unsound** when $a + b$ or $e$ can trap.
   - Every rewrite rule in `egraph_opt` (or any self-hosted optimizer) must prove that the rewritten expression preserves exact trap behavior and evaluation order on all inputs, backed by negative counterexample tests (e.g., `I64::MAX + 1 - 1`).

### 6. Two-Level Verification Oracle & Acceptance Architecture

Every compiler slice must be tested at **two distinct levels**:

#### 6.1 Level 1 — Cross-Engine Compiler Execution Agreement

Compare the behavior of the compiler implementation across available execution paths on the same input $P$:

| Path ID | What Executes | Input | Output Artifact | Purpose |
| :--- | :--- | :--- | :--- | :--- |
| **Path R (`Rust`)** | Host Rust compiler (`src/platform/compiler/`) | Authoritative graph / `LKJCI1` for $P$ | $A_{\text{Rust}}(P)$ | Reference host baseline (investigate discrepancies against the spec; do not treat host bugs as normative) |
| **Path V (`VM`)** | `lkjscript` compiler running on the bytecode VM | Same `LKJCI1` for $P$ | $A_{\text{VM}}(P)$ | Verifies `lkjscript` compiler logic under metered VM evaluation |
| **Path I (`Ref`)** | Same `lkjscript` compiler running on the canonical reference interpreter | Same `LKJCI1` for $P$ | $A_{\text{Ref}}(P)$ | Verifies `lkjscript` compiler logic on the disjoint reference evaluator (must satisfy $\text{bytes}(A_{\text{VM}}(P)) == \text{bytes}(A_{\text{Ref}}(P))$) |
| **Path N (`Native`)** | Standalone native Linux `x86_64` ELF executable of the `lkjscript` compiler ($C_1, C_2, C_3$) | Same `LKJCI1` for $P$ | $A_{\text{Native}}(P)$ | Verifies native codegen of the compiler itself (must satisfy $\text{bytes}(A_{\text{Native}}(P)) == \text{bytes}(A_{\text{VM}}(P))$) |

*Important Epistemic Note*: Paths V, I, and N execute the **same `lkjscript` compiler source**, so agreement among V, I, and N proves evaluator/backend consistency, not independent compiler design. That is why **Level 2** is mandatory.

#### 6.2 Level 2 — Produced Artifact Validation & Behavioral Execution

For every emitted artifact $A \in \{A_{\text{VM}}(P), A_{\text{Native}}(P)\}$ (both `.lkja` and `ELF64` formats):

1. **Structural & Admission Validation**:
   - Pass `.lkja` outputs through the strict artifact bundle validator/loader.
   - Pass `ELF64` outputs through structural ELF header/segment inspection.
2. **Behavioral & Trap Execution Equivalence**:
   - Execute the emitted `.lkja` (on VM and Reference) and the emitted `ELF64` executable (as an isolated child process) across:
     - Positive inputs covering all supported types, control flows, closures, tail loops, and regions;
     - Boundary arithmetic inputs (`I64::MIN`, `I64::MAX`, `-1`, `0`, overflow/div-zero trap cases);
     - Negative compiler inputs (malformed `LKJCI1` headers, out-of-bounds indices, cyclic types, type mismatches, affine double-use, region escape attempts) asserting deterministic rejection.
   - When any mismatch occurs, preserve a minimized reproducible input fixture in `tests/fixtures/` before fixing the bug.

### 7. Verification Provenance, Disk Discipline & Per-Turn Execution Loop

#### 7.1 Verification Provenance & Commit Ordering

1. **Discover Actual Verifier Plan**:
   - Do not hard-code gate counts such as `29/29`. Discover the active verifier plan and gate inventory via `lkjscript-dev` (`tools/bootstrap-verifier.sh` and `lkjscript-dev check`) and report the actual observed gate counts, pass/fail status, and fresh/reused observations honestly.
2. **Verify the Exact Tree You Commit**:
   - **Never edit tracked files (including `docs/status.md`, generated docs, or compacted packs) after running acceptance checks and then commit without re-verifying**, unless an explicitly documented verifier contract excludes that file from the verified input digest.
   - Update `docs/status.md` and any generated discovery files *before* running the final `check changed --machine` / `check full --fresh --machine` pass for the commit, so the committed Git tree matches the verified input snapshot bit-for-bit.
3. **Honest Status States**:
   - In `docs/status.md` (kept concise, `< 80 lines`, while keeping detailed contracts in `docs/spec/`), record the actual verification state (`not_run`, `running`, `failed`, `passed`, `stale`, or `blocked`), distinguishing the **Latest Attempt** from the **Last Accepted Commit**.

#### 7.2 Ownership-Aware Disk & Resource Discipline

1. **Explicit Storage Accounting & Headroom**:
   - Measure workspace disk usage before and after major builds using explicit error-checked commands (avoid `|| true` pipelines that mask Git or filesystem errors):
     ```sh
     git status -sb
     df -h .
     du -sh target .artifacts 2>/dev/null
     ```
   - Target keeping the active repository + `$PWD/target` footprint below **8 GiB** (with ~6 GiB as a soft warning threshold for `$PWD/target` to leave 2 GiB of transient peak headroom during linking and 3-stage bootstrap comparisons). If the baseline environment already exceeds this budget due to external files, report the baseline accurately rather than deleting files outside your ownership.
2. **Ownership-Aware Cleanup (No Blind Globbing)**:
   - **Never** run blind wildcard deletions such as `rm -rf /tmp/lkj*` or delete a `verifier_root` that may belong to another active process or retained release handoff.
   - Track only the temporary directories, stage binaries (`C1`, `C2`, `C3`), and scratch files created by your current task PID/session. Record their SHA-256 digests and sizes in the acceptance receipt first, then remove transient stage binaries once their comparison is complete.
   - Retain compact failure logs or minimized reproducers (`< 64 KiB`) when a gate fails so diagnostic evidence is never erased before debugging.
3. **Measured Cargo Profile & Repository GC Policy ([S12](https://www.google.com/search?q=https%3A%2F%2Fdoc.rust-lang.org%2Fcargo%2Fcommands%2Fcargo-clean.html), [S13](https://www.google.com/search?q=https%3A%2F%2Fdoc.rust-lang.org%2Fcargo%2Freference%2Fprofiles.html))**:
   - Keep fat LTO out of iterative builds (`lto = false` or `lto = "off"` as appropriate for the profile) and use a single shared `$PWD/target`. Note that `cargo clean -p lkjscript` cleans only package-specific artifacts; if multi-profile accumulation in `$PWD/target` exceeds the soft threshold, clean stale test binaries selectively between milestones.
   - Run `lkjscript gc --project <PATH>` only when persistent disk repositories accumulate unreachable packs, ensuring required roots and active readers are respected and sufficient free space exists for the new pack before replacement.

#### 7.3 First Concrete Slice & Continuous Autonomous Loop

1. **Immediate First Slice (Start Here)**:
   - **Step A1**: Inspect `git rev-parse HEAD`, `git status -sb`, `AGENTS.md`, `docs/status.md`, `docs/spec/regions.md`, `docs/spec/native-elf.md`, and `src/platform/compiler/`. Verify which claims from the supplied `b83b6dd` snapshot hold on the live checkout.
   - **Step A2**: Define the minimal **Canonical Compiler Input Schema (`LKJCI1`)** and implement a small, deterministic **Milestone B `lkjscript` compiler slice** (for example, a native `lkjscript` module that takes an `LKJCI1`-encoded normalized function closure, emits a valid `.lkja` or standalone `x86_64` ELF64 binary on the VM and reference interpreter, and passes Level 1 + Level 2 execution checks).
2. **Between-Turn Continuation Policy**:
   - Work on a clean branch or directly on `main` according to the repository's normal verified workflow (never force-push or destroy unrelated worktrees).
   - After each verified commit, check remaining budget and immediately begin the next dependency-justified slice along **Milestones B → C → D → E → F → G**.
   - Pause and report to the Owner **only** if: (a) Milestone F is completely verified, (b) the execution/token budget is exhausted, or (c) a genuine external blocker cannot be resolved within repository authority.

## Deliverable 3: Epistemic Classification & Evidence Table

| Item / Claim | Category | Current Epistemic Status | Required Action by Executing Agent |
| :--- | :--- | :--- | :--- |
| **Authoritative Meaning Graph (`GraphRepository`) as sole editable program authority** | **Owner Requirement** | Binding architectural rule ([AGENTS.md](https://www.google.com/search?q=https%3A%2F%2Fraw.githubusercontent.com%2Flkjsxc%2Flkjscript%2Fmain%2FAGENTS.md), [docs/direction.md](https://www.google.com/search?q=https%3A%2F%2Fraw.githubusercontent.com%2Flkjsxc%2Flkjscript%2Fmain%2Fdocs%2Fdirection.md)) | Preserve across all milestones; compiler self-hosting targets the meaning graph ($S$), not surface text files. |
| **Long-term self-hosting & native `x86_64` Linux execution** | **Owner Requirement** | Binding project trajectory ([docs/direction.md](https://www.google.com/search?q=https%3A%2F%2Fraw.githubusercontent.com%2Flkjsxc%2Flkjscript%2Fmain%2Fdocs%2Fdirection.md), [docs/roadmap.md](https://www.google.com/search?q=https%3A%2F%2Fraw.githubusercontent.com%2Flkjsxc%2Flkjscript%2Fmain%2Fdocs%2Froadmap.md)) | Reach Tier 2 Semantic Compiler Self-Hosting emitting both `.lkja` and Linux `x86_64` ELF64. |
| **Single current-format policy (no legacy fallback readers or compatibility shims)** | **Owner Requirement** | Binding policy ([docs/campaigns/202609281300.md](https://www.google.com/search?q=https%3A%2F%2Fraw.githubusercontent.com%2Flkjsxc%2Flkjscript%2Fmain%2Fdocs%2Fcampaigns%2F202609281300.md), [AGENTS.md](https://www.google.com/search?q=https%3A%2F%2Fraw.githubusercontent.com%2Flkjsxc%2Flkjscript%2Fmain%2FAGENTS.md)) | Maintain explicit current-format magic/version/checksum validation and clean rejection; do not keep legacy format readers. |
| **Strict Rust safety & lint gates (`#![forbid(unsafe_code)]`, denied `unwrap`/`expect`/`panic`)** | **Owner Requirement** | Binding repository invariant ([Cargo.toml](https://www.google.com/search?q=https%3A%2F%2Fraw.githubusercontent.com%2Flkjsxc%2Flkjscript%2Fmain%2FCargo.toml), [AGENTS.md](https://www.google.com/search?q=https%3A%2F%2Fraw.githubusercontent.com%2Flkjsxc%2Flkjscript%2Fmain%2FAGENTS.md)) | Keep all lints green and verify emitted machine code independently ([S9](https://www.google.com/search?q=https%3A%2F%2Fdoc.rust-lang.org%2Frustc%2Flints%2Flisting%2Fallowed-by-default.html%23unsafe-code)). |
| **3-stage bootstrap fixed-point criterion (`bytes(C2) == bytes(C3)`)** | **Verified External Fact** | Standard compiler bootstrap criterion ([S1](https://www.google.com/search?q=https%3A%2F%2Fgcc.gnu.org%2Finstall%2Fbuild.html), [S2](https://www.google.com/search?q=https%3A%2F%2Freproducible-builds.org%2Fdocs%2Fdeterministic-build-systems%2F), [S3](https://www.google.com/search?q=https%3A%2F%2Fdwheeler.com%2Ftrusting-trust%2F)) | Use $\text{bytes}(C_2) == \text{bytes}(C_3)$ as the primary bootstrap gate; keep optional $\text{bytes}(C_1) == \text{bytes}(C_2)$ separate. |
| **Linux syscall partial `read`/`write` & `mmap` lifetime semantics** | **Verified External Fact** | Documented Linux kernel ABI ([S6](https://www.google.com/search?q=https%3A%2F%2Fman7.org%2Flinux%2Fman-pages%2Fman2%2Fmmap.2.html), [S7](https://www.google.com/search?q=https%3A%2F%2Fman7.org%2Flinux%2Fman-pages%2Fman2%2Fread.2.html), [S8](https://www.google.com/search?q=https%3A%2F%2Fman7.org%2Flinux%2Fman-pages%2Fman2%2Fwrite.2.html)) | Loop on partial `read`/`write` and `EINTR`; distinguish logical arena cursor reset from OS `munmap`/RSS. |
| **Repository state at commit `b83b6dd5dc235daaef0483d10342988f7712085c` (Phases 1–4: `54c07ec`, `dfc57ac`, `20b031b`, `a9c40eb`, `b83b6dd`)** | **Supplied Claim (Pending Local Verification)** | Supplied from recent repository snapshots; unverified in environments without live checkout access | Verify in **Milestone A Step A1** (`git log`, file inspection, verifier discovery). Adapt if `HEAD` has advanced or differs. |
| **Specific `region-slice` / `choice` cell layouts, hardware arena registers, E-Graph migration order, and auto-parallelism** | **Design Hypothesis** | Replaceable engineering proposals | Implement only when justified by the self-hosted compiler's dependency inventory (Milestones B–E) or measured bottlenecks (Milestone G). |

## Deliverable 4: Summary of Consequential Corrections

1. **Separated Requirements, Verified Facts, Supplied Claims, and Hypotheses (Sections 1 & 2)**:
   - Reclassified the `b83b6dd` repository snapshot as supplied context requiring bounded verification at startup, removed instructions forbidding repository inspection, stripped hyperbolic rhetoric and unmeasured percentages, and replaced all search-wrapped links with direct repository and specification URLs.
2. **Defined Exact Self-Hosting Tiers and Supported Profile $P_{\text{self}}$ (Section 3)**:
   - Explicitly distinguished **Tier 1 (Backend Self-Hosting)**, **Tier 2 (Semantic Compiler Self-Hosting over the authoritative meaning graph)**, and **Tier 3 (Wider Toolchain Independence)**, and required an explicit language profile $P_{\text{self}}$ with deterministic diagnostic rejection of unsupported constructs.
3. **Corrected Bootstrap Fixed-Point Math (`C2 == C3` vs. `C1 == C2`) (Section 4)**:
   - Adopted unambiguous $(S, E, C_0, C_1, C_2, C_3)$ notation, made $\text{bytes}(C_2) == \text{bytes}(C_3)$ (for both `ELF64` and `.lkja`) the primary self-hosting fixed-point gate ([S1](https://www.google.com/search?q=https%3A%2F%2Fgcc.gnu.org%2Finstall%2Fbuild.html)), separated optional $\text{bytes}(C_1) == \text{bytes}(C_2)$ host parity, and added explicit reproducibility controls ([S2](https://www.google.com/search?q=https%3A%2F%2Freproducible-builds.org%2Fdocs%2Fdeterministic-build-systems%2F)) and mutation/general-program tests ([S3](https://www.google.com/search?q=https%3A%2F%2Fdwheeler.com%2Ftrusting-trust%2F)).
4. **Replaced Rigid Phase Chain with Dependency-Driven Milestones A–G (Section 5)**:
   - Reordered the roadmap so the agent reaches a working end-to-end compiler component in `lkjscript` early (**Milestone B**) and runs it natively (**Milestone C**), using that actual compiler workload to pull in only necessary region/runtime/I/O features while deferring `egraph_opt` migration and parallelization to **Milestone G**.
5. **Specified Normative Boundary, Region, I/O, Quota, and Verification Contracts (Sections 6–14)**:
   - Defined the `LKJCI1` binary input schema with bounds and traversal-fuel checks ([S4](https://www.google.com/search?q=https%3A%2F%2Fcapnproto.org%2Fencoding.html), [S5](https://www.google.com/search?q=https%3A%2F%2Fflatbuffers.dev%2Flanguages%2Frust%2F)); fixed region slice ownership (restricting repeated/indexed cells to non-affine duplicable types), `Bytes` lifetime separation (bounded owning copies), and capability-carried generation provenance; defined the partial-read/write-safe `stdin`/`stdout`/`stderr` binary filter contract ([S7](https://www.google.com/search?q=https%3A%2F%2Fman7.org%2Flinux%2Fman-pages%2Fman2%2Fread.2.html), [S8](https://www.google.com/search?q=https%3A%2F%2Fman7.org%2Flinux%2Fman-pages%2Fman2%2Fwrite.2.html)); required pre-acceptance status updates so committed trees match verified trees; and replaced blind `/tmp/lkj*` deletion with ownership-aware cleanup.
