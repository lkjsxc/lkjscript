# Create and distribute a native library

This walkthrough builds a reusable `sum-by<Item>` library and a shipping command
that uses it with its own `Parcel` type. It works with the published v0.1.40
executable. You need that executable, a text editor and ordinary file operations.
The programs below are complete native declarations; no host-language generator
or compiler checkout is involved.

Start in a new directory with your installed executable copied to `./lkjscript`:

```sh
./lkjscript new ./aggregation --template command --name aggregation
./lkjscript --project ./aggregation status
./lkjscript package builtin query owners --name list-fold-left
```

The command recipe supplies an exact standard dependency and a small `main`
command. We add the library in a separate module. The example keeps that starter
command; only `sum-by` is exported from the new module.

## Author and check the library

Save this as `library.lkjc`, replacing `LIBRARY_BASE` with the `rev_...` reported by
the library's `status`:

```text
request base=LIBRARY_BASE
declarations.begin
(units
  (use std builtin)
  (module create aggregation (as $aggregation)
    (function create step (visibility private)
      (type-parameter create Item)
      (parameter create amount (type (function (Item) I64)))
      (parameter create total (type I64))
      (parameter create item (type Item))
      (returns I64) (effect pure)
      (body (call std::add (local total) (invoke (local amount) (local item)))))
    (function create sum-by (visibility public)
      (type-parameter create Item)
      (parameter create items (type (list Item)))
      (parameter create amount (type (function (Item) I64)))
      (returns I64) (effect pure)
      (body
        (call std::list-fold-left (types Item I64) (local items) (i64 0)
          (bind (function-value step (types Item)) (local amount)))))
    (function create identity (visibility private)
      (parameter create value (type I64))
      (returns I64) (effect pure)
      (body (local value)))
    (test create empty (visibility private)
      (actual (call sum-by (types I64) (list I64) (function-value identity)))
      (expected (i64 0)))
    (test create signed-values (visibility private)
      (actual
        (call sum-by (types I64) (list I64 (i64 7) (i64 -3) (i64 8))
          (function-value identity)))
      (expected (i64 12)))))
declarations.end
```

`sum-by` accepts any item type and a pure function that projects one item to an
integer. Explicit `(types Item I64)` instantiates the ordinary standard fold.
`bind` supplies the mapper argument of `step`, leaving its total and item
arguments for the fold. Items are visited in order; empty input returns zero.
Both the mapper and integer addition may fail, and the first failure stops the fold.

Plan, review the complete proposal, and apply using the exact `plan_...` token
printed by the plan command in place of `LIBRARY_PLAN`:

```sh
./lkjscript --project ./aggregation change plan --input-file ./library.lkjc --output ./library.logical-plan
./lkjscript --project ./aggregation change apply --input-file ./library.lkjc --plan LIBRARY_PLAN
./lkjscript --project ./aggregation check
./lkjscript --project ./aggregation package current export --kind transport --output ./aggregation.lkjp
```

`check` runs the graph tests and requires agreement between the production and
reference evaluators. Keep these four fields from the export's `package` record:

| Export field | Placeholder below | Purpose |
| --- | --- | --- |
| `id` | `LIBRARY_PACKAGE` | Logical package identity |
| `revision` | `LIBRARY_REVISION` | Accepted semantic revision |
| `package-revision` | `LIBRARY_PACKAGE_REVISION` | Exact logical package selection |
| `transport` | `LIBRARY_TRANSPORT` | Exact exported container |

These values come from your program. A package name does not replace an exact
selection. The transport includes private implementation bodies and the exact
dependency closure; private visibility is not source confidentiality.

## Use it with an application-owned type

Create a separate consumer, stage the exported transport, and inspect its public
interface. Substitute the observed export values in these commands:

```sh
./lkjscript new ./shipping --template command --name shipping
./lkjscript --project ./shipping package dependency stage --transport LIBRARY_TRANSPORT --input-file ./aggregation.lkjp
./lkjscript --project ./shipping package dependency query owners --package-revision LIBRARY_PACKAGE_REVISION
./lkjscript --project ./shipping status
```

Staging validates and stores code without changing the consumer's accepted HEAD.
The following reviewed request installs the exact dependency and creates the
consumer together. Save it as `consumer.lkjc`; replace `CONSUMER_BASE` with this
consumer's current `rev_...`, and substitute the three library identities:

```text
request base=CONSUMER_BASE
add.dependency package=LIBRARY_PACKAGE semantic-revision=LIBRARY_REVISION package-revision=LIBRARY_PACKAGE_REVISION
declarations.begin
(units
  (use std builtin)
  (use sums LIBRARY_PACKAGE LIBRARY_PACKAGE_REVISION)
  (module create shipping (as $shipping)
    (record create Parcel (visibility private)
      (field create units (type I64))
      (field create grams (type I64)))
    (function create parcel-weight (visibility private)
      (parameter create parcel (type Parcel))
      (returns I64) (effect pure)
      (body
        (call std::multiply
          (field (local parcel) Parcel::units)
          (field (local parcel) Parcel::grams))))
    (function create shipment-weight (visibility private)
      (parameter create parcels (type (list Parcel)))
      (returns I64) (effect pure)
      (body
        (call sums::sum-by (types Parcel) (local parcels) (function-value parcel-weight))))
    (test create two-parcels (visibility private)
      (actual
        (call shipment-weight
          (list Parcel
            (record Parcel (field Parcel::units (i64 2)) (field Parcel::grams (i64 125)))
            (record Parcel (field Parcel::units (i64 3)) (field Parcel::grams (i64 200))))))
      (expected (i64 850)))
    (component create console (visibility private)
      (port create weight (type (function ((list Parcel)) I64)) (function shipment-weight))))
  (target create weight (component shipping::console) (runner command) (port shipping::console::weight)))
declarations.end
```

