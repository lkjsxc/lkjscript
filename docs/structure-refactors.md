# Structural refactoring plans

The source catalog is the frontend's claim about resolved identities and exact
spelling sites. These planners never grant checked-IR authority. They return
patches against the catalog's exact source snapshot. The transaction driver
elaborates the complete candidate and checks its IR independently before it
publishes any file. A planner failure appends no patches.

Text remains the repository representation. Unchanged bytes, including comments
and spacing outside each planned span, are copied without formatting. Operations
in one request address the original snapshot; overlapping incompatible edits
are refused rather than silently ordered. Every refusal identifies a source
identity, an `E_EDIT_*` rule, and a decision site when a particular reference
prevents the operation.

## Request vocabulary

These objects appear in the transaction's version-1 `operations` array.
`op`, `id`, and optional `expect` are common fields. `expect` is the current
item's source-text digest. Unknown operation fields and unknown nested descriptor
fields are rejected. Identities have no leading `crate::`; source paths emitted
by a relocation may have that prefix.

### Change a signature

~~~json
{
  "op": "change-signature",
  "id": "math::combine",
  "parameters": [
    "math::combine::param:right",
    "math::combine::param:left",
    {"name": "bias", "type": "i64", "argument": "current_bias"}
  ]
}
~~~

Existing parameter identities select and order retained parameters; omission
removes a parameter. A descriptor introduces one parameter. Its `argument` is
an expression supplied once in the request and elaborated separately in each
caller's scope.

Each resolved direct call evaluates its original receiver and arguments exactly
once, in their original order, into fresh local bindings. New arguments are then
evaluated in the descriptor order. The final call supplies retained bindings in the new parameter order and
new expressions directly in their declared parameter positions, preserving their
contextual types. A removed argument is still evaluated and retained until
the enclosing rewritten call expression ends. `self` must remain the first
existing parameter. Direct method calls use the original receiver's declared
borrow form. Every original argument and the complete call retain their exact
inferred types, so removing a parameter cannot widen an overflowing integer
expression. A new parameter name that would capture an existing resolved global
value is refused.

The planner changes only the declaration's parameter list and resolved call
expressions. It leaves the function body intact, so removing a parameter still
used there is refused by candidate checking. Function values, unresolved incoming
calls, and abstract trait declarations require explicit decisions. This version
does not coordinate a signature change across a trait's implementation cohort.

### Add or remove a field

~~~json
{"op":"add-field","id":"Point","source":"tag: i64","value":"default_tag()"}
{"op":"remove-field","id":"Point::field:tag","replacement":"fallback_tag()"}
~~~

`add-field` addresses a struct or named enum variant. `source` contains exactly
one named field. Record constructors without a base receive the `value`
expression after their existing fields. The complete constructor retains its
original inferred type, so a new generic field cannot silently change the types
of old initializers. Struct-update constructors obtain the
new field from their base. Record patterns without `..` gain `tag: _`; patterns
with `..` already cover it.

`remove-field` addresses the field identity. Every explicit constructor
initializer, including the removed initializer, still runs exactly once in
source order. A struct-update base runs after the explicit initializers. Fresh
bindings preserve that order while the rebuilt constructor omits the field.
Removed initializer values live until that rewritten constructor expression
ends. Original initializer and result types are explicitly retained; a removed
field cannot silently widen its initializer. Replacement accesses retain the
original field type and borrow their receiver without taking ownership.

A field access needs `replacement`; its receiver is evaluated once before that
expression. Assignments through the removed field require an explicit body
decision. Removing a pattern field is refused if any resolved use depends on a
binding introduced by that field, or if an ambiguous pattern-origin use could
depend on that binding. Field labels are matched to typed reference
spans; a similarly named field in a nested or unrelated type is not changed.

Tuple and unit declarations require a constructor-style decision. The planner
does not infer a conversion from those forms to a named record.

### Add or remove a variant

~~~json
{"op":"add-variant","id":"Choice","source":"Later(i64)","arm":"Choice::Later(_) => 0i64"}
{"op":"remove-variant","id":"Choice::variant:Later","replacement":"Choice::Ready"}
~~~

`add-variant` accepts exactly one variant and one unguarded arm. The arm must
name the new variant. Each directly relevant match receives it before an
unguarded catch-all, or at the end of the arm list. Exact scrutinee types find
wildcard-only matches even when the body mentions no existing variant. Resolved
alias RHS types and generic argument environments recover the nominal enum;
repeated alias expansion or a needed omitted argument is an explicit decision
site. Nested matches that require a surrounding-pattern rule report their
location.

`remove-variant` prunes alternatives from match patterns and removes a complete
arm when its pattern becomes impossible. It descends through tuple, record,
variant, reference and or-pattern structure. A constructor needs `replacement`;
its original arguments or field initializers run once in order before the
replacement expression. Non-match destructuring and indirect constructor uses
require an explicit decision. Imports of a removed variant must also be handled
explicitly. Changed matches retain both their scrutinee type and result type,
and removed constructor arguments retain their original inferred types. Removing
an arm cannot widen an existing constructor payload merely by removing a type
constraint. A
macro-generated match without an exact source `match` expression is refused;
rewriting it could erase evaluation outside the represented match.

### Extract an expression

A call-site identity already selects an exact expression:

~~~json
{
  "op": "extract-function",
  "id": "main::body:BODY_DIGEST::call:0",
  "name": "calculate",
  "parameters": [],
  "result": "i64"
}
~~~

