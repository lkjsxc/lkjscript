# Language contract 1

This specification defines authored lkjscript meaning. The parser, semantic validator, bytecode VM,
AST reference evaluator, value checker, and differential tests are the executable oracles. It does
not define host grants, process topology, storage schemas, or application policy.

## Modules and declarations

A file contains exactly one `(module name …)` form. Declarations are `record`, `variant`,
`interface`, closed `extern`, `fn`, `task`, `const`, `component`, and `test`. Imports explicitly bind
an alias to a local module or `dependency-alias.module`; exports are explicit and tests cannot be
exported. Undeclared, private, ambiguous, duplicate, or foreign references reject at validation.

An `extern` is a general optimized primitive selected from the validator-owned closed registry. Its
authored parameter/result signature must exactly match the intrinsic contract. It is not arbitrary
native FFI, and an unknown or signature-forged extern rejects before authority publication.

## Types and values

Types are unit, bool, signed i64, immutable bytes, immutable UTF-8 text, source-origin `StaticText`,
opaque secret, nominal named type, structural record, homogeneous list, ordered map, option, result,
stream, and function. Records have unique named fields. Variants have a closed unique case set and
optional typed payload. Lists and maps are immutable values; helpers return new collections.

Map keys are bool, i64, bytes, or text. Their order is first by key kind in that listed order and
then by the natural total order of the contained value. Map construction rejects duplicate keys,
and iteration/JSON projection is deterministic. Values are limited to depth 256 and 1,000,000 total
collection items at checked boundaries.

`StaticText` can only be written literally in accepted source. Runtime text cannot coerce to it;
database statements and configuration keys use it to prevent injection by ordinary application
data. Secret, stream, function, transaction, and other resource values are non-durable. Opaque
resource identity has no public serialization.

## Evaluation

Expressions are literals, variables, lazy `if`, lexical `let`, ordered `do`, direct calls, record
construction/field access, variant construction/match, list/map construction, function reference,
capability `perform`, and lexical `transaction`. Operands, binding values, arguments, fields, list
items, and map entries evaluate in source order. Only the selected conditional or match arm runs.
There is no implicit coercion or ambient overload resolution.

Signed arithmetic is checked. Overflow, division by zero, signed division overflow, invalid
canonical integer spelling, missing list/map elements, invalid UTF-8 conversion, wrong runtime
shape, fuel exhaustion, and explicit operation-contract violation are traps. Text equality is UTF-8
byte equality; `text.length` is UTF-8 byte length, not scalar or grapheme count. Bytes are exact.
Collection and record equality is structural after nominal owner equality where applicable.

Pure evaluation is deterministic and independent of grants, wall time, randomness, scheduler, and
external state. Task functions are deterministic only relative to their ordered typed capability
outcomes.

## Effects and recursion

`fn` is pure and may call only pure closure. `task` declares requirement aliases and may call pure
or task functions and perform those aliases. The validator computes the transitive capability
closure; undeclared or mismatched effects reject. Taking a task function reference is pure, but only
a component runner may execute it with grants.

User recursion consumes explicit VM/reference continuation frames rather than native Rust stack.
Execution policy bounds instruction fuel, call depth, and value stack. The bytecode and AST routes
must agree on result, trap class/code, capability ordering, and exhaustion.

## Typed JSON

JSON contract 1 accepts UTF-8 JSON with signed i64 integers only; floating point and out-of-range
unsigned values reject. Default bounds are 1 MiB total/string, depth 128, and 100,000 items.
Duplicate object fields and trailing input reject. Typed decode additionally rejects unknown or
missing fields, wrong nominal shape/case, invalid base64 bytes, and type/range mismatch with a
precise JSON path. Encoding is deterministic and bounded.

Expected application outcomes such as absent, stale, denied, or invalid domain input should be
typed values or component responses. They are not semantic traps or infrastructure diagnostics.
