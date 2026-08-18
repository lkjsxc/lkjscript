#!/usr/bin/env python3
"""Exercise byte construction, repair, history, and restart through production binaries."""

import base64
import json
import os
import pathlib
import shutil
import subprocess
import sys
import tempfile
import time


ROOT = pathlib.Path(__file__).resolve().parents[2]
CLI = pathlib.Path(sys.argv[1]).resolve() if len(sys.argv) > 1 else ROOT / "target/release/lkjscript"
state = None
session = None
request_id = 0
query_id = 0
measurements = []
run_timings = []
session_processes = 0
application_measurements = []
release_measurements = []


def symbol(number):
    return f"draft_{number}"


def local(number):
    return {"kind": "draft", "data": symbol(number)}


def existing(node):
    return {"kind": "existing", "data": node}


def nominal(target):
    return {"nominal": target}


def result(number):
    return {
        "kind": "operation_result",
        "data": {"operation": local(number), "output": 0},
    }


def parameter(number):
    return {"kind": "function_parameter", "data": local(number)}


def block_argument(number):
    return {"kind": "block_argument", "data": local(number)}


def expression(number, kind, data=None):
    operation = {"kind": kind}
    if data is not None:
        data = dict(data) if isinstance(data, dict) else data
        if kind == "for_i64":
            data["index_symbol"] = symbol(data["index_symbol"])
            data["carried_symbol"] = symbol(data["carried_symbol"])
        operation["data"] = data
    return {"symbol": symbol(number), "operation": operation}


def yielding(operations, value):
    return {"operations": operations, "yield_value": value}


def function(number, name, parameters, result_type, operations, return_value):
    return {
        "kind": "create_function",
        "data": {
            "symbol": symbol(number),
            "module": local(2),
            "name": name,
            "parameters": [
                {**item, "symbol": symbol(item["symbol"])} for item in parameters
            ],
            "result": result_type,
            "body": {"operations": operations, "return_value": return_value},
        },
    }


def call(number, function_number, arguments):
    return expression(number, "call", {
        "function": local(function_number),
        "arguments": arguments,
    })


def encode_bytes(value):
    return base64.urlsafe_b64encode(bytes(value)).rstrip(b"=").decode("ascii")


def bytes_value(value):
    return {"kind": "bytes", "data": encode_bytes(value)}


def field(field_number, value):
    return {"field": local(field_number), "value": value}


def persisted_value(value):
    """Project a query ValueRef back into the strict transaction ValueDraft shape."""
    if value["kind"] in ("function_parameter", "block_argument"):
        return {"kind": value["kind"], "data": existing(value["data"])}
    if value["kind"] == "operation_result":
        return {
            "kind": "operation_result",
            "data": {
                "operation": existing(value["data"]["operation"]),
                "output": value["data"]["output"],
            },
        }
    raise RuntimeError(f"unsupported persisted repair value: {value}")


def eq_i64_function():
    return function(
        100,
        "eq_i64",
        [
            {"symbol": 101, "name": "lhs", "ty": "i64"},
            {"symbol": 102, "name": "rhs", "ty": "i64"},
        ],
        "bool",
        [
            expression(103, "lt_i64", {"lhs": parameter(101), "rhs": parameter(102)}),
            expression(104, "if", {
                "condition": result(103),
                "result": "bool",
                "then_body": yielding([
                    expression(105, "const_bool", False),
                ], result(105)),
                "else_body": yielding([
                    expression(106, "lt_i64", {
                        "lhs": parameter(102), "rhs": parameter(101),
                    }),
                    expression(107, "if", {
                        "condition": result(106),
                        "result": "bool",
                        "then_body": yielding([
                            expression(108, "const_bool", False),
                        ], result(108)),
                        "else_body": yielding([
                            expression(109, "const_bool", True),
                        ], result(109)),
                    }),
                ], result(107)),
            }),
        ],
        result(104),
    )


def canonicalize_function():
    return function(
        200,
        "canonicalize_payload",
        [
            {"symbol": 201, "name": "input", "ty": "bytes"},
            {"symbol": 202, "name": "length", "ty": "i64"},
        ],
        "bytes",
        [
            expression(203, "const_bytes", encode_bytes(b"")),
            expression(204, "const_i64", 1),
            expression(205, "for_i64", {
                "start": result(204),
                "end_exclusive": parameter(202),
                "step": 1,
                "initial": result(203),
                "carried": "bytes",
                "index_symbol": 206,
                "carried_symbol": 207,
                "body": yielding([
                    expression(208, "bytes_at", {
                        "value": parameter(201), "index": block_argument(206),
                    }),
                    expression(209, "const_i64", 0),
                    call(210, 100, [result(208), result(209)]),
                    expression(211, "if", {
                        "condition": result(210),
                        "result": "bytes",
                        "then_body": yielding([], block_argument(207)),
                        "else_body": yielding([
                            expression(212, "const_i64", 1),
                            expression(213, "bytes_slice", {
                                "value": parameter(201),
                                "start": block_argument(206),
                                "length": result(212),
                            }),
                            expression(214, "hole", {"expected": "bytes"}),
                        ], result(214)),
                    }),
                ], result(211)),
            }),
        ],
        result(205),
    )