Other expressions use byte offsets relative to their function item's text.
The item digest is mandatory with offsets:

~~~json
{
  "op": "extract-function",
  "id": "compute",
  "expect": "ITEM_TEXT_DIGEST",
  "start": 28,
  "end": 36,
  "name": "calculate",
  "parameters": [
    {"id":"compute::param:x","name":"value","type":"i64","argument":"x"}
  ],
  "result": "i64"
}
~~~

The range must equal one complete syntax expression with unambiguous inferred
type evidence. The supplied result type must retain that inferred type. Every
free local binding must be represented by its exact binding or parameter identity. A capture has
an explicit name, type and argument. Its type must retain the binding's inferred
type, with exactly the requested reference layer. Type comparison ignores
whitespace and explicit `crate::` prefixes; other aliases require canonical
spelling. Its argument is restricted to that binding,
`&binding`, or `&mut binding`; capture acquisition introduces no arbitrary
effectful expression. A borrowed capture rewrites references to `(*parameter)`,
including preserving record shorthand as `old_field: new_value`.

Owned or unresolved value captures require an explicit borrow to preserve
ownership and drop timing. A copied scalar that the expression writes, mutably
borrows, or uses as a method receiver also needs a reference capture. An inner
binding cannot shadow a supplied capture name. Captures that are missing,
duplicated, unused, from another function, or declared inside the selection
are refused.

The new free function is inserted in the original semantic module. Resolved
global paths are qualified before they move. The source function's generic
header and where clause are copied, and its type parameters are passed
explicitly to the extracted function. The retained capture and result types
are independently elaborated in that scope.

Extraction currently addresses free functions. Escaping return, try, divergence,
break, continue, closures and nested item declarations require a body-level
decision. Scope evidence must be complete for the moved range. Paths whose
generic arguments interrupt the exact editable prefix require a separate path
decision. These are explicit limits, not guesses from source names.

### Inline a function

~~~json
{"op":"inline-function","id":"math::add","remove":true}
~~~

Optional `sites` is a nonempty array of exact call-site identities. Without it,
all resolved direct calls are selected. `remove` defaults to false and removes
the definition only when every remaining use permits that removal.

Inlining currently takes a concrete free function. Old arguments are evaluated
once in their original order into hygienic temporaries with the original
parameter types. A nested scope then introduces the original parameter names
and mutability, followed by the function body. Resolved globals and parameter
type paths are qualified, so a caller's local spelling cannot capture them.
The complete inlined body is bound to a fresh variable with the original
relocated result type before its value is returned. This retains narrow integer
inference, overflow behavior and result coercions. Hygiene covers the entire
callee source, including unused parameter names. A wildcard `_` parameter becomes
a fresh named binding, preserving its destruction at function exit; a resolved
use of that wildcard requires an explicit decision. A final top-level
`return value;` becomes the block's tail value.

Generic substitution, early returns, `?`, escaping control, closures, nested
items and unresolved scope require explicit decisions. Recursive expansion
is refused when the selected call is inside the definition itself. An
unselected call, import or indirect function use prevents definition removal.
A moved private reference that the caller cannot access is refused by the
candidate's visibility check. Cross-module inline refuses method expressions:
an imported trait can change dispatch even when the candidate still checks.
Relative include macros also need an explicit source-file relocation rule.

## Evidence and verification

All seven operations need the full typed catalog, including exact expression
types. An items-only catalog is insufficient. Canonical inferred nominal types
are emitted with their resolved source paths, including reference prefixes;
missing type evidence and ambiguous intrinsic spellings are explicit refusals.

Only exact resolved references select declaration or use sites. Token scanning
is used for syntax delimiters, comma ownership and fresh-name hygiene, never
to guess a symbol's target. Missing typed members, ambiguous pattern origins,
associated-function gaps and other unresolved references remain decision sites.

The focused executable `src/structure_refactor_cases.lkj` constructs typed
catalogs from source, runs the production planner, independently elaborates and
checks each candidate, and interprets checked candidates against concrete
expected results. Its cases cover argument and initializer evaluation order,
nested calls, record shorthand, nested match alternatives, scope capture,
mutable extraction, generic extraction, terminal returns, selected inline
sites, declared result widths and traps, wildcard-parameter destruction order,
reference-type relocation, deliberate generated-name collisions,
stale/invalid requests and candidate refusal. Publication recovery and
whole-tree atomicity are the transaction driver's separate contract.

Run `bash tools/structure-refactor-test.sh PATH_TO_COMPILER` from the repository
root to build and execute the focused native harness. The compiler argument
defaults to `target/lkjscript`.


## Rename and relocation postcondition

A rename or a move that claims to preserve meaning needs more than a passing
type check. A newly selected spelling can collide with another dispatch choice.
The transaction driver must require equal canonical checked-IR semantic identity
before publishing a rename-only or relocation-only transaction. If a request
also contains behavior-changing operations, its pure rename/relocation subset
must first be independently elaborated, checked and compared before the complete
candidate is checked. A mismatch is a refusal naming the affected operation;
it is not an advisory warning.

Source identity changes and source-only signatures may still appear in the plan.
They do not weaken this checked-IR postcondition. A member planner must preserve
field shorthand bindings and rename a complete proved trait/implementation
cohort. Alias provenance, unresolved associated calls, instance-dependent method
selection and scope changes remain explicit decision sites until their evidence
is complete.
