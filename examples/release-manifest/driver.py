#!/usr/bin/env python3
"""Build, repair, restart, and run a bounded binary release-manifest classifier."""

import base64
import json
import os
import pathlib
import subprocess
import sys
import tempfile
import time


ROOT = pathlib.Path(__file__).resolve().parents[2]
CLI = pathlib.Path(sys.argv[1]).resolve() if len(sys.argv) > 1 else ROOT / "target/release/lkjscript"
METRICS_PATH = pathlib.Path(sys.argv[2]).resolve() if len(sys.argv) > 2 else None
request_id = 0
query_id = 0
state = None
measurements = []


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


def existing_result(node):
    return {
        "kind": "operation_result",
        "data": {"operation": existing(node), "output": 0},
    }


def parameter(number_or_node, persisted=False):
    target = existing(number_or_node) if persisted else local(number_or_node)
    return {"kind": "function_parameter", "data": target}


def block_argument(number):
    return {"kind": "block_argument", "data": local(number)}


def expression(number, kind, data=None):
    operation = {"kind": kind}
    if data is not None:
        if kind == "for_i64":
            data = dict(data)
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


def reject_body(reason_variant, first):
    return yielding([
        expression(first, "construct_variant", {"variant": local(reason_variant)}),
        expression(first + 1, "construct_variant", {
            "variant": local(32), "payload": result(first),
        }),
    ], result(first + 1))


def accept_body(class_variant, first):
    return yielding([
        expression(first, "construct_variant", {"variant": local(class_variant)}),
        expression(first + 1, "construct_variant", {
            "variant": local(31), "payload": result(first),
        }),
    ], result(first + 1))


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


def payload_valid_function():
    return function(
        200,
        "payload_valid",
        [{"symbol": 201, "name": "manifest", "ty": "bytes"}],
        "bool",
        [
            expression(202, "const_i64", 8),
            expression(203, "const_i64", 24),
            expression(204, "const_bool", True),
            expression(205, "for_i64", {
                "start": result(202),
                "end_exclusive": result(203),
                "step": 1,
                "initial": result(204),
                "carried": "bool",
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
                        "result": "bool",
                        "then_body": yielding([
                            expression(212, "const_bool", False),
                        ], result(212)),
                        "else_body": yielding([], block_argument(207)),
                    }),
                ], result(211)),
            }),
            expression(213, "const_i64", 31),
            expression(214, "bytes_at", {"value": parameter(201), "index": result(213)}),
            expression(215, "const_i64", 165),
            call(216, 100, [result(214), result(215)]),
            expression(217, "if", {
                "condition": result(205),
                "result": "bool",
                "then_body": yielding([], result(216)),
                "else_body": yielding([
                    expression(218, "const_bool", False),
                ], result(218)),
            }),
        ],
        result(217),
    )


def check_payload_function():
    return function(
        300,
        "check_payload",
        [
            {"symbol": 301, "name": "manifest", "ty": "bytes"},
            {"symbol": 302, "name": "channel", "ty": "i64"},
        ],
        nominal(local(30)),
        [
            expression(310, "hole", {"expected": "bool"}),
            expression(311, "if", {
                "condition": result(310),
                "result": nominal(local(30)),
                "then_body": yielding([
                    expression(312, "const_i64", 0),
                    call(313, 100, [parameter(302), result(312)]),
                    expression(314, "if", {
                        "condition": result(313),
                        "result": nominal(local(30)),
                        "then_body": accept_body(11, 315),
                        "else_body": accept_body(12, 317),
                    }),
                ], result(314)),
                "else_body": reject_body(27, 319),
            }),
        ],
        result(311),
    )


def check_flags_function():
    return function(
        400,
        "check_flags",
        [
            {"symbol": 401, "name": "manifest", "ty": "bytes"},
            {"symbol": 402, "name": "channel", "ty": "i64"},
        ],
        nominal(local(30)),
        [
            expression(403, "const_i64", 7),
            expression(404, "bytes_at", {"value": parameter(401), "index": result(403)}),
            expression(405, "const_i64", 0),
            call(406, 100, [result(404), result(405)]),
            expression(407, "if", {
                "condition": result(406),
                "result": nominal(local(30)),
                "then_body": yielding([
                    call(408, 300, [parameter(401), parameter(402)]),
                ], result(408)),
                "else_body": yielding([
                    expression(409, "const_i64", 1),
                    call(410, 100, [result(404), result(409)]),
                    expression(411, "if", {
                        "condition": result(410),
                        "result": nominal(local(30)),
                        "then_body": yielding([
                            call(412, 300, [parameter(401), parameter(402)]),
                        ], result(412)),
                        "else_body": reject_body(26, 413),
                    }),
                ], result(411)),
            }),
        ],
        result(407),
    )