def entry_function():
    return function(
        300,
        "canonicalize",
        [{"symbol": 301, "name": "input", "ty": "bytes"}],
        nominal(local(20)),
        [
            expression(302, "bytes_len", {"value": parameter(301)}),
            expression(303, "const_i64", 1),
            expression(304, "lt_i64", {"lhs": result(302), "rhs": result(303)}),
            expression(305, "if", {
                "condition": result(304),
                "result": nominal(local(20)),
                "then_body": yielding([
                    expression(306, "construct_variant", {"variant": local(22)}),
                ], result(306)),
                "else_body": yielding([
                    expression(307, "const_i64", 0),
                    expression(308, "bytes_at", {
                        "value": parameter(301), "index": result(307),
                    }),
                    expression(309, "const_i64", 165),
                    call(310, 100, [result(308), result(309)]),
                    expression(311, "if", {
                        "condition": result(310),
                        "result": nominal(local(20)),
                        "then_body": yielding([
                            call(312, 200, [parameter(301), result(302)]),
                            expression(313, "construct_product", {
                                "product": local(10),
                                "fields": [field(11, result(312))],
                            }),
                            expression(314, "construct_variant", {
                                "variant": local(21), "payload": result(313),
                            }),
                        ], result(314)),
                        "else_body": yielding([
                            expression(315, "construct_variant", {"variant": local(22)}),
                        ], result(315)),
                    }),
                ], result(311)),
            }),
        ],
        result(305),
    )


def bounds_probe_function():
    return function(
        400,
        "bounds_probe",
        [{"symbol": 401, "name": "input", "ty": "bytes"}],
        "i64",
        [
            expression(402, "bytes_len", {"value": parameter(401)}),
            expression(403, "bytes_at", {
                "value": parameter(401), "index": result(402),
            }),
        ],
        result(403),
    )


def stream_function():
    return function(
        500,
        "canonicalize_stream",
        [{"symbol": 501, "name": "input", "ty": "bytes"}],
        "bytes",
        [
            expression(502, "bytes_len", {"value": parameter(501)}),
            call(503, 200, [parameter(501), result(502)]),
        ],
        result(503),
    )


def application_operations():
    return [
        {"kind": "create_package", "data": {
            "symbol": symbol(1), "name": "binary-canonicalizer",
        }},
        {"kind": "create_module", "data": {
            "symbol": symbol(2), "package": local(1), "name": "canonical",
        }},
        {"kind": "create_product_type", "data": {
            "symbol": symbol(10), "module": local(2), "name": "CanonicalBytes",
            "fields": [{"symbol": symbol(11), "name": "octets", "ty": "bytes"}],
        }},
        {"kind": "create_sum_type", "data": {
            "symbol": symbol(20), "module": local(2), "name": "CanonicalResult",
            "variants": [
                {"symbol": symbol(21), "name": "canonical", "payload": nominal(local(10))},
                {"symbol": symbol(22), "name": "rejected"},
            ],
        }},
        eq_i64_function(),
        canonicalize_function(),
        entry_function(),
        bounds_probe_function(),
        stream_function(),
        {"kind": "set_entry_function", "data": {
            "package": local(1), "function": local(300),
        }},
    ]


def selected_symbols():
    return [1, 10, 11, 20, 21, 22, 100, 200, 214, 300, 400, 500]


def application_command(arguments, input_value=None, expected_returncode=0):
    encoded = None
    if input_value is not None:
        encoded = json.dumps(input_value, separators=(",", ":")).encode()
    started = time.monotonic_ns()
    completed = subprocess.run(
        [str(CLI), "app", *arguments],
        input=encoded,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        timeout=30,
        check=False,
    )
    application_measurements.append({
        "command": arguments[0],
        "elapsed_nanoseconds": time.monotonic_ns() - started,
        "input_bytes": len(encoded or b""),
        "output_bytes": len(completed.stdout),
        "diagnostic_bytes": len(completed.stderr),
        "exit": completed.returncode,
    })
    if completed.returncode != expected_returncode:
        raise RuntimeError(
            f"application command {arguments} returned {completed.returncode}: "
            f"stdout={completed.stdout!r} stderr={completed.stderr!r}"
        )
    return completed


