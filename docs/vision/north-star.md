# North Star

## Purpose

State the mission for the `lkjscript2026` runtime.

## Mission

Ship a tiny, cache-friendly functional language that weak AI models can author
reliably, with a real bytecode VM ready for later JIT, and an ecosystem grown
in `.lkjscript` rather than host frameworks or third-party Rust crates.

The long horizon:

- **Scratch host** — own the low-level OS surface; avoid crates.io dependencies.
- **Libraries in `.lkjscript`** — termios, sockets, buffered IO, and apps live as
  script libraries on a thin syscall-shaped primitive layer, not as fat Rust
  feature wrappers.
- **Eventual speed** — interpreter correctness first; baseline JIT and adaptive
  specialization later so the same programs can become extremely fast without
  rewriting product logic into Rust frameworks.