def check_channel_function():
    return function(
        500,
        "check_channel",
        [{"symbol": 501, "name": "manifest", "ty": "bytes"}],
        nominal(local(30)),
        [
            expression(502, "const_i64", 5),
            expression(503, "bytes_at", {"value": parameter(501), "index": result(502)}),
            expression(504, "const_i64", 0),
            call(505, 100, [result(503), result(504)]),
            expression(506, "if", {
                "condition": result(505),
                "result": nominal(local(30)),
                "then_body": yielding([
                    call(507, 400, [parameter(501), result(503)]),
                ], result(507)),
                "else_body": yielding([
                    expression(508, "const_i64", 1),
                    call(509, 100, [result(503), result(508)]),
                    expression(510, "if", {
                        "condition": result(509),
                        "result": nominal(local(30)),
                        "then_body": yielding([
                            call(511, 400, [parameter(501), result(503)]),
                        ], result(511)),
                        "else_body": reject_body(25, 512),
                    }),
                ], result(510)),
            }),
        ],
        result(506),
    )


def check_target_function():
    return function(
        600,
        "check_target",
        [{"symbol": 601, "name": "manifest", "ty": "bytes"}],
        nominal(local(30)),
        [
            expression(602, "const_i64", 6),
            expression(603, "bytes_at", {"value": parameter(601), "index": result(602)}),
            expression(604, "const_i64", 1),
            call(605, 100, [result(603), result(604)]),
            expression(606, "if", {
                "condition": result(605),
                "result": nominal(local(30)),
                "then_body": yielding([
                    call(607, 500, [parameter(601)]),
                ], result(607)),
                "else_body": yielding([
                    expression(608, "const_i64", 2),
                    call(609, 100, [result(603), result(608)]),
                    expression(610, "if", {
                        "condition": result(609),
                        "result": nominal(local(30)),
                        "then_body": yielding([
                            call(611, 500, [parameter(601)]),
                        ], result(611)),
                        "else_body": reject_body(24, 612),
                    }),
                ], result(610)),
            }),
        ],
        result(606),
    )


def check_version_function():
    return function(
        700,
        "check_version",
        [{"symbol": 701, "name": "manifest", "ty": "bytes"}],
        nominal(local(30)),
        [
            expression(702, "const_i64", 4),
            expression(703, "bytes_at", {"value": parameter(701), "index": result(702)}),
            expression(704, "const_i64", 1),
            call(705, 100, [result(703), result(704)]),
            expression(706, "if", {
                "condition": result(705),
                "result": nominal(local(30)),
                "then_body": yielding([
                    call(707, 600, [parameter(701)]),
                ], result(707)),
                "else_body": reject_body(23, 708),
            }),
        ],
        result(706),
    )


def check_magic_function():
    return function(
        800,
        "check_magic",
        [{"symbol": 801, "name": "manifest", "ty": "bytes"}],
        nominal(local(30)),
        [
            expression(802, "const_i64", 0),
            expression(803, "const_i64", 4),
            expression(804, "bytes_slice", {
                "value": parameter(801), "start": result(802), "length": result(803),
            }),
            expression(805, "const_bytes", encode_bytes(b"LKJM")),
            expression(806, "bytes_equal", {"lhs": result(804), "rhs": result(805)}),
            expression(807, "if", {
                "condition": result(806),
                "result": nominal(local(30)),
                "then_body": yielding([
                    call(808, 700, [parameter(801)]),
                ], result(808)),
                "else_body": reject_body(22, 809),
            }),
        ],
        result(807),
    )


def classify_function():
    return function(
        900,
        "classify",
        [{"symbol": 901, "name": "manifest", "ty": "bytes"}],
        nominal(local(30)),
        [
            expression(902, "bytes_len", {"value": parameter(901)}),
            expression(903, "const_i64", 32),
            call(904, 100, [result(902), result(903)]),
            expression(905, "if", {
                "condition": result(904),
                "result": nominal(local(30)),
                "then_body": yielding([
                    call(906, 800, [parameter(901)]),
                ], result(906)),
                "else_body": reject_body(21, 907),
            }),
        ],
        result(905),
    )