def release_command(arguments, input_value=None, expected_returncode=0):
    encoded = None
    if input_value is not None:
        encoded = json.dumps(input_value, separators=(",", ":")).encode()
    started = time.monotonic_ns()
    completed = subprocess.run(
        [str(CLI), "release", *arguments],
        input=encoded,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        timeout=30,
        check=False,
    )
    release_measurements.append({
        "command": arguments[0],
        "elapsed_nanoseconds": time.monotonic_ns() - started,
        "input_bytes": len(encoded or b""),
        "output_bytes": len(completed.stdout),
        "diagnostic_bytes": len(completed.stderr),
        "exit": completed.returncode,
    })
    if completed.returncode != expected_returncode:
        raise RuntimeError(
            f"release command {arguments} returned {completed.returncode}: "
            f"stdout={completed.stdout!r} stderr={completed.stderr!r}"
        )
    return completed


def start_stack(count_readiness):
    global session, session_processes
    session = subprocess.Popen(
        [str(CLI), "--state", str(state), "session"],
        stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )
    session_processes += 1
    response = rpc({
        "kind": "describe_schema",
        "data": {"projection": {"kind": "manifest"}},
    }, "schema_manifest" if count_readiness else "restart_session_ready",
        counted=count_readiness)
    return expect(expect(response, "describe_schema"), "manifest")


def stop_stack():
    global session
    if session is None:
        return
    session.stdin.close()
    if session.wait(timeout=5) != 0:
        raise RuntimeError("CLI session close failed")
    session_stderr = session.stderr.read()
    if session_stderr:
        raise RuntimeError(f"CLI session wrote stderr: {session_stderr.decode()}")
    session = None


def rpc(request, purpose, counted=True):
    global request_id
    request_id += 1
    envelope = {"version": 10, "request_id": request_id, "request": request}
    encoded = json.dumps(envelope, separators=(",", ":")).encode()
    started = time.monotonic_ns()
    session.stdin.write(encoded + b"\n")
    session.stdin.flush()
    response_bytes = session.stdout.readline()
    elapsed = time.monotonic_ns() - started
    if not response_bytes:
        raise RuntimeError(f"CLI session ended during {purpose}")
    response = json.loads(response_bytes)
    if response.get("version") != 10 or response.get("request_id") != request_id:
        raise RuntimeError(f"response correlation mismatch for {purpose}")
    measurements.append({
        "purpose": purpose,
        "counted": counted,
        "elapsed_nanoseconds": elapsed,
        "json_request_bytes": len(encoded),
        "json_response_bytes": len(response_bytes),
    })
    if "response" not in response:
        raise RuntimeError(f"boundary response for {purpose}: {response}")
    return response["response"]


def expect(response, kind):
    if response.get("kind") != kind:
        raise RuntimeError(f"expected {kind}, received {response}")
    return response.get("data")


def expect_error(response, code, target=None):
    error = expect(response, "error")
    if error.get("code") != code:
        raise RuntimeError(f"expected {code}, received {error}")
    if target is not None and error.get("target") != target:
        raise RuntimeError(f"expected target {target}, received {error}")
    return error


def apply_request(workspace, revision, mode, operations, returned=None):
    return {
        "kind": "apply_transaction",
        "data": {
            "transaction": {
                "workspace": workspace,
                "base_revision": revision,
                "mode": mode,
                "operations": operations,
            },
            "response": {"return_symbols": [symbol(item) for item in (returned or [])]},
        },
    }


def query_batch(workspace, revision, queries, purpose):
    global query_id
    items = []
    expected_ids = []
    for value in queries:
        query_id += 1
        expected_ids.append(query_id)
        items.append({"id": query_id, "query": value})
    data = expect(rpc({
        "kind": "query_batch",
        "data": {"workspace": workspace, "revision": revision, "queries": items},
    }, purpose), "query_batch_result")
    if [item["id"] for item in data["results"]] != expected_ids:
        raise RuntimeError(f"query correlation mismatch for {purpose}")
    return [expect(item["outcome"], "success") for item in data["results"]]


def query(workspace, revision, value, purpose):
    return query_batch(workspace, revision, [value], purpose)[0]


def run_request(workspace, revision, entry, value, purpose, fuel=10_000_000):
    return rpc({
        "kind": "run",
        "data": {
            "workspace": workspace,
            "revision": revision,
            "entry": entry,
            "arguments": [bytes_value(value)],
            "policy": {"fuel": fuel, "maximum_frames": 2_000},
        },
    }, purpose)


def canonical_value(ids, value):
    return {
        "kind": "sum",
        "data": {
            "ty": ids[20],
            "variant": ids[21],
            "payload": {
                "kind": "product",
                "data": {
                    "ty": ids[10],
                    "fields": [{"field": ids[11], "value": bytes_value(value)}],
                },
            },
        },
    }


def rejected_value(ids):
    return {"kind": "sum", "data": {"ty": ids[20], "variant": ids[22]}}


