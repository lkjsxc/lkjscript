# Own nominal records across packages

`library.lkjc` defines `Packet<T>`, its same-shaped but distinct `Twin<T>`, and a
nested `Envelope<T>`. Their type identities are public; representation access is
limited to the exact defining package and module. Public functions construct,
observe, borrow and consume these values. The supplier contains no concrete
resource type and can be checked and exported before its consumers exist. Its
public `Reader` contract, `PacketReader<T>` implementation scheme and generic
`observe<S>` function preserve nominal Self identity through exact witnesses.

`Empty`, `Data<T>` and the two phantom declarations also have affine ownership.
An empty layout or a layout containing only ordinary data still cannot be copied
or consumed twice. Every type argument is admitted even when it contributes no
field. Ordinary parameters require `transferable`; owned parameters require
`owned`, optionally together with the other ownership constraints.

`application.lkjc` supplies an `OwnedI64Cell` and uses only public supplier
functions. It obtains the stamp through the exported generic `observe` function
and an exact `PacketReader<OwnedI64Cell>` witness. It reads a directly borrowed cell, wraps its packet, reads through a
nested borrowed result, then consumes the outer owner and extracts the cell.
For the `nominal-main` target, argument file `[42]` has this independent result:

```json
{"stamp":42,"direct":42,"nested":42,"payload":42}
```

The full I64 range is supported. The `nominal-data` target accepts an ordinary
record. For `[{"name":"data λ","values":[-7,0,42]}]`, its result is:

```json
{"before":{"name":"data λ","values":[-7,0,42]},"after":{"name":"data λ","values":[-7,0,42]}}
```

`tasks.lkjc` transfers a complete nominal packet through a joined parallel worker.
The `nominal-transfer` target gives `{"stamp":42,"payload":42,"side":42}` for
`[42]`. `nominal-trap` takes a list of I64 and deliberately traps at index -1
while a borrowed cell is live. `nominal-fuel` takes a nonnegative iteration count,
retains its packet through native recursive calls, and finally returns 42.

Copy the executable and these literal files into an owned directory outside the
checkout. Consult [status](../../docs/status.md) for source acceptance and public
availability before choosing an executable. Create a minimal supplier and a
command consumer through `lkjscript new`. Prepend `request base=BASE` to each
proposal using the project's current `status`; plan, apply and check the supplier
through the native CLI. Export it with `package current export --kind transport
--output supplier.lkjp` before creating the consumer declarations.

Stage the observed exact transport in the consumer using `package dependency
stage --transport DIGEST --input-file supplier.lkjp`. Prepend this dependency
request and import to `application.lkjc`, substituting the export's observed
identities:

```text
add.dependency package=SUPPLIER_ID semantic-revision=SUPPLIER_SEMANTIC package-revision=SUPPLIER_PACKAGE_REVISION
declarations.begin
(units (use nominal SUPPLIER_ID SUPPLIER_PACKAGE_REVISION))
declarations.end
```

Plan, apply and check. Author `tasks.lkjc` in the same consumer using the same
`nominal` import, then build with `build --output owned-nominals.lkja`. Keep the
artifact next to `owned-nominals.deployment.json`. No grants, secrets or
configuration are needed. Detached execution uses:

```sh
./lkjscript run --deployment owned-nominals.deployment.json \
  --arguments-file arguments.json --result-file result.json
```

The five `native_owned_nominals_` public tests use copied executables, retained
literal inputs and independent expected outputs. They check canonical re-entry,
exact nominal identity, foreign and same-package representation refusal, nested
borrow origins, complete unused/untaken admission, recursive-layout rejection,
empty/data-only affinity, parallel transfer, traps and finite fuel refusal. They
remove source projects and transports before repeating artifact execution, check
joined cleanup and run valid invocations after failures. One test changes private
metadata from I64 to an ordinary record wrapper, explicitly updates the consumer
dependency and executes both artifacts with the same downstream literal text.
These are verification scenarios; acceptance requires their passing results.

The export test also authors a small three-package chain from
`opaque-leaf.lkjc`, `opaque-wrapper.lkjc` and `opaque-consumer.lkjc`. The leaf
package C owns a public `Leaf` with a private ordinary field. Package B owns a
public `Envelope` whose private child is that exact C nominal. Consumer A stages
and selects only B's complete transport and imports it as `wrapper`; B's supplier
imports C as `leaf`. A relays the complete owner and uses B's public read/finish
functions. The `nominal-opaque` target gives `{"before":42,"after":42}` for
`[42]`, including after all three projects and source transports are removed.
A direct exact reference to C is rejected because trusted transitive layout
admission does not grant direct authoring authority. B's fields remain private.