def bounds_probe_function():
    return function(
        950,
        "bounds_probe",
        [{"symbol": 951, "name": "manifest", "ty": "bytes"}],
        "i64",
        [
            expression(952, "bytes_len", {"value": parameter(951)}),
            expression(953, "bytes_at", {"value": parameter(951), "index": result(952)}),
        ],
        result(953),
    )


def application_operations():
    return [
        {"kind": "create_package", "data": {
            "symbol": symbol(1), "name": "release-manifest",
        }},
        {"kind": "create_module", "data": {
            "symbol": symbol(2), "package": local(1), "name": "root",
        }},
        eq_i64_function(),
        payload_valid_function(),
        check_payload_function(),
        check_flags_function(),
        check_channel_function(),
        check_target_function(),
        check_version_function(),
        check_magic_function(),
        classify_function(),
        bounds_probe_function(),
        {"kind": "create_sum_type", "data": {
            "symbol": symbol(10), "module": local(2), "name": "ReleaseClass",
            "variants": [
                {"symbol": symbol(11), "name": "stable"},
                {"symbol": symbol(12), "name": "preview"},
            ],
        }},
        {"kind": "create_sum_type", "data": {
            "symbol": symbol(20), "module": local(2), "name": "RejectReason",
            "variants": [
                {"symbol": symbol(21), "name": "wrong_length"},
                {"symbol": symbol(22), "name": "wrong_magic"},
                {"symbol": symbol(23), "name": "unsupported_format"},
                {"symbol": symbol(24), "name": "unsupported_target"},
                {"symbol": symbol(25), "name": "forbidden_channel"},
                {"symbol": symbol(26), "name": "invalid_flags"},
                {"symbol": symbol(27), "name": "invalid_payload"},
            ],
        }},
        {"kind": "create_sum_type", "data": {
            "symbol": symbol(30), "module": local(2), "name": "Decision",
            "variants": [
                {"symbol": symbol(31), "name": "accept", "payload": nominal(local(10))},
                {"symbol": symbol(32), "name": "reject", "payload": nominal(local(20))},
            ],
        }},
        {"kind": "set_entry_function", "data": {
            "package": local(1), "function": local(900),
        }},
    ]


def selected_symbols():
    return [
        10, 11, 12,
        20, 21, 22, 23, 24, 25, 26, 27,
        30, 31, 32,
        100, 200, 300, 301, 310, 400, 500, 600, 700, 800, 900, 950,
    ]


def encode_bytes(value):
    return base64.urlsafe_b64encode(bytes(value)).rstrip(b"=").decode("ascii")


def bytes_value(value):
    return {"kind": "bytes", "data": encode_bytes(value)}


def sum_value(type_id, variant_id, payload=None):
    data = {"ty": type_id, "variant": variant_id}
    if payload is not None:
        data["payload"] = payload
    return {"kind": "sum", "data": data}


def accepted(ids, class_symbol):
    return sum_value(ids[30], ids[31], sum_value(ids[10], ids[class_symbol]))


def rejected(ids, reason_symbol):
    return sum_value(ids[30], ids[32], sum_value(ids[20], ids[reason_symbol]))


def manifest(channel=0, target=1, flags=0):
    value = bytearray(32)
    value[0:4] = b"LKJM"
    value[4] = 1
    value[5] = channel
    value[6] = target
    value[7] = flags
    value[8:24] = bytes(range(1, 17))
    value[31] = 0xA5
    return value


def rpc(request, purpose, counted=True):
    global request_id
    request_id += 1
    envelope = {"version": 11, "request_id": request_id, "request": request}
    encoded = json.dumps(envelope, separators=(",", ":")).encode()
    started = time.monotonic_ns()
    completed = subprocess.run(
        [str(CLI), "--state", str(state), "rpc"],
        input=encoded,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        check=False,
    )
    elapsed = time.monotonic_ns() - started
    if completed.returncode != 0:
        raise RuntimeError(
            f"CLI failed for {purpose} ({completed.returncode}): {completed.stderr.decode()}"
        )
    if completed.stderr:
        raise RuntimeError(f"CLI wrote stderr for {purpose}: {completed.stderr.decode()}")
    response = json.loads(completed.stdout)
    if response.get("version") != 11 or response.get("request_id") != request_id:
        raise RuntimeError(f"response correlation mismatch for {purpose}")
    measurements.append({
        "purpose": purpose,
        "counted": counted,
        "elapsed_nanoseconds": elapsed,
        "json_request_bytes": len(encoded),
        "json_response_bytes": len(completed.stdout),
    })
    return response["response"]


