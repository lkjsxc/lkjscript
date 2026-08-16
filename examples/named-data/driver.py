#!/usr/bin/env python3
"""Named records, fixed variants, complete lazy handling, and repair via the public CLI."""

import json
import os
import pathlib
import subprocess
import sys
import tempfile
import time

CLI = pathlib.Path(sys.argv[1]).resolve()
DAEMON = pathlib.Path(sys.argv[2]).resolve()
request_id = 0
daemon = None
state = None


def rpc(request):
    global request_id
    request_id += 1
    envelope = {"version": 4, "request_id": request_id, "request": request}
    completed = subprocess.run(
        [str(CLI), "--state", str(state), "rpc"],
        input=json.dumps(envelope, separators=(",", ":")).encode(),
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        check=False,
    )
    if completed.returncode != 0:
        raise RuntimeError(f"CLI failed ({completed.returncode}): {completed.stderr.decode()}")
    response = json.loads(completed.stdout)
    if response["request_id"] != request_id:
        raise RuntimeError("response correlation mismatch")
    return response["response"]


def expect(response, kind):
    if response.get("kind") != kind:
        raise RuntimeError(f"expected {kind}, received {response}")
    return response.get("data")


def expect_error(response, code):
    error = expect(response, "error")
    if error["code"] != code:
        raise RuntimeError(f"expected error {code}, received {error}")
    return error


def local(handle):
    return {"kind": "local", "data": handle}


def existing(node):
    return {"kind": "existing", "data": node}


def nominal(target):
    return {"nominal": target}


def result(handle):
    return {"kind": "operation_result", "data": {"operation": local(handle), "output": 0}}


def parameter(handle):
    return {"kind": "function_parameter", "data": local(handle)}


def payload(handle):
    return {"kind": "block_argument", "data": local(handle)}


def expression(handle, kind, data=None):
    operation = {"kind": kind}
    if data is not None:
        operation["data"] = data
    return {"handle": handle, "operation": operation}


def yielding(operations, value):
    return {"operations": operations, "yield_value": value}


def function(handle, name, parameters, result_type, operations, return_value):
    return {
        "kind": "create_function",
        "data": {
            "handle": handle,
            "module": local(2),
            "name": name,
            "parameters": parameters,
            "result": result_type,
            "body": {"operations": operations, "return_value": return_value},
        },
    }


def field(field_handle, value):
    return {"field": local(field_handle), "value": value}


def arm(variant, body, payload_handle=None):
    value = {"variant": local(variant), "body": body}
    if payload_handle is not None:
        value["payload_handle"] = payload_handle
    return value


def start_daemon():
    global daemon
    daemon = subprocess.Popen(
        [str(DAEMON), "--state", str(state), "--foreground"],
        stdout=subprocess.DEVNULL,
        stderr=subprocess.PIPE,
    )
    endpoint = state / "lkjscript.sock"
    deadline = time.monotonic() + 5
    while not endpoint.exists():
        if daemon.poll() is not None:
            raise RuntimeError(f"daemon exited early: {daemon.stderr.read().decode()}")
        if time.monotonic() >= deadline:
            raise RuntimeError("daemon readiness timeout")
        time.sleep(0.001)


def stop_daemon():
    global daemon
    if daemon is None:
        return
    if daemon.poll() is None:
        expect(rpc({"kind": "shutdown"}), "acknowledged")
        if daemon.wait(timeout=5) != 0:
            raise RuntimeError("daemon shutdown failed")
    daemon = None


def query(workspace, revision, query_value):
    response = expect(
        rpc({
            "kind": "query_batch",
            "data": {
                "workspace": workspace,
                "revision": revision,
                "queries": [{"id": request_id + 1, "query": query_value}],
            },
        }),
        "query_batch_result",
    )
    return expect(response["results"][0]["outcome"], "success")


def run(workspace, revision, entry, arguments, fuel=1_000_000):
    return rpc({
        "kind": "run",
        "data": {
            "workspace": workspace,
            "revision": revision,
            "entry": entry,
            "arguments": arguments,
            "policy": {"fuel": fuel, "maximum_frames": 1_000},
        },
    })


