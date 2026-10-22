# Ordinary execution without default cumulative quotas

Date: 2026-10-22. Baseline: 2144fc8012664096899d3585a8c12343139f7ee3.

## Decision and reconciliation

Complete the existing work/default-cumulative-execution-20261022 slice before
changing compiler representations. Its twelve modified files already implement
ordinary unmetered cumulative policies and focused controls. Reconciliation found
one checkout, no active build or verification jobs, no untracked source and an
unchanged index. Preserve and review that work; regenerate public discovery pages
through the product owner and add this decision and current status.

Remove the ordinary defaults of 10,000,000 instructions, 268,435,456 cumulative
modeled allocated bytes, 1,000,000 collection items and 100,000 capability calls.
RunPolicy::default(), its named unmetered constructor, NormalizedRunPolicy::default()
and its named foreground constructor select None for all four dimensions.
Ordinary source execution and differential batches therefore use the same policy
as trusted command execution.

This slice removes default policy quotas. It does not remove semantic admission,
physical allocation failure, cancellation, deadlines, exact deployment grants,
or implementation limits on call depth, operand stack and individual values.
Removing recursive evaluator protections requires a separate traversal audit and
implementation. A general user program may diverge without an explicit execution
quota or cancellation. No synthetic compiler-pass fuel is removed here.

## Descriptor and observation contracts

instruction_fuel remains a required positive integer or null within an explicit
execution object. The three optional cumulative fields impose no quota when
omitted or null. Positive numbers retain their exact selected limits.
The two live-state fields remain required positive integers; resident routes
still require explicit execution and runtime objects.

Omission no longer restores historical implicit quotas. This is an intentional
current-format policy compatibility cut without a legacy reader. Encoding
materializes all selections, including null, and round trips preserve absence
and explicit numbers. No accepted program graph, artifact, capability grant or
application data is rewritten.

Cumulative quota enforcement uses checked addition and compares the complete
charge with Some(maximum) before permitting work. Overflow under an explicit
quota refuses, including Some(u64::MAX). An absent quota uses saturating
observation only: u64::MAX then means a lower bound. These counters do not size
buffers, prove offsets valid, authorize grants or replace checked storage arithmetic.
Discovery describes observed instruction/expression counters by their actual u64
domain and live frame observations by the host usize domain, rather than claiming
the removed execution defaults are observation maxima.

## Independent verification

Focused controls compare policy defaults with independent literal expectations.
They admit work above each former threshold, preserve exact-fit/one-more refusal
for explicit quotas, and distinguish saturated observation from quota overflow.
Descriptor tests cover omitted fields, each explicit null, integer extremes,
invalid or missing fields and materialized round trips.

The two evaluator control test preserves differential equality and independent
capability grants, per-dimension explicit refusal and cancellation. The public
resident-policy test authors a counting program, executes ordinary source with
no override, then removes its authoring project and checks detached execution.
Both routes return 1,000,000; source execution also requires canonical-reference
equality and unchanged accepted HEAD. The production witness observes:

| Observation | Measured value | Former default |
| --- | ---: | ---: |
| Instructions | 11,000,010 | 10,000,000 |
| Modeled cumulative allocated bytes | 649,351,110 | 268,435,456 |
| Maximum call depth | 1 | 4,096 retained |
| Live call frames after completion | 0 | Not a cumulative quota |

These allocation observations are not allocator-request totals, live retained
heap, mapped arena capacity or RSS. The fixture independently checks explicit
HTTP quota refusal, deadline cancellation, successful health after each failure
and joined shutdown. Invalid configuration rejects before artifact, secret or
listener access. Offline effect probes require null for unselected quotas. The transported
foreground matrix selects all six fields explicitly for its bounded comparison;
its independent quota expectations do not depend on RunPolicy::default(). Thus
neither fixture reinstalls historical defaults implicitly or weakens bounded controls.

The completed focused run passed 5 control tests, 37 deployment tests, the
two-evaluator policy test and 3 public policy tests. The existing focused backend
run passed 113 selfhost tests and 10 native ELF controls; its optional measurement
test remained ignored. Logs are under .artifacts/selfhost/default-execution.
Those focused results do not replace exact staged-tree bootstrap, fresh full
acceptance and release-target inspection. For this slice's src/ and tests/
changes, changed selection widens to the same 29 gates as full. One complete
fresh full run covers that selection without duplicating the same expensive
workspace harnesses. The complete publication receipt is
.artifacts/lkjscript-dev/latest-receipt.json.

The first frozen candidate completed all gates: 28 passed and offline_packages
failed. Its bounded foreground fixture constructed RunPolicy::default(), so the
independent literal expectation correctly detected that it no longer selected
quotas. The fixture now selects all six fields explicitly; the refusal and
effective-policy oracles remain unchanged. The revised candidate requires its
own staged bootstrap and fresh full acceptance.

## Compiler identity and next dependency

No maintained .lkjc proposal or accepted compiler object changes in this slice.
Live owners, retirement history, complete LKJSG001 bytes, mode-1 LKJCI1 units,
historical storage and the source-driven admission frontier therefore remain the
preceding compiler baseline. No compiler speedup, source-driven fixed point or
native phase reclamation is claimed.

The immediate compiler frontier remains the kernel callable-flow work quota and
carrier representation headroom. Audit their whole producer-consumer chains
before selecting the smallest complete cutover. Preserve complete-carrier digest
binding and whole-inventory validation while advancing active-profile E2/E3.
