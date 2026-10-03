# Owned tasks with explicit authority

This maintained example uses three separately accepted native packages. The generic
library is checked and exported before any concrete implementation or consumer
exists. `tests/public_cli/native_owned_effects.rs` authors each literal proposal
through a copied public executable, checks untouched drafts, transports exact
packages, reviews an identity-preserving edit, and runs the artifact after removing
all three authoring projects and both package transports.

`library.lkjc` defines the closed `Storage` contract: a synchronous borrowed read
and a consuming replace. Its `transform<T,E,R>` task consumes an owned value,
uses caller-supplied clock requirement R, invokes an ordinary task callback under
caller-supplied effect row E, and returns the owner. The declared row is R + E.
`relay` and `nested-relay` forward all four applications explicitly: type, effect,
requirement and exact implementation witness. Clock results are discarded.

`carriers.lkjc` implements `Storage` for OwnedI64Cell and ByteBuffer. A second
implementation for the same OwnedI64Cell type reads 99, making witness selection
observable. Cell replacement retains the cell allocation. Buffer replacement
appends an octet and may grow its allocation; this example does not promise that
the entire buffer algorithm avoids reallocation.

`application.lkjc` supplies two independent clock requirements. The same Scalar
witness runs first with R = clock-a and E = clock-b, then with the bindings
reversed through a relay. Alternate and Octets also traverse nested forwarding.
The callbacks add one to the borrowed read result. The complete deterministic
result is:

```json
{"scalar":43,"rebound":43,"alternate":100,"bytes":{"$bytes":"SQI="}}
```

The ordinary cell changes 42 to 43. The buffer changes `[73]` to `[73,2]`.
The alternate cell changes 42 to 100. Execution performs eight clock operations,
four under each requirement, and does not use clock values in the result.

## Author through the product

Copy a compatible executable and these three files to an owned directory outside
the checkout. Create each package with the `command` template so the exact builtin
standard dependency is selected:

```sh
./lkjscript new library --template command --name owned-effects-library
./lkjscript new carriers --template command --name owned-effects-carriers
./lkjscript new application --template command --name owned-effects-application
```

For each request, prepend `request base=BASE` with that project's current `status`
revision. Accept the library literal first with `change plan --input-file FILE`,
then `change apply --input-file FILE --plan TOKEN`, and run `check`. Export it
using `package current export --kind transport --output library.lkjp`.

The export reports package id, semantic revision, package revision and transport
digest. Stage it in carriers and application using `package dependency stage
--transport DIGEST --input-file library.lkjp`. Before the carriers literal, add
the following exact dependency request and native import, substituting only
identities observed in the library export:

```text
add.dependency package=LIBRARY_ID semantic-revision=LIBRARY_SEMANTIC package-revision=LIBRARY_PACKAGE_REVISION
declarations.begin
(units (use owned-effects LIBRARY_ID LIBRARY_PACKAGE_REVISION))
declarations.end
```

Plan, apply, check and export carriers in the same way. Stage its transport in
application. Before the application literal, add both exact dependency requests
and native imports named `owned-effects` and `owned-effect-carriers`. An untouched
`change draft --module MODULE --output FILE` must plan as `unchanged`.

## Run without authoring sources

Build with `--project application build --output owned-effects.lkja`. Place the
artifact beside the supplied `owned-effects.deployment.json`. It selects target
`owned-effects` and grants `wall_clock` separately to `clock-a` and `clock-b`,
each with its own sharing domain and example authority revision. No secrets or
configuration values are needed. Then run:

```sh
./lkjscript run --deployment owned-effects.deployment.json --result-file result.json
```

The public acceptance test constructs this descriptor and checks both missing
grants and an incompatible adapter before successful execution. It also rejects
missing effect or requirement operands, an incorrect Self witness, mismatched
callback effects, insufficient caller effects, foreign authority and an invalid
witness in an untaken branch. Each rejected proposal preserves accepted HEAD.