def reading_value(ids, value, valid):
    return {
        "kind": "product",
        "data": {
            "ty": ids[3],
            # Reversed input proves exact identity-keyed normalization.
            "fields": [
                {"field": ids[5], "value": {"kind": "bool", "data": valid}},
                {"field": ids[4], "value": {"kind": "i64", "data": value}},
            ],
        },
    }


def input_value(ids, variant_handle, value=None):
    data = {"ty": ids[6], "variant": ids[variant_handle]}
    if value is not None:
        data["payload"] = value
    return {"kind": "sum", "data": data}


def application_operations():
    evaluate = function(
        10,
        "evaluate",
        [{"handle": 11, "name": "input", "ty": nominal(local(6))}],
        "i64",
        [expression(12, "match_sum", {
            "scrutinee": parameter(11),
            "result": "i64",
            # Deliberately not declaration order; canonical expansion normalizes by variant identity.
            "arms": [
                arm(9, yielding([], payload(19)), 19),
                arm(7, yielding([
                    expression(14, "project_field", {"value": payload(13), "field": local(4)}),
                    expression(15, "project_field", {"value": payload(13), "field": local(5)}),
                    expression(16, "const_i64", 0),
                    expression(17, "if", {
                        "condition": result(15),
                        "result": "i64",
                        "then_body": yielding([], result(14)),
                        "else_body": yielding([], result(16)),
                    }),
                ], result(17)), 13),
                arm(8, yielding([expression(18, "const_i64", 0)], result(18))),
            ],
        })],
        result(12),
    )
    main_function = function(
        30, "main", [], "i64",
        [
            expression(31, "const_i64", 42),
            expression(32, "const_bool", True),
            expression(33, "hole", {"expected": nominal(local(3))}),
            expression(34, "construct_variant", {"variant": local(7), "payload": result(33)}),
            expression(35, "call", {"function": local(10), "arguments": [result(34)]}),
        ],
        result(35),
    )
    disabled = function(
        40, "evaluate_disabled", [{"handle": 41, "name": "value", "ty": "i64"}], "i64",
        [
            expression(42, "const_bool", False),
            expression(43, "construct_product", {
                "product": local(3), "fields": [field(5, result(42)), field(4, parameter(41))],
            }),
            expression(44, "construct_variant", {"variant": local(7), "payload": result(43)}),
            expression(45, "call", {"function": local(10), "arguments": [result(44)]}),
        ],
        result(45),
    )
    missing = function(
        50, "evaluate_missing", [], "i64",
        [
            expression(51, "construct_variant", {"variant": local(8)}),
            expression(52, "call", {"function": local(10), "arguments": [result(51)]}),
        ],
        result(52),
    )
    override = function(
        55, "evaluate_override", [{"handle": 56, "name": "value", "ty": "i64"}], "i64",
        [
            expression(57, "construct_variant", {"variant": local(9), "payload": parameter(56)}),
            expression(58, "call", {"function": local(10), "arguments": [result(57)]}),
        ],
        result(58),
    )
    make_reading = function(
        60, "make_reading",
        [{"handle": 61, "name": "value", "ty": "i64"}, {"handle": 62, "name": "valid", "ty": "bool"}],
        nominal(local(3)),
        [expression(63, "construct_product", {
            "product": local(3), "fields": [field(5, parameter(62)), field(4, parameter(61))],
        })],
        result(63),
    )
    lazy_probe = function(
        70, "lazy_match_probe", [{"handle": 71, "name": "input", "ty": nominal(local(6))}], "i64",
        [expression(72, "match_sum", {
            "scrutinee": parameter(71), "result": "i64", "arms": [
                arm(7, yielding([expression(74, "const_i64", 0)], result(74)), 73),
                arm(8, yielding([expression(75, "const_i64", 0)], result(75))),
                arm(9, yielding([
                    expression(77, "const_i64", 9_223_372_036_854_775_807),
                    expression(78, "const_i64", 1),
                    expression(79, "add_i64", {"lhs": result(77), "rhs": result(78)}),
                ], result(79)), 76),
            ],
        })],
        result(72),
    )
    return [
        {"kind": "create_package", "data": {"handle": 1, "name": "reading-app"}},
        {"kind": "create_module", "data": {"handle": 2, "package": local(1), "name": "root"}},
        evaluate, main_function, disabled, missing, override, make_reading, lazy_probe,
        {"kind": "create_product_type", "data": {
            "handle": 3, "module": local(2), "name": "Reading", "fields": [
                {"handle": 4, "name": "value", "ty": "i64"},
                {"handle": 5, "name": "valid", "ty": "bool"},
            ],
        }},
        {"kind": "create_sum_type", "data": {
            "handle": 6, "module": local(2), "name": "Input", "variants": [
                {"handle": 7, "name": "sample", "payload": nominal(local(3))},
                {"handle": 8, "name": "missing"},
                {"handle": 9, "name": "override", "payload": "i64"},
            ],
        }},
        {"kind": "set_entry_function", "data": {"package": local(1), "function": local(30)}},
    ]


