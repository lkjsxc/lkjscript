# Your first native command

Start with one compatible lkjscript executable in a new working directory.
This guide uses the same public operations on released v0.1.44 and the local
v0.1.45 development product. It does not require a compiler checkout, Cargo,
Python or a host-language semantic generator. Ordinary file copying and editing
are host operations, not hidden program-authoring interfaces.

## Create, check and run

```sh
lkjscript new hello --template command --name hello
lkjscript --project hello status
lkjscript --project hello check
lkjscript --project hello run main
lkjscript --project hello build --output hello/generated/application.lkja
lkjscript run --deployment hello/command.deployment.json
```

Both runs return `"hello"`. Project execution of a pure command compares two
independent evaluators. Deployment execution runs production once. The generated
descriptor points to `generated/application.lkja` relative to itself; building an
unrelated filename does not update that descriptor.

## Author your own function, test and target

Save the following [literal native unit](examples/squares.lkjc) as `squares.lkjc`.
Replace `BASE` with the exact `revision id=rev_...` from the current `status`.
Do not include `id=` in the replacement. These identifiers are observed values,
not names to invent or copy from another project.

```text
request base=BASE
declarations.begin
(units
  (use std builtin)
  (module create math (as $math)
    (function create square (visibility public)
      (parameter create value (type I64))
      (returns I64) (effect pure)
      (body (call std::multiply (local value) (local value))))
    (function create answer (visibility private)
      (returns I64) (effect pure)
      (body (call square (i64 12))))
    (test create twelve-squared (visibility private)
      (actual (call square (i64 12))) (expected (i64 144)))
    (component create console (visibility private)
      (port create main (type (function () I64)) (function answer))))
  (target create squares (component math::console) (runner command) (port math::console::main)))
declarations.end
```

The public function multiplies its input by itself. A private entry point supplies
12, the independent expected-value test requires 144, and the Command target
exposes that entry point. Arithmetic, types and declarations use the same public
language as larger libraries; this is not a special demonstration primitive.

## Review before publication

```sh
lkjscript --project hello change plan --input-file squares.lkjc --output squares.logical-plan
lkjscript --project hello change apply --input-file squares.lkjc --plan TOKEN
lkjscript --project hello check
lkjscript --project hello run squares
```

Inspect the proposed owners, tests and semantic effects. Replace `TOKEN` with the
exact `plan token=plan_...` from the successful plan before running apply.
The result is `144`. The plan file is inspectable evidence, not imported authority.
The token binds the request and its validated effects at the exact base revision.
An accepted edit changes that revision; fetch a fresh status for a later proposal.
An invalid or stale request must not partially publish meaning.

## Re-enter without the original source

Discover the accepted module, using its observed `mod_...` identity:

```sh
lkjscript --project hello query find module math
lkjscript --project hello change draft --owner MODULE --output math-draft.lkjc
lkjscript --project hello change plan --input-file math-draft.lkjc
```

Replace `MODULE` with that exact identity. An untouched draft reports
`outcome=unchanged`: it needs no application step and no plan-output destination.
Edit that proposal, review a new plan and apply its exact token to change the
program. The draft is reconstructed from accepted meaning, not from the saved
`squares.lkjc` file. There is no second synchronized source authority.

## Run independently of the authoring project

Create a new runtime directory and build the accepted bundle there:

```sh
mkdir runtime
lkjscript --project hello build --output runtime/squares.lkja
cp hello/command.deployment.json runtime/squares.deployment.json
```

Edit only these two deployment fields, keeping the other generated fields intact:

```json
"artifact": "squares.lkja",
"target": "squares"
```

This is a fragment of the existing object, not a complete descriptor. It selects
an accepted target; it does not author a new function or grant a capability.
Copy your actual executable into `runtime/lkjscript`, then run there:

```sh
cd runtime
./lkjscript run --deployment squares.deployment.json
```

The result remains `144`. Runtime files are just the executable, bundle and
descriptor. Move the authoring directory and literal proposal aside to demonstrate
that this route does not reopen them. Retain those sources separately for future
changes: an artifact is not an authoring backup or a self-contained native binary.
The descriptor's relative artifact path remains valid when the runtime directory
moves. Keep a compatible executable; runtime selection does not migrate data.

## Continue with ordinary composition

The [library guide](native-library.md) transports a generic library into another
application. [HTTP](native-http.md) and [typed HTML](native-html.md) add actual
resident workloads. [Text composition](native-text.md) needs the newer standard;
it is not provided by public v0.1.44. Each guide identifies its tested runtime.

[Generated grammar](../generated/change-grammar.md) and the executable's
`capabilities change` describe further declaration kinds and their bounds.
[Current status](../status.md) separates public releases from development source.
