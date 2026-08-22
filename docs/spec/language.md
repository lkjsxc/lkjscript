# Language meaning

Status: normative for meaning graph contract 1.

## Representation

Language constructs are typed semantic records in canonical module shards. There is no maintained
source grammar. Public transaction JSON and CLI schema describe construction; deterministic JSON
review projection describes inspection. Names locate meaning, while typed stable IDs own
references and continuity.

Evaluation is strict and left-to-right except `if` branch choice and variant `match`, which evaluate
only the selected body. `let` bindings evaluate in declared order. `do` evaluates in order and
returns its final value. Capability operations and lexical transactions preserve that order.

## Types and values

The closed current type surface is:

- `Unit`, `Bool`, checked signed `I64`, immutable `Bytes`, UTF-8 `Text`, and compile-time
  `StaticText`;
- opaque `Secret` and typed live `Resource` handles;
- nominal records and variants, plus structural adapter records;
- homogeneous lists and deterministic ordered maps;
- option and result values;
- task-scoped byte streams; and
- function types.

There are no implicit coercions. `I64` arithmetic traps on overflow and division edge cases.
Indexing, collection counts, and all decoder sizes are checked. Text is valid UTF-8; names use the
closed portable identifier rules and no normalization-dependent identity. Maps permit bool, i64,
bytes, or text keys and iterate by the specified total order. Runtime values are bounded to depth
256 and 1,000,000 collection items.

Live resources, secrets, streams, database transactions, queue leases, and runtime handles never
enter durable graph values. A durable literal has one canonical typed encoding and exact bound.

## Declarations

A module owns imports, exports, and declarations. Declaration kinds are record, variant,
interface, closed external function, pure function, task function, constant, component, and test.

Records own ordered stable field identities, mutable field names, and exact types. Variants own
stable case identities and optional exact payload types. Interfaces own stable operation identities,
parameters, result, declared failure class, and applicable limits. Constants own one typed pure
expression.

Functions own stable parameter identities, exact result, and body. A pure function may call only
pure meaning. A task function declares the capability aliases and exact interfaces it may perform.
There is no ambient overload resolution, global mutation, closure capture, generics, traits,
floating point, set, user scheduler primitive, or dynamic evaluation in contract 1.

## Expressions and bindings

Expression kinds are unit/bool/i64/text/static-text literals, variable, conditional, lexical let,
sequencing, function call, record construction and field projection, variant construction and
match, list, map, function reference, capability operation, and lexical capability transaction.

Bindings and expression sites receive stable typed IDs when semantic operations, diagnostics, or
relations need robust selection. Their structural paths are canonical within the owning
declaration and are validated against the operation tree. Paths and dense compiler indexes are not
global identities.

All references lower to exact package/module/declaration/member identities during validation.
Unresolved or ambiguous references are allowed only as closed draft holes; an accepted revision
contains neither.

## Equality, expectation, and failure

Value equality is type-directed and deterministic. Resource and secret values do not provide
durable equality. Tests own actual and expected typed expressions; success requires equality in
both bytecode and the semantic reference interpreter.

A typed `Result` is an ordinary expected program value. Trap, capability failure, possible
visibility, resource exhaustion, cancellation, corruption, and infrastructure failure are runtime
classes and may not be silently converted into one another. Exact adapter contracts define which
external failures become typed operation results.

## Semantic validation

Acceptance validates namespace uniqueness, visibility, imports/exports, nominal identity, type
agreement, effect closure, capability operation membership, component requirements and ports,
target bindings, test types, expression/binding identity shape, canonical relations, and exact
dependency closure. Relation reconstruction is an independent check of retained relation bytes.

Compiler lowering consumes validated graph structures directly. The prepared bytecode tier and
the independent semantic reference interpreter must agree. Rendering or parsing text is absent
from build, test, run, service, and worker paths.