def assert_run_i64(workspace, revision, entry, arguments, expected):
    value = expect(run(workspace, revision, entry, arguments), "run")["value"]
    if value != {"kind": "i64", "data": expected}:
        raise RuntimeError(f"unexpected i64 Run result: {value}")


def run_oracles(workspace, ids):
    assert_run_i64(workspace, 2, ids[30], [], 42)
    assert_run_i64(workspace, 2, ids[40], [{"kind": "i64", "data": 99}], 0)
    assert_run_i64(workspace, 2, ids[50], [], 0)
    assert_run_i64(workspace, 2, ids[55], [{"kind": "i64", "data": 17}], 17)
    assert_run_i64(workspace, 2, ids[10], [input_value(ids, 7, reading_value(ids, 5, True))], 5)
    assert_run_i64(workspace, 2, ids[10], [input_value(ids, 7, reading_value(ids, 5, False))], 0)
    assert_run_i64(workspace, 2, ids[10], [input_value(ids, 8)], 0)
    assert_run_i64(workspace, 2, ids[10], [input_value(ids, 9, {"kind": "i64", "data": 23})], 23)
    output = expect(run(workspace, 2, ids[60], [
        {"kind": "i64", "data": 9}, {"kind": "bool", "data": True},
    ]), "run")["value"]
    expected_output = {
        "kind": "product",
        "data": {"ty": ids[3], "fields": [
            {"field": ids[4], "value": {"kind": "i64", "data": 9}},
            {"field": ids[5], "value": {"kind": "bool", "data": True}},
        ]},
    }
    if output != expected_output:
        raise RuntimeError(f"unexpected nominal Run output: {output}")
    # The overflow arm must not run when `missing` is selected.
    assert_run_i64(workspace, 2, ids[70], [input_value(ids, 8)], 0)
    selected = run(workspace, 2, ids[70], [input_value(ids, 9, {"kind": "i64", "data": 1})])
    expect_error(selected, "runtime_trap")
    # A runtime trap must not poison the daemon; prove Run remains usable before shutdown.
    assert_run_i64(workspace, 2, ids[30], [], 42)


