# Ordinary versus index-populated cache attribution

This is a separate predeclared condition, not a revision of the ordinary
campaign. It measures whether adding a semantic index changes a later ordinary
one-body build's Store work. No paging change, speed milestone, cache policy
change or million-line run is included.

## Fixed identities and limits

Use the frozen instrumented producer
82d85b1a16818c391f194f0f956a32d077a6c9639452841c86715d04c07c68f7
(31,727,173 B). Its final Store/driver hashes and common-emitter construction are
in compiler-cache-io.meta and compiler-cache-io.setup.log. The common emitter
71b2fc066cd74e9e37cb3134173903c9b322b83472b9293f1a49df7c1d18f3b7
constructs the generator and applies the checked body transaction. No executable
from the concurrent no-create-read experiment is used.

Freeze compiler census, generator source and physical library/runtime to commit
b7e8ed035d155e75438406c7474a747220ff9537. Generate exactly 10,000 then, if the
predeclared resource guard fits, 100,000 lines at seed 20261116. Each rung uses
one generated source path, one private key and one instrumented producer. All
ordinary outputs explicitly use inline0. Both A and B have a fixed 2 GiB store
capacity, distinct from the earlier ordinary campaign's 1 GiB capacity.

Run one worker. Every compiler/generator/copy command has an external 4 GiB
RLIMIT_AS and a 900 s watchdog. Require 8 GiB free disk before setup and each
rung. These are host experiment limits, not language resource policy. Preserve
every attempt, refusal and partial row; do not repeat timings or relax a refusal.

## Fixed sequence per rung

1. Generate the pristine tree, then an ordinary cold build into empty store A.
2. Close that command and copy A to B, including its authenticated budget.
3. Hash A's small preexisting ordinary entry set in both stores and require an
   exact copy. This verification is outside timed compiler work.
4. Build the same pristine source with --index using B only. Require output
   bytes identical to the cold A output.
5. Hash A's preexisting ordinary entry set in both stores again. Record every
   difference without editing either store. This distinguishes pure added index
   population from an index build that also changes prior ordinary payloads.
6. Apply one checked replace-body edit::target transaction, xor1 to xor2, through
   the common emitter and a separate fresh operation store.
7. Build that changed source against A, then B, once each in that fixed order.
8. Build the same changed source with --no-store. Require all three changed
   output images to agree and to differ from the cold image.
9. Only after the last timed command, collect bounded file count/byte totals by
   fixed namespace family for A and B; then remove the generated rung.

The preexisting entry hash pass reads only ordinary records named in A, not
B's added semantic pages. Both sides receive the same explicit ordinary-record
prewarming. The copy, index population, manifest reads, body transaction, fixed
A-before-B order and host activity still affect caches; no file-cache reset or
randomization is claimed. All generated source digest manifests are recorded
after generation/publication, not before each timed build. Store byte totals
use directory metadata after each measurement and do not read payloads.

If indexing changes a prior ordinary entry, retain the observation and label the
condition accordingly; do not call the resulting comparison a pure count-of-files
intervention. The original source/key/producer/selector identities remain the
same, and all image controls still apply.

## Predeclared 100k guard

After the 10k index observation, predict 100k peak RSS from its recorded RSS times
the historical observed ratio 2,670,368/370,004, then multiply by 1.20. Both
historical values are KiB from compiler-scale-final.tsv's 100k/10k index rows.
Require that prediction at most 3,670,016 KiB (3.5 GiB).

Also require 10k index launch wall times 10×1.50 at most 900,000 ms, and 10k
post-index store bytes times 10×1.20 at most 2,147,483,648 B. Record the exact
predictions/decision before generating 100k. Any failed guard makes 100k explicitly
not run; the 10k observations remain. These are extrapolations, not observations.
RSS does not bound virtual address space; even a fitting prediction may receive
an actual 4 GiB allocation refusal, which is retained without retry.

## Timing and attribution validity

Every command records launch-inclusive wall, GNU wall/user/kernel/peak RSS,
paired three-component loads, exact producer/source/output identities and the
store scope. Copy, generation, index population and the transaction are
preparation rows, separate from body A/B.

Use the unchanged Store counter definitions from compiler-cache-io.protocol.md:
accepted authenticated payload/envelope bytes, decode/reseal/retain work and
disjoint timing intervals; no claim of physical syscall bytes. Require a single
summary, zero global failures/overflow, no pending checkpoint, exact phase sums
and envelope-byte arithmetic for all successful instrumented stored rows.

For cold A and body A/B, assigned Store time must fit the existing driver
store_io phase. Index population can legitimately perform Store I/O inside
semantic_index; validate its accounting against the combined disjoint driver
store_io and semantic_index phases and retain both separately. Do not force that
preparation row into store_io alone or change the previous campaign's parser.

No timing ratio establishes a causal optimization gain. This condition can
identify collateral index-header/budget work or whole ordinary payload traffic;
it does not measure a page-migration speedup, explain a different historical
cohort by identity, or establish a backend floor.
