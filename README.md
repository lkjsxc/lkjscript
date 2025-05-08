# lkjscript

## Table of Contents

- [Introduction](#introduction)
- [Features](#features)
- [Usage](#usage)
- [Language Reference](#language-reference)
  - [Syntax Basics](#syntax-basics)
  - [Variables and Scope](#variables-and-scope)
  - [Data Types](#data-types)
  - [Operators](#operators)
  - [Control Flow](#control-flow)
    - [Conditional Execution (`if`, `else`)](#conditional-execution-if-else)
    - [Loops (`loop`, `continue`, `break`)](#loops-loop-continue-break)
  - [Functions (`fn`, `return`)](#functions-fn-return)
  - [Pointers and Dereferencing (`&`, `*`)](#pointers-and-dereferencing--)
  - [Built-in Functions (`_read`, `_write`, `_usleep`)](#built-in-functions-_read-_write-_usleep)
- [Compiler Overview](#compiler-overview)
  - [Tokenization](#tokenization)
  - [Parsing](#parsing)
  - [Semantic Analysis & Symbol Resolution](#semantic-analysis--symbol-resolution)
  - [Bytecode Generation & Linking](#bytecode-generation--linking)
- [Virtual Machine (VM) Overview](#virtual-machine-vm-overview)
  - [Architecture](#architecture)
  - [Instruction Set](#instruction-set)
- [Examples](#examples)
- [Contributing](#contributing)
- [License](#license)

## Introduction

lkjscript is a simple scripting language. It features a custom compiler that transforms lkjscript code into compact bytecode, which is then executed by a lightweight virtual machine. This project is designed to explore the fundamentals of compiler and interpreter design.

## Features

*   **Minimalist Syntax**: Expressions are newline-delimited. No semicolons. Comments with `//`.
*   **Data Type**: Solely supports 64-bit signed integers (`int64`).
*   **Variables**: All variables are locally scoped to functions. No global variables. Implicitly declared on first use.
*   **Pointers**: Supports C-style pointer operations with `&` (address-of variable) and `*` (dereference).
*   **Assignment**: Unique assignment semantics: `&variable = expression`. The left-hand side must be an address.
*   **Control Flow**:
    *   Conditional execution: `if`/`else`.
    *   Loops: `loop` construct with `break <value>` (loop evaluates to this value) and `continue`.
*   **Functions**: User-defined functions with `fn` and `return <value>`. All functions must return a value (implicitly returns 0 if `return` is omitted at the end).
*   **Operators**: Rich set of arithmetic, bitwise, logical, and comparison operators.
*   **Built-in Functions**: Basic I/O (`_read`, `_write`) and process control (`_usleep`).
*   **Compilation Process**: Multi-stage compilation:
    1.  Tokenization
    2.  Recursive Descent Parsing (generates an AST-like node list)
    3.  Semantic Analysis & Symbol Resolution (resolves variable offsets and function addresses)
    4.  Bytecode Generation and Linking
*   **Virtual Machine**: Stack-based VM executes the custom bytecode.
*   **Portability**: Designed to compile and run within a Docker container.
*   **Limitations**:
    *   No character or string literals.
    *   No lvalues/rvalues in the traditional sense; explicit address usage for assignments.
    *   The unary `&` operator can only be applied to variables, not complex expressions.
    *   Logical `&&` and `||` operators currently behave as bitwise operators in the VM.

## Usage

1.  **Clone the repository:**
    ```bash
    git clone https://github.com/lkjsxc/lkjscript
    cd lkjscript
    ```
2.  **Build the Docker image:**
    ```bash
    docker build -t lkjscript .
    ```

3.  **Prepare your lkjscript code:**
    Place your lkjscript code in a file named `lkjscriptsrc` inside the `src` directory (i.e., `src/lkjscriptsrc`).


4.  **Run your script:**
    ```bash
    docker run --rm -v ./src/lkjscriptsrc:/data/lkjscriptsrc lkjscript
    ```
    The `docker run` command mounts your local `src/lkjscriptsrc` file into the container at `/data/lkjscriptsrc`, which is the path the interpreter expects.

## Language Reference

### Syntax Basics

*   **Comments**: Single-line comments start with `//` and extend to the end of the line.
    ```lkjscript
    // This is a comment
    &x = 10 // Assign 10 to x
    ```
*   **Expressions and Statements**: Each expression or statement is typically written on its own line. Newlines act as delimiters.
*   **Blocks**: Code blocks for `if`, `else`, `loop`, and `fn` are enclosed in curly braces `{ }`.
    ```lkjscript
    if *ptr > 0 {
      // Do something
    }
    ```
*   **Semicolons**: Semicolons are not used or supported.

### Variables and Scope

*   All variables are implicitly declared upon their first use.
*   Variables are lexically scoped to the function in which they are defined. There are no global variables.
*   Variables store 64-bit signed integers.
*   Example:
    ```lkjscript
    fn my_func() {
      &a = 10       // 'a' is local to my_func
      &b = &a + 5   // 'b' is also local
      return *b
    }
    ```

### Data Types

*   **`int64`**: The only fundamental data type. It represents a 64-bit signed integer. All values, including results of expressions and memory addresses, are treated as `int64`.
*   **Pointers**: While not a distinct type, pointer concepts are supported through operators. A pointer is simply an `int64` value that holds a memory address.

### Operators

Operators are listed by approximate precedence, from lowest to highest:

1.  **Comma**: `,`
    *   Primarily used to separate arguments in function calls.
2.  **Assignment**: `=`
    *   `&variable = expression`: Assigns the result of `expression` to the memory location of `variable`. The LHS must evaluate to an address.
3.  **Logical OR**: `||`
    *   `a || b`: Logical OR. *Note: Currently implemented as bitwise OR in the VM.*
4.  **Logical AND**: `&&`
    *   `a && b`: Logical AND. *Note: Currently implemented as bitwise AND in the VM.*
5.  **Bitwise OR**: `|`
    *   `a | b`: Bitwise OR.
6.  **Bitwise XOR**: `^`
    *   `a ^ b`: Bitwise XOR.
7.  **Bitwise AND**: `&`
    *   `a & b`: Bitwise AND.
8.  **Equality**: `==`, `!=`
    *   `a == b`: Equal to.
    *   `a != b`: Not equal to.
9.  **Relational**: `<`, `>`, `<=`, `>=`
    *   `a < b`: Less than.
    *   `a > b`: Greater than.
    *   `a <= b`: Less than or equal to.
    *   `a >= b`: Greater than or equal to.
10. **Shift**: `<<`, `>>`
    *   `a << b`: Bitwise left shift.
    *   `a >> b`: Bitwise right shift.
11. **Additive**: `+`, `-`
    *   `a + b`: Addition.
    *   `a - b`: Subtraction.
12. **Multiplicative**: `*`, `/`, `%`
    *   `a * b`: Multiplication.
    *   `a / b`: Division (integer division). Returns `INT64_MAX` (9223372036854775807) on division by zero.
    *   `a % b`: Modulo. Returns `INT64_MAX` on modulo by zero.
13. **Unary**:
    *   `+expression`: Unary plus (no-op).
    *   `-expression`: Arithmetic negation.
    *   `!expression`: Logical NOT. (Parsed but currently **not implemented** in the VM).
    *   `~expression`: Bitwise NOT (one's complement).
    *   `*expression`: Dereference pointer. `expression` must evaluate to a memory address.
    *   `&variable`: Address-of. Yields the memory address of `variable`. Can only be applied to a variable name.
14. **Grouping & Primary**:
    *   `(expression)`: Grouping.
    *   `identifier`: Variable access (evaluates to its value).
    *   `number`: Integer literal (e.g., `123`, `-42`).
    *   `function_name(arg1, arg2, ...)`: Function call.

### Control Flow

#### Conditional Execution (`if`, `else`)

*   `if (condition_expression) { statement_block }`
*   `if (condition_expression) { statement_block_if_true } else { statement_block_if_false }`
*   The `condition_expression` is true if it evaluates to a non-zero value, and false if it evaluates to zero.

```lkjscript
&x = 10
if *x > 5 {
  // This block executes
} else {
  // This block does not
}
```

#### Loops (`loop`, `continue`, `break`)

*   `loop { statement_block }`: Creates an infinite loop.
*   `break expression`: Immediately exits the innermost `loop`. The `loop` construct itself then evaluates to the value of `expression`.
*   `continue`: Skips the rest of the current iteration and jumps to the beginning of the innermost `loop`.
*   All `loop` constructs must eventually evaluate to a value via a `break expression` statement.

```lkjscript
// Example: find the length of a null-terminated sequence in memory
// Assume 'itr' is a pointer initialized to the start of the sequence,
// and 'begin' is the starting address of the sequence.
&len = loop {
  if *itr == 0 {      // Check for null terminator
    break itr - begin // Loop evaluates to the length
  }
  &itr = itr + 1      // Advance pointer
  // 'continue' is implicit here if not breaking
}
```

### Functions (`fn`, `return`)

*   **Definition**:
    ```lkjscript
    fn function_name(param1, param2) {
      // statements
      return return_value_expression
    }
    ```
*   Arguments are passed by value.
*   Functions must explicitly return a value using the `return` keyword. If `return` is omitted at the very end of a function body, the function implicitly returns `0`.
*   **Calling**:
    ```lkjscript
    &result = function_name(arg1, arg2)
    ```

```lkjscript
fn add(a, b) {
  return a + b
}

&sum = add(5, 7) // sum will be 12 (value, not address)
                 // to store, use: &address_for_sum = add(5,7)
```

### Pointers and Dereferencing (`&`, `*`)

*   `&variable`: The address-of operator. It returns the memory address of `variable`.
*   `*expression`: The dereference operator. `expression` must evaluate to a memory address. It returns the value stored at that memory address.
*   These are fundamental for assignment, as lkjscript requires an address on the left side of `=`.

```lkjscript
&x = 0          // x is a variable (memory location)
&ptr = &x       // ptr now holds the address of x

ptr = 100       // Dereference ptr to assign 100 to x
                // This is equivalent to &x = 100

&value = *ptr   // Dereference ptr to get the value of x (100)
                // and store it in 'value'
```

### Built-in Functions (`_read`, `_write`, `_usleep`)
```
&ch = 0
&result = _read(0, &ch, 1)
&result = _write(1, &ch, 1)
&result = _usleep(10000)
```

## Compiler Overview

The lkjscript compiler transforms source code into executable bytecode through several stages:

### Tokenization

*   **Input**: Raw lkjscript source code string from `lkjscriptsrc`.
*   **Process (`compile_tokenize`)**:
    *   Scans the source character by character.
    *   Identifies and creates tokens for:
        *   Keywords (`if`, `else`, `loop`, `fn`, `return`, `break`, `continue`).
        *   Identifiers (variable names, function names).
        *   Integer literals.
        *   Operators (e.g., `+`, `*`, `==`, `&&`, `&`, `*`).
        *   Delimiters (e.g., `(`, `)`, `{`, `}`, `\n`).
        *   Built-in function names (`_read`, `_write`, `_usleep`).
    *   Handles comments (`//`) by skipping them.
    *   Whitespace (spaces) is used to separate tokens but is not tokenized itself (except `\n`).
*   **Output**: A linear stream of `token_t` structures, where each token contains a pointer to its string representation in the source and its length.

### Parsing

*   **Input**: The token stream from the tokenizer.
*   **Process (`compile_parse` and related `compile_parse_*` functions)**:
    *   Employs a recursive descent parser to build an intermediate representation (a list of `node_t` structures, akin to an AST).
    *   `compile_parse_fn`: Parses function definitions.
    *   `compile_parse_stat`: Parses statements within blocks or at the top level.
    *   Expression parsing (`compile_parse_expr`, `compile_parse_assign`, ..., `compile_parse_primary`): Handles operator precedence and associativity to structure expressions correctly.
    *   Generates `node_t` entries that represent operations (e.g., `TY_INST_ADD`), operands (constants, variable tokens), control flow constructs (e.g., `TY_INST_JMP`, `TY_INST_JZ` with temporary label IDs), and structural markers (`TY_LABEL`, `TY_LABEL_SCOPE_OPEN/CLOSE`).
*   **Output**: A list of `node_t` structures representing the program's structure and operations.

### Semantic Analysis & Symbol Resolution

*   **Input**: The `node_t` list from the parser.
*   **Process (`compile_analyze`)**:
    *   Iterates through the `node_t` list.
    *   **Symbol Table Management**: Maintains a symbol table (`mem.compile.map`) to track variables and functions.
    *   **Variable Resolution**:
        *   For nodes representing variable access (`TY_INST_PUSH_LOCAL_VAL`, `TY_INST_PUSH_LOCAL_ADDR`), resolves the variable's token to its stack offset relative to the Base Pointer (BP).
        *   Assigns new offsets for newly encountered local variables within the current scope.
        *   Function arguments are assigned specific negative offsets from BP (e.g., -4, -5).
    *   **Function Call Resolution**: For `TY_INST_CALL` nodes, resolves the function name token to an internal ID (index in the symbol map).
    *   **Scope Handling**: Uses `TY_LABEL_SCOPE_OPEN` and `TY_LABEL_SCOPE_CLOSE` nodes (generated during parsing of functions) to manage variable scopes. When a scope closes, relevant entries in the symbol table are effectively deactivated for subsequent lookups.
*   **Output**: The `node_t` list with variable tokens replaced by their stack offsets and function call tokens replaced by their function IDs.

### Bytecode Generation & Linking

This phase consists of two sub-steps:

1.  **`compile_tobin` (AST to Pre-linked Bytecode)**:
    *   **Input**: The resolved `node_t` list.
    *   **Process**:
        *   Iterates through the `node_t` list.
        *   Translates each node into one or more bytecode instructions (`int64_t` values).
            *   Instruction types (e.g., `TY_INST_ADD`) become their enum values.
            *   Operands (constants, resolved stack offsets) are written as subsequent `int64_t` values.
            *   `TY_LABEL` nodes: Records the current bytecode address in the symbol table against the label's ID.
        *   Initializes global VM registers (IP, BP, SP) in the `mem.bin` memory array.
    *   **Output**: A sequence of bytecode instructions in `mem.bin`, where jump/call targets are still label IDs.

2.  **`compile_link` (Linking)**:
    *   **Input**: Pre-linked bytecode and the symbol table (now containing actual addresses for labels).
    *   **Process**:
        *   Iterates through the generated bytecode.
        *   For instructions with label ID operands (`TY_INST_JMP`, `TY_INST_JZ`, `TY_INST_CALL`), replaces the label ID with the actual bytecode address of that label (retrieved from the symbol table).
    *   **Output**: Final, executable bytecode stored in `mem.bin`.

## Virtual Machine (VM) Overview

The lkjscript VM is a simple stack-based machine that executes the bytecode generated by the compiler.

### Architecture

*   **Memory (`mem_t mem`)**: A single large array of `int64_t` (`mem.bin`) of size `MEM_SIZE` (16MB). This array stores global registers, bytecode, and the runtime stack.
    *   **Global Registers Area** (first `MEM_GLOBAL_SIZE = 32` `int64_t`s):
        *   `mem.bin[GLOBALADDR_IP]`: Instruction Pointer - address of the next instruction to execute.
        *   `mem.bin[GLOBALADDR_SP]`: Stack Pointer - address of the top of the current evaluation stack (points to the next free slot, grows upwards).
        *   `mem.bin[GLOBALADDR_BP]`: Base Pointer - address of the base of the current function's stack frame.
    *   **Code Segment**: Bytecode instructions start immediately after the global registers area.
    *   **Stack Segment**: The runtime stack grows upwards in memory. Each function call establishes a new stack frame, notionally allocated `MEM_STACK_SIZE` (256 `int64_t`s) by `TY_INST_CALL`.
*   **Execution Loop (`execute`)**: Fetches, decodes, and executes bytecode instructions one by one, manipulating the stack and VM registers.

### Instruction Set

Instructions are `int64_t` opcodes, sometimes followed by one `int64_t` operand. `SP` refers to `mem.bin[GLOBALADDR_SP]`, `IP` to `mem.bin[GLOBALADDR_IP]`, `BP` to `mem.bin[GLOBALADDR_BP]`. Stack operations: `mem[SP++] = val` (push), `val = mem[--SP]` (pop).

*   **Control Flow & Termination:**
    *   `TY_INST_NOP`: No operation.
    *   `TY_INST_END`: Terminates VM execution.
    *   `TY_INST_JMP operand`: `IP = operand` (operand is an absolute bytecode address).
    *   `TY_INST_JZ operand`: `val = pop(); if (val == 0) IP = operand`.
    *   `TY_INST_CALL operand`: (operand is function address)
        1.  Push `IP + 1` (return address).
        2.  Push current `SP`.
        3.  Push current `BP`.
        4.  `IP = operand`.
        5.  `BP = current SP + 3` (new frame base, after pushed linkage).
        6.  `SP = SP + MEM_STACK_SIZE` (allocate stack space for new frame).
    *   `TY_INST_RETURN`:
        1.  `ret_val = pop()`.
        2.  `IP = mem[BP - 3]` (restore return IP).
        3.  `SP = mem[BP - 2]` (restore caller's SP).
        4.  `BP = mem[BP - 1]` (restore caller's BP).
        5.  `push(ret_val)`.

*   **Stack & Memory Operations:**
    *   `TY_INST_PUSH_CONST operand`: `push(operand)`.
    *   `TY_INST_PUSH_LOCAL_VAL operand`: `push(mem[BP + operand])` (operand is stack offset).
    *   `TY_INST_PUSH_LOCAL_ADDR operand`: `push(BP + operand)`.
    *   `TY_INST_DEREF`: `addr = pop(); push(mem[addr])`.
    *   `TY_INST_ASSIGN1` (and aliases `ASSIGN2`-`ASSIGN4`): `val = pop(); addr = pop(); mem[addr] = val`.

*   **Arithmetic Operations** (pop two values `val1`, `val2`; push `val1 op val2`):
    *   `TY_INST_ADD`, `TY_INST_SUB`, `TY_INST_MUL`
    *   `TY_INST_DIV`: `val1 / val2`. Returns `INT64_MAX` if `val2 == 0`.
    *   `TY_INST_MOD`: `val1 % val2`. Returns `INT64_MAX` if `val2 == 0`.

*   **Bitwise & Shift Operations** (pop two values; push result):
    *   `TY_INST_SHL` (`<<`), `TY_INST_SHR` (`>>`)
    *   `TY_INST_BITOR` (`|`), `TY_INST_BITXOR` (`^`), `TY_INST_BITAND` (`&`)

*   **Logical & Comparison Operations** (pop two values; push 0 or 1):
    *   `TY_INST_OR`: `val1 | val2` (Note: bitwise OR behavior).
    *   `TY_INST_AND`: `val1 & val2` (Note: bitwise AND behavior).
    *   `TY_INST_EQ` (`==`), `TY_INST_NE` (`!=`)
    *   `TY_INST_LT` (`<`), `TY_INST_LE` (`<=`), `TY_INST_GT` (`>`), `TY_INST_GE` (`>=`)

*   **Unary Operations** (pop one value; push result):
    *   `TY_INST_BITNOT`: `~val` (bitwise complement).
    *   `TY_INST_NEG`: (Defined in `type_t` but not generated by parser or handled by VM).
    *   `TY_INST_NOT`: (`!` operator. Parsed, but **not implemented** in the VM `execute` function).

*   **Built-in Function Calls:**
    *   `TY_INST_READ`: `count = pop(); addr = pop(); fd = pop(); push(read(fd, &mem[addr], count))`.
    *   `TY_INST_WRITE`: `count = pop(); addr = pop(); fd = pop(); push(write(fd, &mem[addr], count))`.
    *   `TY_INST_USLEEP`: `usec = pop(); push(usleep(usec))`.

## Examples

A simple loop to calculate a sum:
```lkjscript
// Calculate sum of 1 to 5
&sum = 0
&i = 1
&limit = 5

&result_of_loop = loop {
  if i > limit {
    break sum // The loop evaluates to the final sum
  }
  &sum = sum + i
  &i = i + 1
}

// 'result_of_loop' now holds 15.
```

Factorial function:
```lkjscript
fn factorial(n) {
  if n == 0 {
    return 1
  }
  if n == 1 { // Optimization / base case
    return 1
  }
  // &n_minus_1 = n - 1 // Not needed, can pass expression
  &recursive_call_result = factorial(n - 1)
  return n * recursive_call_result
}

&fact5_addr = factorial(5)
// fact5_addr now stores the result 120.
```

## Contributing

Contributions are welcome! If you find bugs, have suggestions for improvements, or want to add features, please feel free to:
1.  Fork the repository.
2.  Create a new branch for your changes.
3.  Make your changes and commit them with clear messages.
4.  Push your branch to your fork.
5.  Open a Pull Request against the main repository.

Please ensure any C code contributions adhere to a consistent style and include comments where necessary.

## License

This project is currently unlicensed. You are free to use, modify, and distribute the code as per standard copyright law, but there are no explicit permissions or restrictions granted by a formal license. If a license is added in the future, it will be available in a `LICENSE` file in the repository.