def run_case(workspace, revision, ids, name, raw, expected, fuel=10_000_000):
    run = expect(run_request(workspace, revision, ids[300], raw, name, fuel), "run")
    if run["value"] != expected:
        raise RuntimeError(f"unexpected {name} result: {run['value']}")
    run_timings.append({
        "purpose": name,
        "compile_nanoseconds": run["compile_nanoseconds"],
        "execute_nanoseconds": run["execute_nanoseconds"],
    })
    return run


def dense_boundary(workspace, ids):
    low = 0
    high = 2_048
    while low + 1 < high:
        middle = (low + high) // 2
        response = run_request(
            workspace, 2, ids[300], bytes([0xA5]) + b"x" * middle,
            f"dense_boundary_{middle}",
        )
        if response.get("kind") == "run":
            low = middle
        else:
            error = expect(response, "error")
            if error["code"] not in (
                "managed_visible_byte_policy_exceeded",
                "retained_byte_policy_exceeded",
                "execution_fuel_exhausted",
            ):
                raise RuntimeError(f"unexpected dense boundary error: {error}")
            high = middle
    run_case(
        workspace, 2, ids, "dense_first_accepted",
        bytes([0xA5]) + b"x" * low, canonical_value(ids, b"x" * low),
    )
    first_rejected = run_request(
        workspace, 2, ids[300], bytes([0xA5]) + b"x" * high,
        "dense_first_rejected",
    )
    rejected = expect(first_rejected, "error")["code"]
    return {"first_accepted_payload_bytes": low, "first_rejected_payload_bytes": high,
            "first_rejected_code": rejected}


def competing_writer_rejects():
    contender = subprocess.run(
        [str(CLI), "--state", str(state), "session"],
        input=b"",
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        timeout=5,
        check=False,
    )
    try:
        boundary = json.loads(contender.stdout)
    except (json.JSONDecodeError, UnicodeDecodeError) as error:
        raise RuntimeError("competing engine returned malformed boundary JSON") from error
    message = boundary.get("error", {}).get("message", "")
    if (contender.returncode != 3
            or boundary.get("error", {}).get("kind") != "transport"
            or "AuthorityBusy" not in message
            or b"AuthorityBusy" not in contender.stderr):
        raise RuntimeError(f"competing engine authority rejection disagrees: {boundary}")