def expect(response, kind):
    if response.get("kind") != kind:
        raise RuntimeError(f"expected {kind}, received {response}")
    return response.get("data")


def expect_error(response, code, target=None):
    error = expect(response, "error")
    if error.get("code") != code:
        raise RuntimeError(f"expected error {code}, received {error}")
    if target is not None and error.get("target") != target:
        raise RuntimeError(f"expected error target {target}, received {error}")
    return error


def apply_request(workspace, revision, mode, operations, return_symbols=None):
    return {
        "kind": "apply_transaction",
        "data": {
            "transaction": {
                "workspace": workspace,
                "base_revision": revision,
                "mode": mode,
                "operations": operations,
            },
            "response": {
                "return_symbols": [
                    symbol(item) if isinstance(item, int) else item
                    for item in (return_symbols or [])
                ],
            },
        },
    }


def query_batch(workspace, revision, queries, purpose):
    global query_id
    items = []
    expected = []
    for value in queries:
        query_id += 1
        expected.append(query_id)
        items.append({"id": query_id, "query": value})
    data = expect(rpc({
        "kind": "query_batch",
        "data": {"workspace": workspace, "revision": revision, "queries": items},
    }, purpose), "query_batch_result")
    if [item["id"] for item in data["results"]] != expected:
        raise RuntimeError(f"query correlation mismatch for {purpose}")
    return [expect(item["outcome"], "success") for item in data["results"]]


def query(workspace, revision, value, purpose):
    return query_batch(workspace, revision, [value], purpose)[0]


def run_request(workspace, revision, entry, value, purpose, fuel=100_000):
    return rpc({
        "kind": "run",
        "data": {
            "workspace": workspace,
            "revision": revision,
            "entry": entry,
            "arguments": [bytes_value(value)],
            "policy": {"fuel": fuel, "maximum_frames": 1_000},
        },
    }, purpose)


def run_case(workspace, revision, ids, name, value, expected, fuel=100_000):
    data = expect(run_request(
        workspace, revision, ids[900], value, name, fuel=fuel,
    ), "run")
    if data["value"] != expected:
        raise RuntimeError(f"unexpected result for {name}: {data['value']}")
    return data


def allocation_probe(workspace, purpose):
    return expect(rpc(apply_request(workspace, 1, "validate_only", [{
        "kind": "create_package",
        "data": {"symbol": "allocation_probe", "name": "allocation-probe"},
    }], ["allocation_probe"]), purpose), "transaction_receipt")


def nominal_query(declaration):
    return {
        "kind": "nominal_type",
        "data": {"declaration": declaration, "page": {"limit": 32}},
    }


def member_name(context, member):
    for item in context["members"]["items"]:
        data = item.get("data", {})
        if data.get("variant") == member or data.get("field") == member:
            return data["name"]
    raise RuntimeError(f"member {member} absent")