def main():
    global state
    with tempfile.TemporaryDirectory(prefix="lkjscript-named-data-") as directory:
        state = pathlib.Path(directory)
        os.chmod(state, 0o700)
        start_daemon()

        manifest = expect(expect(rpc({"kind": "describe_schema", "data": {
            "projection": {"kind": "manifest"},
        }}), "describe_schema"), "manifest")
        digest = manifest["digest"]
        sections = [
            "semantic_types_and_nodes", "nominal_declarations", "transactions_and_expressions",
            "queries_and_repair", "runtime_and_run", "errors_and_limits",
        ]
        section_result = expect(expect(rpc({"kind": "describe_schema", "data": {
            "projection": {"kind": "sections", "data": {"sections": sections}},
        }}), "describe_schema"), "sections")
        if section_result["digest"] != digest or len(section_result["sections"]) != 6:
            raise RuntimeError("six-section schema projection mismatch")
        unchanged = expect(expect(rpc({"kind": "describe_schema", "data": {
            "projection": {"kind": "full"}, "known_digest": digest,
        }}), "describe_schema"), "unchanged")
        if unchanged["digest"] != digest:
            raise RuntimeError("known-digest response mismatch")

        created_workspace = expect(rpc({"kind": "create_workspace"}), "workspace_created")
        workspace = created_workspace["workspace"]
        handles = [3, 4, 5, 6, 7, 8, 9, 10, 30, 31, 32, 33, 40, 50, 55, 60, 70, 79]
        receipt = expect(rpc({"kind": "apply_transaction", "data": {
            "transaction": {
                "workspace": workspace, "base_revision": 0, "mode": "commit",
                "idempotency_key": "81818181818181818181818181818181",
                "operations": application_operations(),
            },
            "response": {"return_handles": handles},
        }}), "transaction_receipt")
        ids = {handle: node for handle, node in receipt["returned_bindings"]}
        if set(ids) != set(handles) or receipt["created_count"] != 97 or receipt["complete_after"]:
            raise RuntimeError("unexpected nominal structured receipt")

        context = expect(query(workspace, 1, {"kind": "repair_context", "data": {
            "target": {"kind": "hole", "data": ids[33]},
            "budget": {
                "body_before": 8, "body_after": 8, "visible_values": 16,
                "incoming_uses": 8, "include_incompatible": True,
            },
        }}), "repair_context")
        if context["operation"] != ids[33] or context["expected_type"] != {"nominal": ids[3]}:
            raise RuntimeError("repair context target/type mismatch")
        nominal_context = context.get("nominal_type")
        if nominal_context is None or nominal_context["declaration"] != ids[3]:
            raise RuntimeError("repair context lacks Reading declaration")
        constructors = [item for item in context["legal_constructors"] if item["code"] == "construct_product"]
        if len(constructors) != 1 or constructors[0]["members"] != [ids[4], ids[5]]:
            raise RuntimeError("repair context lacks exact Reading constructor")

        def refinement(valid):
            value, boolean = (ids[31], ids[32]) if valid else (ids[32], ids[31])
            return {"kind": "apply_transaction", "data": {
                "transaction": {
                    "workspace": workspace, "base_revision": 1, "mode": "commit",
                    "operations": [{"kind": "refine_hole", "data": {
                        "hole": existing(ids[33]),
                        "replacement": {"kind": "construct_product", "data": {
                            "product": existing(ids[3]),
                            "fields": [
                                {"field": existing(ids[5]), "value": {"kind": "operation_result", "data": {"operation": existing(boolean), "output": 0}}},
                                {"field": existing(ids[4]), "value": {"kind": "operation_result", "data": {"operation": existing(value), "output": 0}}},
                            ],
                        }},
                    }}],
                },
                "response": {"return_handles": []},
            }}

        expect_error(rpc(refinement(False)), "type_mismatch")
        summary = expect(query(workspace, 1, {"kind": "workspace_summary"}), "workspace_summary")
        if summary["revision"] != 1 or summary["complete"]:
            raise RuntimeError("invalid refinement changed durable state")
        expect_error(run(workspace, 1, ids[30], []), "compile_incomplete")

        repaired = expect(rpc(refinement(True)), "transaction_receipt")
        if repaired["revision"] != 2 or repaired["created_count"] != 0 or not repaired["complete_after"]:
            raise RuntimeError("valid refinement did not preserve identity")
        diff = expect(query(workspace, 2, {"kind": "semantic_diff", "data": {
            "from": 1, "page": {"limit": 8},
        }}), "semantic_diff")
        if not any(item["node"] == ids[33] and item["kind"]["kind"] == "operation_refined" for item in diff["page"]["items"]):
            raise RuntimeError("semantic diff lacks identity-preserving refinement")

        run_oracles(workspace, ids)
        stop_daemon()
        start_daemon()
        for revision in (1, 2):
            for node in ids.values():
                view = expect(query(workspace, revision, {"kind": "node", "data": {
                    "node": node, "expand": False,
                }}), "node")
                if view["summary"]["node"] != node:
                    raise RuntimeError("retained semantic identity changed")
        expect_error(run(workspace, 1, ids[30], []), "compile_incomplete")
        run_oracles(workspace, ids)
        stop_daemon()
        print(json.dumps({
            "schema": {"manifest": True, "sections": len(sections), "unchanged": True},
            "revisions": [1, 2],
            "repair": "operation_refined",
            "oracles": "scalar, nominal input/output, lazy and selected overflow passed",
            "restart": "passed",
            "shutdown": "acknowledged",
        }, separators=(",", ":")))


if __name__ == "__main__":
    try:
        main()
    finally:
        if daemon is not None and daemon.poll() is None:
            daemon.terminate()
            try:
                daemon.wait(timeout=5)
            except subprocess.TimeoutExpired:
                daemon.kill()
                daemon.wait()
