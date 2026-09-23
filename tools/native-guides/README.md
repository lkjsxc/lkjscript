# Native public-guide tool

This ordinary lkjscript program renders the product's operation and diagnostic
reference pages. The built executable's `capabilities --generate-docs` and
`--verify-generated` routes use its embedded artifact. The native program owns
layout, field escaping and text assembly; Rust observes authoritative registry
metadata, admits typed arguments, executes the pure command and publishes derived
files. Six other generated reference pages remain implemented in Rust.

This is a maintained native development tool, not compiler self-hosting and not
an external semantic generator. It needs no Python, Node, shell subprocess or
compiler checkout at runtime. The bootstrap compiler and platform adapters remain
Rust. The initial implementation uses exactly the existing standard library.

## Authority and regeneration

`project/` is the accepted editable meaning repository, with two modules:
`markup` owns reusable HTML tables, cells, escaped code blocks and balanced Text
concatenation; `guides` owns typed operation, template, diagnostic and exit inputs
and the two public command targets. The original literal proposal is retained in
`requests/create.lkjc`; it is not a second synchronized authority. Its `BASE`
placeholder is replaced only when reproducing fresh authoring.

From the compiler repository root, using a compatible installed executable:

```sh
lkjscript --project tools/native-guides/project check
lkjscript --project tools/native-guides/project query find module guides
lkjscript --project tools/native-guides/project change draft --owner MODULE --output edit.lkjc
lkjscript --project tools/native-guides/project change plan --input-file edit.lkjc --output edit.logical-plan
lkjscript --project tools/native-guides/project change apply --input-file edit.lkjc --plan TOKEN
lkjscript --project tools/native-guides/project build --output rebuilt-guides.lkja
```

Replace `MODULE` and `TOKEN` with the observed exact identities. An untouched draft
plans as unchanged and has no publication token. Check runs eight native tests
and 45 tests from the retained exact standard supplier, differentially. A checked
build of unchanged authority must match `generated/guides.lkja` byte for byte.
After a reviewed edit, replace that derived artifact with the accepted build,
rebuild the host executable, then regenerate and verify `docs/generated/` through
the public capabilities route. Do not manually edit graph storage or compiled bytes.
A failed artifact build does not roll back an accepted source edit.

The maintained public CLI test owns fresh copied-product authoring, canonical
re-entry, independent expected output, old/new detached execution and exact
artifact regeneration. It is included in the existing workspace/source acceptance
owner; no new receipt or parallel test authority is introduced.

## Ordinary command inputs

`operations` takes four positional JSON arguments: a Build record, Operation
records, Template records and runner-observation Text. `diagnostics` takes three:
a Build record, Diagnostic records and Exit records. Record field names come from
the accepted native declarations, not from positional field order. For example:

```sh
lkjscript --project tools/native-guides/project run diagnostics --arguments '[{"product":"demo","version":"1","capabilities":"example"},[{"code":"bad_input","class":"source","meaning":"Invalid input","retry":"Repair it"}],[{"status":1,"meaning":"Rejected"}]]' --result-file diagnostics.json
```

The output file contains one JSON Text value. The host adapter decodes that value
to Markdown bytes. Larger inputs use `--arguments-file`. Build and copy the bundle
with a compatible executable and an ordinary Command deployment descriptor to run
these targets after removing the authoring project; no grant or secret is needed.

The generated Markdown now embeds HTML tables and escaped `pre` blocks. Pipes,
backticks, quotes, angle brackets and Unicode in metadata are content, not table
syntax or HTML markup. Markdown renderers must support ordinary HTML tables;
plain-source consumers see HTML rather than pipe-table notation. Commands, compact
capability records, diagnostic codes, graph/artifact formats and application data
are unchanged. The embedded program is pure and cannot opt into tasks or component
grants; normal typed input/output bounds and cancellation remain in force.

Balanced concatenation avoids repeatedly copying the entire document prefix. This
is an algorithmic property, not a claim that interpreting this program beats the
former Rust formatter. Record actual cold generation costs before publication.
