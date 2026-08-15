#!/usr/bin/env python3
"""Production CLI/daemon structured-pure repair example (Python 3 stdlib only)."""

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
    envelope = {"version": 3, "request_id": request_id, "request": request}
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


def local(handle):
    return {"kind": "local", "data": handle}


def existing(node):
    return {"kind": "existing", "data": node}


def result(handle):
    return {"kind": "operation_result", "data": {"operation": local(handle), "output": 0}}


def parameter(handle):
    return {"kind": "function_parameter", "data": local(handle)}


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
    outcome = response["results"][0]["outcome"]
    return expect(outcome, "success")


def run(workspace, revision, entry, arguments):
    return rpc({
        "kind": "run",
        "data": {
            "workspace": workspace,
            "revision": revision,
            "entry": entry,
            "arguments": arguments,
            "policy": {"fuel": 1_000_000, "maximum_frames": 10_000},
        },
    })


def main():
    global state
    with tempfile.TemporaryDirectory(prefix="lkjscript-structured-pure-") as directory:
        state = pathlib.Path(directory)
        os.chmod(state, 0o700)
        start_daemon()
        created = expect(rpc({"kind": "create_workspace"}), "workspace_created")
        workspace = created["workspace"]

        operations = [
            {"kind": "create_package", "data": {"handle": 1, "name": "app"}},
            {"kind": "create_module", "data": {"handle": 2, "package": local(1), "name": "structured"}},
            {"kind": "create_function", "data": {
                "handle": 10, "module": local(2), "name": "range_sum",
                "parameters": [{"handle": 11, "name": "n", "ty": "i64"}], "result": "i64",
                "body": {"operations": [
                    {"handle": 12, "operation": {"kind": "const_i64", "data": 0}},
                    {"handle": 13, "operation": {"kind": "for_i64", "data": {
                        "start": result(12), "end_exclusive": parameter(11), "step": 1,
                        "initial": result(12), "carried": "i64", "index_handle": 14,
                        "carried_handle": 15, "body": {"operations": [
                            {"handle": 16, "operation": {"kind": "hole", "data": {"expected": "i64"}}}
                        ], "yield_value": result(16)}
                    }}}
                ], "return_value": result(13)}
            }},
            {"kind": "create_function", "data": {
                "handle": 20, "module": local(2), "name": "normalize_and_sum",
                "parameters": [{"handle": 21, "name": "n", "ty": "i64"}], "result": "i64",
                "body": {"operations": [
                    {"handle": 22, "operation": {"kind": "const_i64", "data": 0}},
                    {"handle": 23, "operation": {"kind": "lt_i64", "data": {"lhs": parameter(21), "rhs": result(22)}}},
                    {"handle": 24, "operation": {"kind": "if", "data": {
                        "condition": result(23), "result": "i64",
                        "then_body": {"operations": [], "yield_value": result(22)},
                        "else_body": {"operations": [{"handle": 25, "operation": {"kind": "call", "data": {"function": local(10), "arguments": [parameter(21)]}}}], "yield_value": result(25)}
                    }}}
                ], "return_value": result(24)}
            }},
            {"kind": "create_function", "data": {
                "handle": 30, "module": local(2), "name": "main", "parameters": [], "result": "i64",
                "body": {"operations": [
                    {"handle": 31, "operation": {"kind": "const_i64", "data": 101}},
                    {"handle": 32, "operation": {"kind": "call", "data": {"function": local(20), "arguments": [result(31)]}}}
                ], "return_value": result(32)}
            }},
            {"kind": "set_entry_function", "data": {"package": local(1), "function": local(30)}},
        ]
        receipt = expect(rpc({"kind": "apply_transaction", "data": {
            "transaction": {"workspace": workspace, "base_revision": 0, "mode": "commit", "operations": operations},
            "response": {"return_handles": [10, 16, 20, 30]},
        }}), "transaction_receipt")
        bindings = {binding[0]: binding[1] for binding in receipt["returned_bindings"]}
        if len(bindings) != 4 or receipt["created_count"] != 36:
            raise RuntimeError("unexpected structured receipt")
        range_sum, hole, normalize, entry = bindings[10], bindings[16], bindings[20], bindings[30]

        context = expect(query(workspace, 1, {"kind": "repair_context", "data": {
            "target": {"kind": "hole", "data": hole},
            "budget": {"body_before": 4, "body_after": 4, "visible_values": 16, "incoming_uses": 8, "include_incompatible": True},
        }}), "repair_context")
        if context["operation"] != hole or context["owner_function"] != range_sum:
            raise RuntimeError("repair context target/owner mismatch")
        if context["function_signature"] != {"parameter_count": 1, "result": "i64"}:
            raise RuntimeError("repair context signature mismatch")
        if context["owner_chain"][0]["node"] != hole or not any(
            fact["node"] == range_sum for fact in context["owner_chain"]
        ):
            raise RuntimeError("repair context owner chain mismatch")
        enclosing = [fact for fact in context["enclosing_regions"] if fact["role"] == "for_body"]
        if len(enclosing) != 1:
            raise RuntimeError("repair context lacks enclosing for_body")
        facts = {fact["role"]: fact for fact in context["visible_block_arguments"]}
        index = facts["loop_index"]["argument"]
        carried = facts["loop_carried"]["argument"]
        if facts["loop_index"]["ordinal"] != 0 or facts["loop_carried"]["ordinal"] != 1:
            raise RuntimeError("loop argument roles/ordinals mismatch")
        body_codes = [item["code"] for item in context["body_window"]]
        hole_at = body_codes.index("hole")
        if body_codes[hole_at:hole_at + 2] != ["hole", "yield"]:
            raise RuntimeError("repair body window lacks hole/yield")
        if not any(
            value["value"]["kind"] == "function_parameter"
            and value["owner_function"] == range_sum
            for value in context["visible_values"]["items"]
        ):
            raise RuntimeError("repair context lacks visible function parameter")
        if not any(
            value.get("producer_code") == "const_i64" and value.get("ordinal") == 0
            for value in context["visible_values"]["items"]
        ):
            raise RuntimeError("repair context lacks prior zero")
        if not any(use["operand_index"] == 0 for use in context["incoming_uses"]["items"]):
            raise RuntimeError("repair context lacks incoming yield use")
        if context.get("blocker", {}).get("target") != hole:
            raise RuntimeError("repair context lacks exact blocker")
        adds = [constructor for constructor in context["legal_constructors"] if constructor["code"] == "add_i64"]
        if len(adds) != 1 or not adds[0]["direct_refinement"] or adds[0]["operand_types"] != ["i64", "i64"]:
            raise RuntimeError("repair context lacks exact direct add_i64 constructor")

        invalid = rpc({"kind": "apply_transaction", "data": {
            "transaction": {"workspace": workspace, "base_revision": 1, "mode": "commit", "operations": [
                {"kind": "refine_hole", "data": {"hole": existing(hole), "replacement": {"kind": "const_bool", "data": True}}}
            ]}, "response": {"return_handles": []},
        }})
        expect(invalid, "error")
        summary = expect(query(workspace, 1, {"kind": "workspace_summary"}), "workspace_summary")
        if summary["complete"]:
            raise RuntimeError("invalid refinement published")

        repaired = expect(rpc({"kind": "apply_transaction", "data": {
            "transaction": {"workspace": workspace, "base_revision": 1, "mode": "commit", "operations": [
                {"kind": "refine_hole", "data": {"hole": existing(hole), "replacement": {"kind": "add_i64", "data": {
                    "lhs": {"kind": "block_argument", "data": existing(carried)},
                    "rhs": {"kind": "block_argument", "data": existing(index)},
                }}}}
            ]}, "response": {"return_handles": []},
        }}), "transaction_receipt")
        if repaired["created_count"] != 0:
            raise RuntimeError("refinement changed identity")

        diff = expect(query(workspace, 2, {"kind": "semantic_diff", "data": {
            "from": 1, "page": {"limit": 32}
        }}), "semantic_diff")
        if not any(change["node"] == hole and change["kind"]["kind"] == "operation_refined" for change in diff["page"]["items"]):
            raise RuntimeError("OperationRefined missing")

        expected_runs = [
            (entry, [], 5050),
            (normalize, [{"kind": "i64", "data": -3}], 0),
            (normalize, [{"kind": "i64", "data": 11}], 55),
        ]
        for function, arguments, expected_value in expected_runs:
            value = expect(run(workspace, 2, function, arguments), "run")["value"]
            if value != {"kind": "i64", "data": expected_value}:
                raise RuntimeError(f"unexpected run value: {value}")
            print(json.dumps(value, separators=(",", ":")))

        expect(run(workspace, 1, entry, []), "error")
        stop_daemon()
        print(json.dumps({"kind": "acknowledged"}, separators=(",", ":")))
        start_daemon()
        for revision in (1, 2):
            for node in (hole, range_sum, normalize, entry):
                view = expect(query(workspace, revision, {"kind": "node", "data": {"node": node, "expand": False}}), "node")
                if view["summary"]["node"] != node:
                    raise RuntimeError("retained identity changed")
        expect(run(workspace, 1, entry, []), "error")
        value = expect(run(workspace, 2, entry, []), "run")["value"]
        if value != {"kind": "i64", "data": 5050}:
            raise RuntimeError("restart result changed")
        print(json.dumps({"restart_verified": value}, separators=(",", ":")))
        stop_daemon()


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