def workflow():
    global state
    with tempfile.TemporaryDirectory(prefix="lkjscript-release-manifest-") as directory:
        state = pathlib.Path(directory)
        os.chmod(state, 0o700)

        manifest_schema = expect(expect(rpc({
            "kind": "describe_schema", "data": {"projection": {"kind": "manifest"}},
        }, "schema_manifest"), "describe_schema"), "manifest")
        roots = [
            "create_workspace", "apply_transaction", "query_workspace_summary",
            "query_node", "query_blockers", "query_body", "query_incoming_uses",
            "query_repair_context", "query_semantic_diff", "query_nominal_type",
            "run",
        ]
        task_schema = expect(expect(rpc({
            "kind": "describe_schema",
            "data": {"projection": {"kind": "roots", "data": {"roots": roots}}},
        }, "task_contract"), "describe_schema"), "roots")
        unchanged = expect(expect(rpc({
            "kind": "describe_schema",
            "data": {
                "projection": {"kind": "full"},
                "known_digest": manifest_schema["digest"],
            },
        }, "schema_unchanged"), "describe_schema"), "unchanged")
        if task_schema["digest"] != manifest_schema["digest"] or unchanged["digest"] != manifest_schema["digest"]:
            raise RuntimeError("schema digest mismatch")

        workspace = expect(rpc({"kind": "create_workspace"}, "create_workspace"),
                           "workspace_created")["workspace"]
        operations = application_operations()
        creation = expect(rpc(apply_request(
            workspace, 0, "commit", operations, selected_symbols(),
        ), "create_incomplete_classifier"), "transaction_receipt")
        ids = {
            int(name.removeprefix("draft_")): node
            for name, node in creation["returned_bindings"]
        }
        if set(ids) != set(selected_symbols()) or creation["revision"] != 1 or creation["complete_after"]:
            raise RuntimeError("incomplete creation contract mismatch")

        blockers = expect(query(workspace, 1, {
            "kind": "blockers", "data": {"page": {"limit": 32}},
        }, "incomplete_blockers"), "blockers")
        if not any(
            item.get("target") == ids[310] and item.get("expected_type") == "bool"
            for item in blockers["items"]
        ):
            raise RuntimeError("reachable bool placeholder missing from blockers")
        context = expect(query(workspace, 1, {
            "kind": "repair_context",
            "data": {
                "target": {"kind": "hole", "data": ids[310]},
                "budget": {
                    "body_before": 8, "body_after": 8, "visible_values": 16,
                    "incoming_uses": 8, "include_incompatible": True,
                },
            },
        }, "payload_repair_context"), "repair_context")
        if context["operation"] != ids[310] or context["expected_type"] != "bool":
            raise RuntimeError("repair context target/type mismatch")
        if not any(item["producer"] == ids[301] and item["ty"] == "bytes"
                   for item in context["visible_values"]["items"]):
            raise RuntimeError("repair context omits manifest parameter")
        before_location = (context["owner_block"], context["owner_function"], context["ordinal"])
        before_uses = context["incoming_uses"]["items"]

        probe_before = allocation_probe(workspace, "probe_before_invalid_repair")
        invalid = expect_error(rpc(apply_request(workspace, 1, "commit", [{
            "kind": "refine_hole",
            "data": {
                "hole": existing(ids[310]),
                "replacement": {"kind": "const_bytes", "data": encode_bytes(b"wrong")},
            },
        }]), "invalid_bytes_repair"), "type_mismatch", ids[310])
        if invalid.get("expected_type") != "bool" or invalid.get("actual_type") != "bytes":
            raise RuntimeError("invalid repair omitted exact byte/bool types")
        probe_after = allocation_probe(workspace, "probe_after_invalid_repair")
        for field in ("revision", "hash", "created_count", "returned_bindings"):
            if probe_before[field] != probe_after[field]:
                raise RuntimeError("invalid repair consumed identity or changed prediction")
        summary_one = expect(query(workspace, 1, {"kind": "workspace_summary"},
                                   "summary_after_invalid"), "workspace_summary")
        if summary_one["revision"] != 1 or summary_one["complete"]:
            raise RuntimeError("invalid repair published")
        expect_error(run_request(
            workspace, 1, ids[900], manifest(), "run_incomplete_revision",
        ), "compile_incomplete")

        valid_operation = [{
            "kind": "refine_hole",
            "data": {
                "hole": existing(ids[310]),
                "replacement": {
                    "kind": "call",
                    "data": {
                        "function": existing(ids[200]),
                        "arguments": [parameter(ids[301], persisted=True)],
                    },
                },
            },
        }]
        predicted = expect(rpc(apply_request(
            workspace, 1, "validate_only", valid_operation,
        ), "validate_valid_repair"), "transaction_receipt")
        repaired = expect(rpc(apply_request(
            workspace, 1, "commit", valid_operation,
        ), "commit_valid_repair"), "transaction_receipt")
        if (
            repaired["revision"] != 2
            or repaired["created_count"] != 0
            or not repaired["complete_after"]
            or predicted["revision"] != repaired["revision"]
            or predicted["hash"] != repaired["hash"]
            or predicted["created_count"] != repaired["created_count"]
        ):
            raise RuntimeError("validate-only/commit refinement mismatch")

        post = query_batch(workspace, 2, [
            {"kind": "node", "data": {"node": ids[310], "expand": True}},
            {"kind": "body", "data": {
                "block": before_location[0], "page": {"limit": 32},
            }},
            {"kind": "incoming_uses", "data": {
                "value": {"kind": "operation_result", "data": {
                    "operation": ids[310], "output": 0,
                }},
                "page": {"limit": 32},
            }},
            {"kind": "semantic_diff", "data": {"from": 1, "page": {"limit": 16}}},
        ], "repair_identity_and_diff")
        repaired_node = expect(post[0], "node")
        repaired_body = expect(post[1], "body")
        repaired_uses = expect(post[2], "incoming_uses")
        repair_diff = expect(post[3], "semantic_diff")
        body_item = next(
            (item for item in repaired_body["items"] if item["operation"] == ids[310]), None,
        )
        target_kinds = sorted(
            item["kind"]["kind"] for item in repair_diff["page"]["items"]
            if item["node"] == ids[310]
        )
        if (
            repaired_node["summary"]["node"] != ids[310]
            or repaired_node["summary"]["owner"] != before_location[0]
            or body_item is None
            or body_item["ordinal"] != before_location[2]
            or body_item["code"] != "call"
            or repaired_uses["items"] != before_uses
            or target_kinds != ["operand_changed", "operation_refined"]
            or any(item["kind"]["kind"] in ("created", "deleted")
                   for item in repair_diff["page"]["items"])
        ):
            raise RuntimeError("identity-preserving repair facts mismatch")

        cases = []
        stable = manifest(channel=0)
        preview = manifest(channel=1, flags=1)
        cases.append(run_case(workspace, 2, ids, "accepted_stable", stable, accepted(ids, 11)))
        cases.append(run_case(workspace, 2, ids, "accepted_preview", preview, accepted(ids, 12)))
        wrong_length = stable[:-1]
        cases.append(run_case(workspace, 2, ids, "wrong_length", wrong_length, rejected(ids, 21)))
        wrong_magic = stable.copy(); wrong_magic[0] = ord("X")
        cases.append(run_case(workspace, 2, ids, "wrong_magic", wrong_magic, rejected(ids, 22)))
        wrong_format = stable.copy(); wrong_format[4] = 2
        cases.append(run_case(workspace, 2, ids, "unsupported_format", wrong_format, rejected(ids, 23)))
        wrong_target = stable.copy(); wrong_target[6] = 9
        cases.append(run_case(workspace, 2, ids, "unsupported_target", wrong_target, rejected(ids, 24)))
        wrong_channel = stable.copy(); wrong_channel[5] = 9
        cases.append(run_case(workspace, 2, ids, "forbidden_channel", wrong_channel, rejected(ids, 25)))
        wrong_flags = stable.copy(); wrong_flags[7] = 2
        cases.append(run_case(workspace, 2, ids, "invalid_flags", wrong_flags, rejected(ids, 26)))
        wrong_payload = stable.copy(); wrong_payload[13] = 0
        cases.append(run_case(workspace, 2, ids, "invalid_payload", wrong_payload, rejected(ids, 27)))
        wrong_terminal = stable.copy(); wrong_terminal[31] = 0
        cases.append(run_case(workspace, 2, ids, "invalid_terminal", wrong_terminal, rejected(ids, 27)))
        run_case(
            workspace, 2, ids, "lazy_wrong_length_low_fuel", bytearray(33),
            rejected(ids, 21), fuel=150,
        )
        expect_error(run_request(
            workspace, 2, ids[900], stable, "selected_payload_same_low_fuel", fuel=150,
        ), "execution_fuel_exhausted")
        expect_error(run_request(
            workspace, 2, ids[950], stable, "deterministic_bounds_probe",
        ), "byte_index_out_of_bounds")

        old_reason = expect(query(workspace, 2, nominal_query(ids[20]),
                                  "old_reason_name"), "nominal_type")
        if member_name(old_reason, ids[27]) != "invalid_payload":
            raise RuntimeError("revision two reason name mismatch")
        renamed = expect(rpc(apply_request(workspace, 2, "commit", [{
            "kind": "rename_node",
            "data": {"node": existing(ids[27]), "name": "payload_policy_failed"},
        }]), "rename_payload_reason"), "transaction_receipt")
        if renamed["revision"] != 3 or renamed["created_count"] != 0:
            raise RuntimeError("rename identity/allocation mismatch")
        rename_results = query_batch(workspace, 3, [
            {"kind": "semantic_diff", "data": {"from": 2, "page": {"limit": 8}}},
            nominal_query(ids[20]),
        ], "rename_diff")
        rename_diff = expect(rename_results[0], "semantic_diff")
        new_reason = expect(rename_results[1], "nominal_type")
        if (
            rename_diff["change_count"] != 1
            or rename_diff["page"]["items"][0]["node"] != ids[27]
            or member_name(new_reason, ids[27]) != "payload_policy_failed"
        ):
            raise RuntimeError("presentation rename diff mismatch")

        # Each direct CLI invocation reopens and validates durable state.
        summaries = [
            expect(query(workspace, revision, {"kind": "workspace_summary"},
                         f"restart_summary_{revision}"), "workspace_summary")
            for revision in (1, 2, 3)
        ]
        if [item["complete"] for item in summaries] != [False, True, True]:
            raise RuntimeError("historical completeness changed after restart")
        names = [
            member_name(expect(query(
                workspace, revision, nominal_query(ids[20]), f"restart_reason_{revision}",
            ), "nominal_type"), ids[27])
            for revision in (1, 2, 3)
        ]
        if names != ["invalid_payload", "invalid_payload", "payload_policy_failed"]:
            raise RuntimeError("historical presentation names changed after restart")
        expect_error(run_request(
            workspace, 1, ids[900], stable, "restart_incomplete_revision",
        ), "compile_incomplete")
        restart_two = run_case(
            workspace, 2, ids, "restart_repaired", stable, accepted(ids, 11),
        )
        restart_three = run_case(
            workspace, 3, ids, "restart_current", preview, accepted(ids, 12),
        )

        workspace_dir = state / "workspaces" / workspace
        artifact_sizes = {
            str(revision): (
                workspace_dir / "revisions" / f"{revision:020d}.lkjscript"
            ).stat().st_size
            for revision in (1, 2, 3)
        }
        head_size = (workspace_dir / "HEAD").stat().st_size
        counted = [item for item in measurements if item["counted"]]
        summary = {
            "schema": {
                "digest": manifest_schema["digest"],
                "roots": len(roots),
                "definitions": len(task_schema["definitions"]),
                "task_json_bytes": next(
                    item["json_response_bytes"] for item in measurements
                    if item["purpose"] == "task_contract"
                ),
                "unchanged": True,
            },
            "revisions": {"incomplete": 1, "repaired": 2, "renamed": 3},
            "repair": {
                "placeholder": ids[310], "identity_preserved": True,
                "invalid_code": invalid["code"], "allocator_rollback": True,
                "validate_only_exact": True,
            },
            "oracles": {
                "accepted": 2, "rejected": 8, "lazy": True,
                "bounds_trap": "byte_index_out_of_bounds", "restart": True,
            },
            "artifacts": {"revision_bytes": artifact_sizes, "head_bytes": head_size},
            "timings": {
                "case_compile_nanoseconds": sum(item["compile_nanoseconds"] for item in cases),
                "case_execute_nanoseconds": sum(item["execute_nanoseconds"] for item in cases),
                "restart_compile_nanoseconds": restart_two["compile_nanoseconds"] + restart_three["compile_nanoseconds"],
                "restart_execute_nanoseconds": restart_two["execute_nanoseconds"] + restart_three["execute_nanoseconds"],
            },
            "interaction": {
                "cli_launches": len(counted),
                "engine_opens": len(counted),
                "connections": 0,
                "request_bytes": sum(item["json_request_bytes"] for item in counted),
                "response_bytes": sum(item["json_response_bytes"] for item in counted),
                "wall_nanoseconds": sum(item["elapsed_nanoseconds"] for item in counted),
            },
            "reopen": "passed on every direct command",
        }
        if METRICS_PATH is not None:
            METRICS_PATH.write_text(json.dumps({
                "summary": summary, "measurements": measurements,
            }, separators=(",", ":")) + "\n")
        return summary


def main():
    print(json.dumps(workflow(), separators=(",", ":")))


if __name__ == "__main__":
    main()
