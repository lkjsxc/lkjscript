# JIT Hook

## Purpose

Describe the stub reserved for later native codegen.

## Contract

`JitHook::maybe_compile` is invoked on calls. `NullJit` always returns false.
Real JIT is deferred until after Mandelbrot and TUI demos.