`Parcel` belongs to the consumer. The library knows only its type parameter and
the supplied projection function. The application chooses the units and accepts
signed I64 inputs; add an application policy if negative counts or weights should
be rejected.

Review and apply with this request's own token in place of `CONSUMER_PLAN`:

```sh
./lkjscript --project ./shipping change plan --input-file ./consumer.lkjc --output ./consumer.logical-plan
./lkjscript --project ./shipping change apply --input-file ./consumer.lkjc --plan CONSUMER_PLAN
./lkjscript --project ./shipping check
./lkjscript --project ./shipping run weight --arguments '[[{"units":2,"grams":125},{"units":3,"grams":200}]]'
./lkjscript --project ./shipping run weight --arguments '[[]]'
```

The results are `850` and `0`, respectively. The outer JSON array contains function
arguments; its one element is the parcel list. Record fields use their declared
names. Integer multiplication and addition are checked: for example, the following
input fails with `normalized_integer_overflow`, rather than producing a wrapped total:

```sh
./lkjscript --project ./shipping run weight --arguments '[[{"units":9223372036854775807,"grams":2}]]'
```

## Resume editing from accepted meaning

Find the exact module owner, then replace `SHIPPING_MODULE` below with its `mod_...`
identity:

```sh
./lkjscript --project ./shipping query find module shipping
./lkjscript --project ./shipping change draft --owner SHIPPING_MODULE --output ./shipping-draft.lkjc
./lkjscript --project ./shipping change plan --input-file ./shipping-draft.lkjc
```

An untouched draft plans as `outcome=unchanged`. Edit that draft when changing the
accepted declarations, and review a fresh plan before applying. Original comments,
formatting and aliases are presentation; the accepted graph owns the program.
Dependency upgrades require an explicit reviewed selection and any necessary
consumer repairs.

## Run a standalone bundle

Build the consumer and copy the executable and bundle into a runtime directory:

```sh
./lkjscript --project ./shipping build --output ./shipping.lkja
mkdir ./runtime
cp ./lkjscript ./shipping.lkja ./runtime/
```

Save the complete descriptor below as `runtime/weight.deployment.json`. Its artifact
path is relative to the descriptor. The four topology fields must be present with
`null` for this foreground command; `execution` and `runtime` may be omitted.

```json
{
  "artifact": "shipping.lkja",
  "target": "weight",
  "listen": null,
  "http": null,
  "session": null,
  "worker": null,
  "streams": {
    "maximum_chunk_bytes": 65536,
    "maximum_buffered_chunks": 8,
    "maximum_total_bytes": 67108864,
    "maximum_live_streams": 1024
  },
  "configuration": {},
  "secrets": [],
  "grants": []
}
```

```sh
cd ./runtime
./lkjscript run --deployment ./weight.deployment.json --arguments '[[{"units":2,"grams":125},{"units":3,"grams":200}]]'
```

This returns `850` using the bundle's complete code closure. It requires no library
or consumer authoring project, package transport, or source request at runtime.
The bundle needs a compatible lkjscript executable; it is not a self-contained
machine executable. Deployment execution invokes production once and reports
`verification=not-performed`; project checks and project runs supplied differential
verification earlier. This pure command needs no capability grants.

## Read arguments from a file

Development v0.1.42 adds `--arguments-file`; published v0.1.40 and the selected
v0.1.41 candidate use the inline form above. With the newer executable, save this
ordinary JSON as `runtime/parcels.json`:

```json
[[{"units":2,"grams":125},{"units":3,"grams":200}]]
```

From the walkthrough's original directory, run either form:

```sh
./lkjscript --project ./shipping run weight --arguments-file ./runtime/parcels.json
./runtime/lkjscript run --deployment ./runtime/weight.deployment.json --arguments-file ./runtime/parcels.json
```

Both return `850`. The file path is relative to the current working directory,
including when the descriptor is elsewhere. Use one argument selector per call;
omitting both supplies `[]`. The file must be regular, with no final symlink, and
fit within the current 1 MiB JSON limit including whitespace. JSON structure and
typed-value limits also apply. This allows inputs too large for the operating
system's command-line arguments; it does not stream an unbounded data set.

The literal library and consumer were exercised with a copied official v0.1.40
binary outside the compiler checkout, including unchanged drafting, empty input,
multiplication/addition overflow, build after moving the producer, and execution
after moving the consumer. The [continuation record](../campaigns/202609222330.md)
owns that bounded tutorial observation. It is separate from release acceptance.
The v0.1.42 development file-input observation freshly authors the same two native
programs and returns `2048000` from 8,192 parcels through both project and detached
bundle execution. Its 196,612-byte input includes a newline. The retained malformed,
wrong-type and overflow inputs reject, and accepted HEAD stays unchanged.
