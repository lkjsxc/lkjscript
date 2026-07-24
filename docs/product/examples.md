# Example And Validation Entries

## Purpose

Classify every root compiled by the source gate so demos, fixtures, and product
workloads do not become unexplained stale surfaces.

## Status

**Current.** The root list is still encoded in xtask; moving it to a checked
manifest is an **Accepted Target**.

## Executable Workloads

| Entry | Role | Runtime acceptance |
| --- | --- | --- |
| `src/examples/hello/main.lkjscript` | factorial/output smoke | exact output `3628800` |
| `src/examples/mandel/main.lkjscript` | numeric/list/render workload | non-empty deterministic shape |
| `src/examples/http/hello.lkjscript` | one-connection TCP/HTTP smoke | curl response contains `ok`, then exits |
| `src/examples/bench/main.lkjscript` | Leibniz diagnostic entry | numeric result checked by benchmark tooling |
| `src/examples/brainfuck/main.lkjscript` | direct Brainfuck interpreter, with optional identical-run folding | authored smoke fixtures and pinned Mandelbrot output checked byte-for-byte by `meta/benchmarks/brainfuck/benchmark.py` |
| `src/examples/bulk-bytes/main.lkjscript` | exact UTF-8 file-buffer round trip | smoke writes, rereads, decodes, and prints exact text |
| `src/examples/lkjedit/main.lkjscript` | full terminal editor acceptance | scripted open/edit/save/reopen/new-file smoke |
| `src/examples/lkjedit/buffer-demo.lkjscript` | terminal redraw/list demonstration | compile coverage only |
| `src/examples/lkjedit/edit-mem.lkjscript` | editor loop with in-memory buffer | compile coverage only |
| `src/examples/lkjedit/hello.lkjscript` | banner and one-byte input demonstration | compile coverage only |

The former `vimlike.lkjscript` entry was byte-for-byte identical to
`main.lkjscript` and was removed rather than retained as a redundant demo.

## Library Validation Roots

These definitions are not standalone product applications. They are compiled
as roots so no in-tree source remains syntax-only:

- `src/std/io/now-ms.lkjscript`
- `src/std/io/wait.lkjscript`

## Lifecycle Rule

Every added root needs a role, expected behavior, and gate. A duplicate or
historical demo is removed unless it exercises a distinct contract. Compile-only
entries should gain runtime assertions when they protect behavior rather than
only source reachability.
