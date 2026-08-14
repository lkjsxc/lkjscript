# lkjscript

`lkjscript` is a source-free, semantic-graph-first programming system. Agents send typed queries and
revision-checked transactions to `lkjscriptd`; the daemon owns durable immutable Semantic Program
Graph revisions and compiles complete entries directly to one verified Core IR and interpreter.
A `.lkjscript` file is a canonical semantic snapshot, not source code.

There is no parser, syntax tree, bytecode VM, JIT, native tier, compatibility reader, or public
network service in the active product.

## Build and verify

```sh
cargo build --workspace --locked
cargo test --workspace --all-targets --all-features --locked
```

A larger deterministic malformed-boundary mutation smoke (not coverage-guided fuzzing) is available
as an ignored release test:

```sh
LKJSCRIPT_MUTATION_SEED=1 LKJSCRIPT_MUTATION_CASES=10000 \
  cargo test --release boundary_mutation_smoke --locked -- \
  --ignored --nocapture --test-threads=1
```

Linux x86-64 with a current stable Rust toolchain is the bootstrap host.

## Generic machine interface

Start the daemon with a private explicit state directory:

```sh
STATE=/tmp/lkjscript-state
mkdir -p "$STATE"
cargo run --locked --bin lkjscriptd -- --state "$STATE" --foreground
```

In another shell, create a workspace. `rpc` reads exactly one strict version-2 JSON envelope from
stdin and emits exactly one typed response:

```sh
STATE=/tmp/lkjscript-state
printf '%s' '{"version":2,"request_id":1,"request":{"kind":"create_workspace"}}' |
  cargo run --quiet --locked --bin lkjscript -- --state "$STATE" rpc
```

Inspect the complete runtime-derived operation, transaction, query, error, ID, and limit vocabulary
without starting a daemon (or send the `describe_schema` request through `rpc`):

```sh
cargo run --quiet --locked --bin lkjscript -- schema --pretty
printf '%s' '{"version":2,"request_id":2,"request":{"kind":"describe_schema"}}' |
  cargo run --quiet --locked --bin lkjscript -- --state "$STATE" rpc
```

Use the workspace ID from creation to commit a complete program that returns `42`. This exact fresh
workspace transaction requests the stable function binding for local handle 3; no source text or
whole-workspace replacement is accepted:

```sh
WORKSPACE=0123456789abcdef0123456789abcdef  # replace with returned workspace
cat <<JSON | cargo run --quiet --locked --bin lkjscript -- --state "$STATE" rpc
{
  "version": 2,
  "request_id": 3,
  "request": {
    "kind": "apply_transaction",
    "data": {
      "transaction": {
        "workspace": "$WORKSPACE",
        "base_revision": 0,
        "mode": "commit",
        "operations": [
          {"kind":"create_package","data":{"handle":1,"name":"app"}},
          {"kind":"create_module","data":{"handle":2,"package":{"kind":"local","data":1},"name":"main"}},
          {"kind":"create_function","data":{"handle":3,"module":{"kind":"local","data":2},"name":"main","result":"i64"}},
          {"kind":"create_region","data":{"handle":4,"function":{"kind":"local","data":3}}},
          {"kind":"create_block","data":{"handle":5,"region":{"kind":"local","data":4}}},
          {"kind":"create_operation","data":{"handle":6,"block":{"kind":"local","data":5},"before":null,"operation":{"kind":"const_i64","data":42}}},
          {"kind":"create_operation","data":{"handle":7,"block":{"kind":"local","data":5},"before":null,"operation":{"kind":"return","data":{"value":{"kind":"operation_result","data":{"operation":{"kind":"local","data":6},"output":0}}}}}},
          {"kind":"set_function_body","data":{"function":{"kind":"local","data":3},"region":{"kind":"local","data":4}}},
          {"kind":"set_entry_function","data":{"package":{"kind":"local","data":1},"function":{"kind":"local","data":3}}}
        ]
      },
      "response": {"return_handles":[3]}
    }
  }
}
JSON
```

Query the committed immutable revision:

```sh
printf '%s' \
  "{\"version\":2,\"request_id\":4,\"request\":{\"kind\":\"query_batch\",\"data\":{\"workspace\":\"$WORKSPACE\",\"revision\":1,\"queries\":[{\"id\":1,\"query\":{\"kind\":\"workspace_summary\"}}]}}}" |
  cargo run --quiet --locked --bin lkjscript -- --state "$STATE" rpc
```

Run that retained revision using the returned handle-3 binding. For the exact fresh-workspace
transaction above it is the canonical decimal Node ID `$WORKSPACE:4`:

```sh
REVISION=1
ENTRY="$WORKSPACE:4"
printf '%s' \
  "{\"version\":2,\"request_id\":5,\"request\":{\"kind\":\"run\",\"data\":{\"workspace\":\"$WORKSPACE\",\"revision\":$REVISION,\"entry\":\"$ENTRY\"}}}" |
  cargo run --quiet --locked --bin lkjscript -- --state "$STATE" rpc
```

Shut down only through the typed request:

```sh
printf '%s' '{"version":2,"request_id":6,"request":{"kind":"shutdown"}}' |
  cargo run --quiet --locked --bin lkjscript -- --state "$STATE" rpc
```

Machine stdout is one JSON response; local usage/JSON errors exit 2, transport errors 3, and
output/conversion errors 4. Typed daemon rejection is still a valid response and exits 0.

The current language implements `unit`, `bool`, `i64`, constants, checked `add_i64`, typed holes,
and `return`. See [status](docs/status.md), [protocol](docs/spec/protocol.md), and the
[evidence-gated roadmap](docs/roadmap.md).
