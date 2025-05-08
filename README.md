# lkjscript

## Introduction

lkjscript is a simple scripting language. It features a custom compiler that transforms lkjscript code into compact bytecode, which is then executed by a lightweight virtual machine. This project is designed to explore the fundamentals of compiler and interpreter design.

## Features

*   Control flow statements are limited to `if`/`else` and `loop`.
*   Pointer support with `&` (address-of) and `*` (dereference) unary operators.
*   Built-in functions for basic I/O (`_read`, `_write`) and process control (`_usleep`).
*   Comments using `//`.
*   Single-pass compilation to custom bytecode.
*   Stack-based virtual machine for bytecode execution.
*   Compile and run within a Docker container.
*   No global variables.
*   No concept of lvalues or rvalues. (Example of assigning the value of `a` to `b`: `&b = a`)
*   The unary `&` operator evaluates a variable to its address. It cannot be applied to expressions.
*   Only 64-bit signed integers are supported.
*   Character and string literals are not currently supported.
*   All functions and `loop` constructs must return a value.
*   Expressions are delimited by newlines.
*   Semicolons are not supported.

## Usage

```bash
git clone https://github.com/lkjsxc/lkjscript
cd lkjscript
docker build -t lkjscript .
docker run --rm -v ./src/lkjscriptsrc:/data/lkjscriptsrc lkjscript
```
Your lkjscript code should be placed in a file named `lkjscriptsrc` within the `src` directory (i.e., `src/lkjscriptsrc`). The `docker run` command mounts this file into the container, and the interpreter expects to find the file at that location.