def workflow():
    global state
    with tempfile.TemporaryDirectory(prefix="lkjscript-binary-canonicalizer-") as directory:
        root = pathlib.Path(directory)
        state = root / "state"
        state.mkdir()
        os.chmod(state, 0o700)
        manifest = start_stack(True)
        competing_writer_rejects()
        roots = [
            "create_workspace", "apply_transaction", "query_workspace_summary",
            "query_node", "query_blockers", "query_body", "query_incoming_uses",
            "query_repair_context", "query_semantic_diff", "query_nominal_type",
            "run",
        ]
        task = expect(expect(rpc({
            "kind": "describe_schema",
            "data": {"projection": {"kind": "roots", "data": {"roots": roots}}},
        }, "schema_task_roots"), "describe_schema"), "roots")
        unchanged = expect(expect(rpc({
            "kind": "describe_schema",
            "data": {"projection": {"kind": "full"}, "known_digest": manifest["digest"]},
        }, "schema_unchanged"), "describe_schema"), "unchanged")
        if task["digest"] != manifest["digest"] or unchanged["digest"] != manifest["digest"]:
            raise RuntimeError("schema digests disagree")
        if "bytes_concat" not in json.dumps(task, separators=(",", ":")):
            raise RuntimeError("targeted schema roots omit bytes_concat")

        workspace = expect(rpc({"kind": "create_workspace"}, "create_workspace"),
                           "workspace_created")["workspace"]
        creation = expect(rpc(apply_request(
            workspace, 0, "commit", application_operations(), selected_symbols(),
        ), "create_incomplete_application"), "transaction_receipt")
        ids = {
            int(name.removeprefix("draft_")): node
            for name, node in creation["returned_bindings"]
        }
        if creation["revision"] != 1 or creation["complete_after"] or set(ids) != set(selected_symbols()):
            raise RuntimeError("incomplete creation receipt is malformed")

        blockers = expect(query(workspace, 1, {
            "kind": "blockers", "data": {"page": {"limit": 16}},
        }, "incomplete_blockers"), "blockers")
        if not any(item.get("target") == ids[214] and item.get("expected_type") == "bytes"
                   for item in blockers["items"]):
            raise RuntimeError("reachable bytes placeholder is absent")
        context = expect(query(workspace, 1, {
            "kind": "repair_context",
            "data": {
                "target": {"kind": "hole", "data": ids[214]},
                "budget": {
                    "body_before": 8, "body_after": 8, "visible_values": 32,
                    "incoming_uses": 8, "include_incompatible": True,
                },
            },
        }, "concat_repair_context"), "repair_context")
        if not any(item["code"] == "bytes_concat" for item in context["legal_constructors"]):
            raise RuntimeError("repair context omits bytes_concat")
        carried = next((item for item in context["visible_values"]["items"]
                        if item["compatible"] and item["value"]["kind"] == "block_argument"), None)
        sliced = next((item for item in context["visible_values"]["items"]
                       if item.get("producer_code") == "bytes_slice"), None)
        if carried is None or sliced is None:
            raise RuntimeError("repair context omits carried bytes or one-octet slice")
        before_location = (context["owner_block"], context["owner_function"], context["ordinal"])
        before_uses = context["incoming_uses"]["items"]

        invalid = expect_error(rpc(apply_request(workspace, 1, "commit", [{
            "kind": "refine_hole",
            "data": {"hole": existing(ids[214]), "replacement": {
                "kind": "const_i64", "data": 0,
            }},
        }]), "invalid_repair"), "type_mismatch", ids[214])
        if invalid.get("expected_type") != "bytes" or invalid.get("actual_type") != "i64":
            raise RuntimeError("invalid repair omits exact types")
        summary = expect(query(workspace, 1, {"kind": "workspace_summary"},
                               "summary_after_invalid"), "workspace_summary")
        if summary["revision"] != 1 or summary["complete"]:
            raise RuntimeError("invalid repair published")
        expect_error(run_request(workspace, 1, ids[300], bytes([0xA5]),
                                 "run_incomplete"), "compile_incomplete")

        repair_operation = [{
            "kind": "refine_hole",
            "data": {
                "hole": existing(ids[214]),
                "replacement": {
                    "kind": "bytes_concat",
                    "data": {
                        "lhs": persisted_value(carried["value"]),
                        "rhs": persisted_value(sliced["value"]),
                    },
                },
            },
        }]
        predicted = expect(rpc(apply_request(
            workspace, 1, "validate_only", repair_operation,
        ), "validate_concat_repair"), "transaction_receipt")
        repaired = expect(rpc(apply_request(
            workspace, 1, "commit", repair_operation,
        ), "commit_concat_repair"), "transaction_receipt")
        if (repaired["revision"] != 2 or repaired["created_count"] != 0
                or not repaired["complete_after"] or predicted["hash"] != repaired["hash"]):
            raise RuntimeError("validate-only and committed refinement disagree")

        post = query_batch(workspace, 2, [
            {"kind": "node", "data": {"node": ids[214], "expand": True}},
            {"kind": "body", "data": {"block": before_location[0], "page": {"limit": 32}}},
            {"kind": "incoming_uses", "data": {
                "value": {"kind": "operation_result", "data": {
                    "operation": ids[214], "output": 0,
                }},
                "page": {"limit": 16},
            }},
            {"kind": "semantic_diff", "data": {"from": 1, "page": {"limit": 16}}},
        ], "repair_identity_and_diff")
        node = expect(post[0], "node")
        body = expect(post[1], "body")
        uses = expect(post[2], "incoming_uses")
        repair_diff = expect(post[3], "semantic_diff")
        body_item = next((item for item in body["items"] if item["operation"] == ids[214]), None)
        if (node["summary"]["owner"] != before_location[0] or body_item is None
                or body_item["ordinal"] != before_location[2] or body_item["code"] != "bytes_concat"
                or uses["items"] != before_uses
                or not any(item["node"] == ids[214]
                           and item["kind"]["kind"] == "operation_refined"
                           for item in repair_diff["page"]["items"])):
            raise RuntimeError("identity-preserving concat refinement facts disagree")

        vectors = [
            ("empty_payload", bytes([0xA5]), b""),
            ("all_padding", bytes([0xA5, 0, 0, 0]), b""),
            ("no_padding", bytes([0xA5, 1, 2, 3]), bytes([1, 2, 3])),
            ("alternating", bytes([0xA5, 0, 1, 0, 2, 0, 3]), bytes([1, 2, 3])),
            ("long_sparse", bytes([0xA5]) + bytes(4_096) + b"abc", b"abc"),
            ("long_dense", bytes([0xA5]) + b"x" * 1_024, b"x" * 1_024),
        ]
        for name, raw, expected in vectors:
            run_case(workspace, 2, ids, name, raw, canonical_value(ids, expected))
        run_case(workspace, 2, ids, "empty_input", b"", rejected_value(ids), fuel=1_000)
        run_case(workspace, 2, ids, "wrong_marker", b"X" + b"x" * 1_024,
                 rejected_value(ids), fuel=1_000)
        expect_error(run_request(workspace, 2, ids[300], bytes([0xA5]) + b"x" * 1_024,
                                 "dense_low_fuel", fuel=1_000), "execution_fuel_exhausted")
        expect_error(run_request(workspace, 2, ids[400], bytes([0xA5, 1]),
                                 "bounds_probe"), "byte_index_out_of_bounds")
        boundary = dense_boundary(workspace, ids)

        run_case(workspace, 2, ids, "after_runtime_traps", bytes([0xA5, 1, 0, 2]),
                 canonical_value(ids, bytes([1, 2])))

        rename = expect(rpc(apply_request(workspace, 2, "commit", [{
            "kind": "rename_node",
            "data": {"node": existing(ids[11]), "name": "canonical_octets"},
        }]), "rename_output_field"), "transaction_receipt")
        if rename["revision"] != 3 or rename["created_count"] != 0:
            raise RuntimeError("rename receipt is malformed")
        rename_diff = expect(query(workspace, 3, {
            "kind": "semantic_diff", "data": {"from": 2, "page": {"limit": 8}},
        }, "rename_diff"), "semantic_diff")
        if (rename_diff["change_count"] != 1
                or rename_diff["page"]["items"][0]["node"] != ids[11]):
            raise RuntimeError("rename semantic diff is malformed")
        run_case(workspace, 3, ids, "renamed_behavior", bytes([0xA5, 3, 0, 4]),
                 canonical_value(ids, bytes([3, 4])))

        stop_stack()
        start_stack(False)
        expect_error(run_request(workspace, 1, ids[300], bytes([0xA5]),
                                 "restart_old_incomplete"), "compile_incomplete")
        run_case(workspace, 2, ids, "restart_repaired", bytes([0xA5, 5, 0, 6]),
                 canonical_value(ids, bytes([5, 6])))
        run_case(workspace, 3, ids, "restart_current", bytes([0xA5, 7, 0, 8]),
                 canonical_value(ids, bytes([7, 8])))
        stop_stack()

        release_request = {
            "version": 1,
            "workspace": workspace,
            "revision": 3,
            "root": ids[1],
            "coordinate": "examples/binary-canonicalizer",
            "user_version": "1.0.0",
            "exports": [
                {"name": "canonical_bytes", "target": ids[10]},
                {"name": "canonical_result", "target": ids[20]},
                {"name": "canonicalize", "target": ids[300]},
                {"name": "bounds_probe", "target": ids[400]},
                {"name": "canonicalize_stream", "target": ids[500]},
            ],
            "dependencies": [],
            "imports": [],
            "tests": [
                {
                    "name": "empty",
                    "target": ids[500],
                    "arguments": [bytes_value(b"")],
                    "expected": {"kind": "value", "data": bytes_value(b"")},
                    "policy": {"fuel": 1_000, "maximum_frames": 2_000},
                },
                {
                    "name": "sparse",
                    "target": ids[500],
                    "arguments": [bytes_value(bytes([0xA5, 0, 1, 0, 2]))],
                    "expected": {"kind": "value", "data": bytes_value(bytes([1, 2]))},
                    "policy": {"fuel": 100_000, "maximum_frames": 2_000},
                },
                {
                    "name": "bounds_trap",
                    "target": ids[400],
                    "arguments": [bytes_value(bytes([0xA5, 1]))],
                    "expected": {
                        "kind": "trap",
                        "data": {"code": "byte_index_out_of_bounds"},
                    },
                    "policy": {"fuel": 1_000, "maximum_frames": 2_000},
                },
            ],
        }
        release_path = root / "binary-canonicalizer.lkjr"
        release_preflight = json.loads(release_command([
            "build", "--state", str(state), "--validate-only",
        ], release_request).stdout)
        release_build = json.loads(release_command([
            "build", "--state", str(state), "--output", str(release_path),
        ], release_request).stdout)
        repeated_release_path = root / "binary-canonicalizer-repeated.lkjr"
        repeated_release = json.loads(release_command([
            "build", "--state", str(state), "--output", str(repeated_release_path),
        ], release_request).stdout)
        if (release_preflight["published"]
                or not release_build["published"]
                or release_build["tests"]["passed"] != 3
                or release_path.read_bytes() != repeated_release_path.read_bytes()
                or repeated_release["inspection"]["release"]
                != release_build["inspection"]["release"]):
            raise RuntimeError("reusable release validate/build determinism failed")
        release_id = release_build["inspection"]["release"]
        release_exports = {
            item["name"]: item["target"]
            for item in release_build["inspection"]["exports"]
        }
        stream_target = {"release": release_id, "item": release_exports["canonicalize_stream"]}
        bounds_target = {"release": release_id, "item": release_exports["bounds_probe"]}

        application_request = {
            "version": 4,
            "root_release": release_id,
            "entry": stream_target,
            "profile": {"kind": "bytes_stream"},
            "policy": {"fuel": 10_000_000, "maximum_frames": 2_000},
            "tests": [
                {
                    "name": "empty",
                    "target": stream_target,
                    "arguments": [bytes_value(b"")],
                    "expected": {"kind": "value", "data": bytes_value(b"")},
                    "policy": {"fuel": 1_000, "maximum_frames": 2_000},
                },
                {
                    "name": "sparse",
                    "target": stream_target,
                    "arguments": [bytes_value(bytes([0xA5, 0, 1, 0, 2]))],
                    "expected": {"kind": "value", "data": bytes_value(bytes([1, 2]))},
                    "policy": {"fuel": 100_000, "maximum_frames": 2_000},
                },
                {
                    "name": "bounds_trap",
                    "target": bounds_target,
                    "arguments": [bytes_value(bytes([0xA5, 1]))],
                    "expected": {
                        "kind": "trap",
                        "data": {"code": "byte_index_out_of_bounds"},
                    },
                    "policy": {"fuel": 1_000, "maximum_frames": 2_000},
                },
            ],
        }
        application_path = root / "binary-canonicalizer.lkja"
        preflight = json.loads(application_command([
            "build", "--release", str(release_path), "--validate-only",
        ], application_request).stdout)
        if preflight["published"] or preflight["tests"]["passed"] != 6:
            raise RuntimeError(f"application validate-only receipt is malformed: {preflight}")
        build = json.loads(application_command([
            "build", "--release", str(release_path), "--output", str(application_path),
        ], application_request).stdout)
        if (not build["published"] or build["tests"]["passed"] != 6
                or build["inspection"]["profile"] != {"kind": "bytes_stream"}
                or build["inspection"]["digest"] != preflight["inspection"]["digest"]):
            raise RuntimeError(f"application build receipt is malformed: {build}")
        repeated_path = root / "binary-canonicalizer-repeated.lkja"
        repeated = json.loads(application_command([
            "build", "--release", str(release_path), "--output", str(repeated_path),
        ], application_request).stdout)
        if (application_path.read_bytes() != repeated_path.read_bytes()
                or repeated["inspection"]["digest"] != build["inspection"]["digest"]):
            raise RuntimeError("equal application builds are not byte-identical")

        failing_request = dict(application_request)
        failing_request["tests"] = [dict(item) for item in application_request["tests"]]
        failing_request["tests"][0] = dict(failing_request["tests"][0])
        failing_request["tests"][0]["expected"] = {
            "kind": "value", "data": bytes_value(b"wrong"),
        }
        blocked_path = root / "blocked.lkja"
        blocked = application_command([
            "build", "--release", str(release_path), "--output", str(blocked_path),
        ], failing_request, expected_returncode=7)
        blocked_error = json.loads(blocked.stdout)
        if (blocked_error.get("contract_version") != 4
                or blocked_error.get("error", {}).get("code") != "application_test_failed"
                or blocked_path.exists()):
            raise RuntimeError(f"failing release test did not block publication: {blocked_error}")

        artifact = sorted(state.rglob("*.lkjscript"))[-1]
        corrupted = bytearray(artifact.read_bytes())
        corrupted[len(corrupted) // 2] ^= 0x01
        artifact.write_bytes(corrupted)
        corrupt_probe = json.dumps({
            "version": 10,
            "request_id": request_id + 1,
            "request": {"kind": "describe_schema", "data": {
                "projection": {"kind": "manifest"},
            }},
        }, separators=(",", ":")).encode()
        failed = subprocess.run(
            [str(CLI), "--state", str(state), "rpc"],
            input=corrupt_probe,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            timeout=5,
            check=False,
        )
        if failed.returncode == 0 or not failed.stderr:
            raise RuntimeError("corrupt authority did not reject on restart")

        shutil.rmtree(state)
        release_validation = json.loads(release_command([
            "validate", "--artifact", str(release_path),
        ]).stdout)
        release_tests = json.loads(release_command([
            "test", "--artifact", str(release_path),
        ]).stdout)
        validation = json.loads(application_command([
            "validate", "--artifact", str(application_path),
        ]).stdout)
        inspection = json.loads(application_command([
            "inspect", "--artifact", str(application_path),
        ]).stdout)
        artifact_tests = json.loads(application_command([
            "test", "--artifact", str(application_path),
        ]).stdout)
        typed = json.loads(application_command([
            "run", "--artifact", str(application_path),
        ], {"version": 4, "arguments": [bytes_value(bytes([0xA5, 3, 0, 4]))]}).stdout)
        stream_input = bytes([0xA5, 5, 0, 6])
        stream_started = time.monotonic_ns()
        streamed = subprocess.run(
            [str(CLI), "app", "stream", "--artifact", str(application_path)],
            input=stream_input,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            timeout=30,
            check=False,
        )
        application_measurements.append({
            "command": "stream",
            "elapsed_nanoseconds": time.monotonic_ns() - stream_started,
            "input_bytes": len(stream_input),
            "output_bytes": len(streamed.stdout),
            "diagnostic_bytes": len(streamed.stderr),
            "exit": streamed.returncode,
        })
        if (release_validation["release"] != release_id
                or release_tests["report"]["passed"] != 3
                or validation["digest"] != build["inspection"]["digest"]
                or inspection != validation
                or artifact_tests["report"]["passed"] != 6
                or typed["result"]["value"] != bytes_value(bytes([3, 4]))
                or streamed.returncode != 0 or streamed.stdout != bytes([5, 6])
                or streamed.stderr):
            raise RuntimeError("offline application validate, inspect, test, run, or stream failed")

        corrupt_application_path = root / "corrupt.lkja"
        corrupt_application = bytearray(application_path.read_bytes())
        corrupt_application[len(corrupt_application) // 2] ^= 1
        corrupt_application_path.write_bytes(corrupt_application)
        corrupt_application_result = application_command([
            "validate", "--artifact", str(corrupt_application_path),
        ], expected_returncode=5)
        if json.loads(corrupt_application_result.stdout).get("error", {}).get("code") != "artifact_corrupt":
            raise RuntimeError("corrupt application bundle did not reject")

        counted = [item for item in measurements if item["counted"]]
        restart_timings = [
            item for item in run_timings if item["purpose"].startswith("restart_")
        ]
        report = {
            "application": "binary-canonicalizer",
            "protocol_version": 10,
            "schema_digest": manifest["digest"],
            "task_schema_json_bytes": len(json.dumps(task, separators=(",", ":")).encode()),
            "calls": len(counted),
            "session_processes": session_processes,
            "engine_opens": session_processes + 3,
            "connections": 0,
            "request_bytes": sum(item["json_request_bytes"] for item in counted),
            "response_bytes": sum(item["json_response_bytes"] for item in counted),
            "elapsed_nanoseconds": sum(item["elapsed_nanoseconds"] for item in counted),
            "timings": {
                "first_compile_nanoseconds": run_timings[0]["compile_nanoseconds"],
                "first_execute_nanoseconds": run_timings[0]["execute_nanoseconds"],
                "measured_compile_nanoseconds": sum(
                    item["compile_nanoseconds"] for item in run_timings
                ),
                "measured_execute_nanoseconds": sum(
                    item["execute_nanoseconds"] for item in run_timings
                ),
                "restart_compile_nanoseconds": sum(
                    item["compile_nanoseconds"] for item in restart_timings
                ),
                "restart_execute_nanoseconds": sum(
                    item["execute_nanoseconds"] for item in restart_timings
                ),
            },
            "dense_boundary": boundary,
            "revisions": {"incomplete": 1, "repaired": 2, "renamed": 3},
            "corrupt_restart_rejected": True,
            "reusable_release": {
                "bytes": len(release_path.read_bytes()),
                "release": release_id,
                "content_digest": release_build["inspection"]["content_digest"],
                "exports": len(release_build["inspection"]["exports"]),
                "private_durable_items": release_build["inspection"]["private_durable_items"],
                "tests": release_build["tests"]["passed"],
                "deterministic_rebuild": True,
                "workspace_independent_validation_and_test": True,
            },
            "application_artifact": {
                "bytes": len(application_path.read_bytes()),
                "digest": build["inspection"]["digest"],
                "graph_digest": build["inspection"]["graph_digest"],
                "nodes": build["inspection"]["flattened_semantic_items"],
                "releases": len(build["inspection"]["releases"]),
                "tests": build["tests"]["passed"],
                "deterministic_rebuild": True,
                "failing_application_test_blocked": True,
                "source_workspace_removed": True,
                "offline_validate_inspect_test_typed_run_stream": True,
                "corrupt_rejected": True,
            },
            "release_workflow": {
                "processes": len(release_measurements),
                "input_bytes": sum(item["input_bytes"] for item in release_measurements),
                "output_bytes": sum(item["output_bytes"] for item in release_measurements),
                "diagnostic_bytes": sum(
                    item["diagnostic_bytes"] for item in release_measurements
                ),
                "elapsed_nanoseconds": sum(
                    item["elapsed_nanoseconds"] for item in release_measurements
                ),
                "failed_processes": sum(
                    item["exit"] != 0 for item in release_measurements
                ),
            },
            "application_workflow": {
                "processes": len(application_measurements),
                "input_bytes": sum(item["input_bytes"] for item in application_measurements),
                "output_bytes": sum(item["output_bytes"] for item in application_measurements),
                "diagnostic_bytes": sum(
                    item["diagnostic_bytes"] for item in application_measurements
                ),
                "elapsed_nanoseconds": sum(
                    item["elapsed_nanoseconds"] for item in application_measurements
                ),
                "failed_processes": sum(
                    item["exit"] != 0 for item in application_measurements
                ),
                "validate_only_equal": True,
            },
        }
        print(json.dumps(report, sort_keys=True))


def main():
    try:
        workflow()
    finally:
        if session is not None and session.poll() is None:
            session.kill()


if __name__ == "__main__":
    main()
