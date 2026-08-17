# Named records, variants, and complete handling

This focused example uses strict protocol-v9 JSON through the direct production CLI. It creates a
named `Reading` record, an `Input` variant with payload and payload-free alternatives, constructors,
projections, complete lazy matching, and a `main` function containing one typed hole.

The driver obtains exact contract facts, requests bounded repair context, proves a field/type-invalid
repair is atomic, repairs the same durable anchor, reviews the change, and runs scalar and nominal
public values. Its oracles include every variant, selected and unselected overflow paths, workspace
reopen, historical revisions, and stable durable identities.

Run from the repository root:

```sh
./examples/named-data/run.sh
```

The script uses production release binaries and a private temporary state directory. JSON is a strict
proposal projection, not stored program authority.
