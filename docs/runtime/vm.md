# VM

## Purpose

Explain the execution engine.

## Shape

- Tagged `u64` values; heap objects in a bump arena
- Dense opcode stream; register-ish locals in contiguous frames
- Host builtins for print, byte IO, thin file FDs, argv, wait, and TTY raw/poll
- Closures reference function prototypes in the chunk
