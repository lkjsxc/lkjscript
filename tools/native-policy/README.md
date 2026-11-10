# Native byte-classification example

This pure lkjscript program is a maintained fixture for public authorship,
artifact reproduction and native-tool embedding. The public CLI tests in
`tests/public_cli/native_policy.rs` rebuild it from accepted meaning, author a fresh
copy, edit it and execute detached artifacts. Routine verification is described in
[Development checks](../../docs/spec/verification.md).

## Meaning and inputs

`project/` contains the accepted editable program. `requests/create-bytes.lkjc`
is its fresh-authoring input, and `generated/policy.lkja` is derived by public
`build`. Changes go through the ordinary authoring commands and a rebuilt artifact.

| Pure command | One positional argument | Result |
| --- | --- | --- |
| `extensions` | List of Text | List of Bool |
| `shebangs` | List of Bytes | List of Bool |

`extensions` recognizes ASCII case variants of `py`. `shebangs` recognizes a
leading `#!` and an ASCII case-insensitive `python` substring before the first LF
within a 512-byte window. The byte window is part of this example's classifier
semantics. CR, NUL and invalid UTF-8 remain raw bytes.

## Reproduction

Use a development-built `lkjscript` executable:

```sh
lkjscript --project tools/native-policy/project check
lkjscript --project tools/native-policy/project build --output rebuilt-policy.lkja
lkjscript --project tools/native-policy/project run extensions --arguments '[["py","rs","PY"]]'
lkjscript --project tools/native-policy/project run shebangs --arguments '[[{"$bytes":"IyFweXRob24="}]]'
```

The final command returns `[true]` for raw `#!python`. Bytes use the language's
canonical padded Base64 JSON representation. The shared `PureTool` embedding
performs strict artifact loading and typed pure invocation.
