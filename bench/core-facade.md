# Core source facade, ranges and phantom ownership

The ordinary source library supplies a core namespace, checked value wrappers, integer limits and borrowed iterator adapters. Frontend metadata preserves nominal PhantomData and exact method applications while reusing the checked IR contract.

## Source contracts and limits

An unshadowed external core namespace resolves to the existing source library declarations; lexical modules, aliases and generic binders retain their identities.

The source facade exports borrow, cmp, fmt, iter, marker, mem, num, option, result, slice, str and a prelude. Its ops alias exposes range contracts only in this form.

Borrow/BorrowMut project the same value; cmp::min/max use Ord; mem::drop/replace/take reuse checked ownership, with Default required by take.

NonZeroUsize::new/get use a private checked usize value, with Copy/Clone/comparison/Display/Debug contracts.

Direct MIN/MAX spellings for u8/u16/u32/u64/usize/i8/i16/i32/i64/isize resolve to typed ordinary source constants; shadowing and exact MAX/MIN reference tokens are preserved.

SaturatingArithmetic::saturating_add for those ten integer types composes checked addition with the appropriate source limit.

Bound and all six range forms are ordinary source values. RangeBounds uses same-element PartialOrd; inclusive iteration tracks exhaustion and integer iteration retains checked arithmetic.

core::slice::Iter names the shared source slice iterator. Copied<Iter<'a,T>> requires Copy and Cloned<Iter<'a,T>> invokes Clone while retaining the source borrow.

CharIndices returns UTF-8 byte offsets and scalar values. FromStr admits source implementations with an associated error; the ten integer implementations reuse checked decimal decoding.

Result-context inference retains exact method type arguments under ExprId, implementation and method identity for later instantiation and proof; receiver-only coercion context remains separate.

Authenticated canonical PhantomData<T:?Sized> has unconditional derived Copy/Clone/Eq/PartialEq/Ord/PartialOrd, conditional Send/Sync through T, and a source Default implementation.

PhantomData retains nominal type applications and type-outlives dependencies without storing or destroying T. Existing named [();0] storage has zero bytes and alignment one in both production layouts.

Cold/warm source queries, authored shadow rename, body edits, atomic invalid edits, provider invalidation and authenticated unit boundaries preserve exact source and marker identities.

## Repaired producer provenance

<!-- corrected-core-evidence-v3 sha256:f43176b8ed929dfc346c407aa2daffd9b741a2c8881d41ae8056186032cd3bfe -->

Bootstrap used private core parent b6cffddc6b, its seed 64b1dd2e and initial native bf5d322f. Published cf379914 is the source ancestor, not the bootstrap seed. The ordinary bootstrap logs record no automatic seed bridge. The first native (fd83ea00de8b) and final fixed-point native (fd83ea00de8b) remain separately labeled; validation retains their full identities and the refreshed seed hash.

All 34 primary records are retained: 20 under frozen bf5 with the repaired library, 4 under the first native, and 10 under the final fixed-point native. The two expected nonzero records are Rust E0308 refusals. The 4-case/8-route repair cohort ran in both early and final phases. The early supplement records 76 LKJ routes, 14 Rust compiles and 4 positive Rust runs; its LKJ routes compare two exact libraries under bf5 and remain early-producer evidence. The local same_name_step_setup_positive setup passed 4 LKJ routes and 2 Rust runs before same_name_step_spoof_without_supertraits reached its body refusal. The original same_name_step_spoof (d9e8ff37) remains authenticated failed history outside these fresh counts. Its ten open-MAX traps establish only the LKJ checked-overflow comparison; its two private-helper checks are unpaired with Rust. The exact foreign RangeFrom<Custom> implementation returned 42 on the old library and received the located overlapping trait implementations refusal at corrected range.lkj:152:1. Rust rejects that foreign implementation for E0117 at its authored header; this is a separate boundary, not parity.

The validation bundle binds the actual nested source checkpoint, independent review, completed census, full command audit and producer provenance. The failed b6 gate remains separate from the corrected full gate, standalone regression and eventual hosted CI.


## Validation

Native SHA-256:

    fd83ea00de8b306e44534b50bdee0da47464e2c096a36c2567e2699a2cb1ec86

Seed gzip SHA-256:

    4bbd931edf8422044cb61acfc6c3c4fba580a0d61e7283474ba624b9696f90a3

The [bootstrap](core-facade-data/bootstrap.log) reaches byte-identical
current-emitter generations. Its exact input and observed bridge path are
retained in the provenance section.
The [focused suite](core-facade-data/cases-native.log) passes 51 cases /
102 native/interpreter routes. The independent
[Rust witness](core-facade-data/cases-rust.log) verifies 24
values/output cases and 27 recorded Rust verdicts.The focused Rust route checks 24 values/output cases and 27 explicit verdict controls. Five reference-bearing PhantomData controls are accepted by Rust and conservatively refused here; the remaining 22 refusals match Rust. The separate repair witness has 3 values and 1 matching mismatch refusal at each optimization level. Strict supplemental Rust evidence has ten source-specific refusal compilations, two setup compilations/runs and two original signed compilations/runs. The original declaration-failure spoof is history only; private helper names and checked overflow are not paired.

The combined producer passes these source-command proofs:

- [closed source and core](core-facade-data/source-contracts.log)
- [core facade](core-facade-data/core-query.log)
- [trait method arguments](core-facade-data/method-query.log)

The final producer passes 51 focused cases/102 routes, 93 prior cases/186 routes and 200 core plus 536 source contracts. The separate range repair cohort passes 4 cases/8 routes before and after bootstrap. Exact Rust inputs yield 3 values and 1 E0308 refusal at each of opt0 and opt3. Original Levenshtein tests and 7,239 native corpus rows pass on both early and final producer phases. The separately retained early strict76 LKJ routes compare old/current libraries under frozen bf5; they are not attributed to the rebuilt native. Exact fourteen Rust compilations and four positive runs are retained separately. The exact foreign RangeFrom<Custom> implementation returned 42 on the old library and received the located overlapping trait implementations refusal at corrected range.lkj:152:1. Rust rejects that foreign implementation for E0117 at its authored header; this is a separate boundary, not parity.

The refreshed-seed four-worker [full gate](core-facade-data/full-gate.log) passes in 1271.693 seconds with 6766 fresh case requests, zero reuse and 128 IR mutants with zero findings. The separate [real-program regression](core-facade-data/standalone-real-programs.log) passes eight original Levenshtein tests per engine, eighteen provenance controls and all 7,239 native corpus rows. Producer and seed hashes remain unchanged.

Twelve fresh unchanged-source observations give nom 45->45, DOM 39->39 and Sozu 968->968 classified sites, with 34 prior-unclassified Sozu sites and zero new unclassified diagnostics.
No original large root or nom own test admits. The
[current frontier](../docs/real-programs-current.json), exact source-hashed
reviews and [previous frontier](core-facade-data/previous-frontier.json) retain
original/projected input identity and complete diagnostic accounting.

The checked IR, independent checker, interpreter and runtime are unchanged.
Exact-head gate and witness CI are recorded in the session report after
publication.
