# Owned FIFO consumer

This native workload exercises the standard package's generic two-stack FIFO.
It is not a second queue implementation. `library.lkjc`, `steps.lkjc` and
`trace.lkjc` are admitted and exported before the separate consumer's concrete
`elements.lkjc` and `application.lkjc` requests are authored.

The exact standard dependency owns empty, length, enqueue, dequeue, front and
discard. The small `Element` contract belongs to this observation workload:
create, borrow-observe and consume-finish make representation differences visible
without giving the trace knowledge of the element's storage. The native trace
uses that witness through the imported generic code, including recursive calls.

Operation records contain `kind` and `value`. Kind 0 enqueues; 1 dequeues; 2
observes the front without consuming it; 3 clears the queue. Other kinds trap.
Every accepted operation reports its kind, whether it observed an element,
its observed value and the resulting length. Completion drains the remainder,
checks the empty owner and reuses it for value 37 before draining again.

Cells preserve their signed integer. A buffer contains the two distinct octets
`n` and `255 - n`, and is observed as `256 * first + second`. A packet owns that
buffer plus stamp 31. Buffer inputs must be 0 through 255; the cell-only boundary
also exercises both signed I64 extremes. These deliberately different outputs
make accidental representation substitution observable.

The public command targets are `fifo-cells`, `fifo-buffers`, `fifo-packets`,
their `-batch` forms, `fifo-forwarded`, and the intentionally failing
`fifo-empty-front`. The forwarded target borrows through another native helper,
ends the loan, pops the original queue and consumes the same cell.

[Public regressions](../../tests/public_cli/native_owned_fifo.rs) own fresh
project creation, exact transport staging, native plan/apply, independent full
results, source-free deployment, malformed input, rejected lifetime violations,
finite fuel refusal and joined cleanup. They do not require network or grants.
The full source profile and maintained native-owned public family include them.
The independent oracle enumerates all 781 words of length zero through four
from enqueue-0, enqueue-255, dequeue, peek and clear. Additional long mixed-stack
and deterministic mixed-operation traces cover refill and collection boundaries.
A comparison count is coverage, not a performance score.

Run the focused source regression from the repository root:

```sh
cargo test --locked --test public_cli native_owned_fifo -- --test-threads=2
```

Set `LKJSCRIPT_RETAIN_PRODUCT_EVIDENCE=1` to retain the temporary authored
requests, intermediate observations, rejected inputs and selected executable
copies. Source-free checks deliberately remove only their owned source projects
and transports; the final artifact, descriptors and execution evidence remain.
Source acceptance and released availability are recorded separately in
[status](../../docs/status.md). The [library guide](../../docs/guides/native-owned-fifo.md)
defines the public representation, lifetime and transfer-cost limits.
