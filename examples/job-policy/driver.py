#!/usr/bin/env python3
"""Create, repair, rename, restart, and run the deterministic job policy via the public CLI."""

import json
import os
import pathlib
import subprocess
import sys
import tempfile
import time

CLI = pathlib.Path(sys.argv[1]).resolve()
DAEMON = pathlib.Path(sys.argv[2]).resolve()
METRICS_PATH = pathlib.Path(sys.argv[3]).resolve() if len(sys.argv) > 3 else None
request_id = 0
query_id = 0
daemon = None
state = None
measurements = []
readiness_nanoseconds = []
def draft_symbol(number):
    return f"draft_{number}"


def rpc(request, purpose, counted=True):
    global request_id
    request_id += 1
    envelope = {"version": 8, "request_id": request_id, "request": request}
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
    response_envelope = json.loads(completed.stdout)
    if response_envelope.get("version") != 8:
        raise RuntimeError(f"response version mismatch for {purpose}")
    if response_envelope.get("request_id") != request_id:
        raise RuntimeError(f"response correlation mismatch for {purpose}")
    measurements.append({
        "purpose": purpose,
        "counted": counted,
        "elapsed_nanoseconds": elapsed,
        "json_request_bytes": len(encoded),
        "json_response_bytes": len(completed.stdout),
        "request": envelope,
        "response": response_envelope,
    })
    return response_envelope["response"]


def expect(response, kind):
    if response.get("kind") != kind:
        raise RuntimeError(f"expected {kind}, received {response}")
    return response.get("data")


def expect_error(response, code, target=None):
    error = expect(response, "error")
    if error["code"] != code:
        raise RuntimeError(f"expected error {code}, received {error}")
    if target is not None and error.get("target") != target:
        raise RuntimeError(f"expected error target {target}, received {error}")
    return error


def local(symbol):
    return {"kind": "draft", "data": draft_symbol(symbol) if isinstance(symbol, int) else symbol}


def existing(node):
    return {"kind": "existing", "data": node}


def nominal(target):
    return {"nominal": target}


def result(symbol):
    return {"kind": "operation_result", "data": {"operation": local(symbol), "output": 0}}


def existing_result(node):
    return {
        "kind": "operation_result",
        "data": {"operation": existing(node), "output": 0},
    }


def parameter(symbol):
    return {"kind": "function_parameter", "data": local(symbol)}


def block_argument(symbol):
    return {"kind": "block_argument", "data": local(symbol)}


def expression(symbol, kind, data=None):
    operation = {"kind": kind}
    if data is not None:
        if kind == "for_i64":
            data = dict(data)
            data["index_symbol"] = draft_symbol(data["index_symbol"])
            data["carried_symbol"] = draft_symbol(data["carried_symbol"])
        operation["data"] = data
    return {"symbol": draft_symbol(symbol), "operation": operation}


def yielding(operations, value):
    return {"operations": operations, "yield_value": value}


def function(symbol, name, parameters, result_type, operations, return_value):
    direct_parameters = [
        {
            **parameter_value,
            "symbol": draft_symbol(parameter_value["symbol"]),
        }
        for parameter_value in parameters
    ]
    return {
        "kind": "create_function",
        "data": {
            "symbol": draft_symbol(symbol),
            "module": local(2),
            "name": name,
            "parameters": direct_parameters,
            "result": result_type,
            "body": {"operations": operations, "return_value": return_value},
        },
    }


def field(field_handle, value):
    return {"field": local(field_handle), "value": value}


def arm(variant_handle, body, payload_symbol=None):
    value = {"variant": local(variant_handle), "body": body}
    if payload_symbol is not None:
        value["payload_symbol"] = draft_symbol(payload_symbol)
    return value


def start_daemon():
    global daemon
    started = time.monotonic_ns()
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
    readiness_nanoseconds.append(time.monotonic_ns() - started)


def stop_daemon(purpose):
    global daemon
    if daemon is None:
        raise RuntimeError(f"daemon is absent before typed shutdown {purpose}")
    status = daemon.poll()
    if status is not None:
        raise RuntimeError(f"daemon exited before typed shutdown {purpose}: {status}")
    expect(rpc({"kind": "shutdown"}, purpose, counted=False), "acknowledged")
    if daemon.wait(timeout=5) != 0:
        raise RuntimeError("daemon shutdown failed")
    daemon = None


def query_batch(workspace, revision, queries, purpose):
    global query_id
    request_queries = []
    expected_ids = []
    for query_value in queries:
        query_id += 1
        expected_ids.append(query_id)
        request_queries.append({"id": query_id, "query": query_value})
    response = expect(
        rpc({
            "kind": "query_batch",
            "data": {
                "workspace": workspace,
                "revision": revision,
                "queries": request_queries,
            },
        }, purpose),
        "query_batch_result",
    )
    if [item["id"] for item in response["results"]] != expected_ids:
        raise RuntimeError(f"query correlation mismatch for {purpose}")
    return [expect(item["outcome"], "success") for item in response["results"]]


def query(workspace, revision, query_value, purpose):
    return query_batch(workspace, revision, [query_value], purpose)[0]


def run(workspace, revision, entry, arguments, purpose, fuel=1_000_000):
    return rpc({
        "kind": "run",
        "data": {
            "workspace": workspace,
            "revision": revision,
            "entry": entry,
            "arguments": arguments,
            "policy": {"fuel": fuel, "maximum_frames": 1_000},
        },
    }, purpose)


def product_value(type_id, fields):
    return {
        "kind": "product",
        "data": {
            "ty": type_id,
            "fields": [{"field": field_id, "value": value} for field_id, value in fields],
        },
    }


def variant_value(type_id, variant_id, value=None):
    data = {"ty": type_id, "variant": variant_id}
    if value is not None:
        data["payload"] = value
    return {"kind": "sum", "data": data}


def i64_value(value):
    return {"kind": "i64", "data": value}


def bool_value(value):
    return {"kind": "bool", "data": value}


def resources_value(ids, cpu, memory, trusted):
    # Deliberately not declaration order; the service normalizes by exact field identity.
    return product_value(ids[100], [
        (ids[103], bool_value(trusted)),
        (ids[102], i64_value(memory)),
        (ids[101], i64_value(cpu)),
    ])


def limits_value(ids, cpu, memory):
    return product_value(ids[110], [
        (ids[112], i64_value(memory)),
        (ids[111], i64_value(cpu)),
    ])


def target_value(ids, variant_handle):
    return variant_value(ids[120], ids[variant_handle])


def mode_value(ids, variant_handle):
    return variant_value(ids[130], ids[variant_handle])


def job_value(ids, cpu, memory, trusted, target_handle, mode_handle):
    return product_value(ids[140], [
        (ids[143], mode_value(ids, mode_handle)),
        (ids[142], target_value(ids, target_handle)),
        (ids[141], resources_value(ids, cpu, memory, trusted)),
    ])


def accepted_value(ids, score):
    return variant_value(ids[160], ids[161], i64_value(score))


def rejected_value(ids, reason_handle):
    reason = variant_value(ids[150], ids[reason_handle])
    return variant_value(ids[160], ids[162], reason)


def triangular_function():
    return function(
        200,
        "triangular",
        [{"symbol": 201, "name": "n", "ty": "i64"}],
        "i64",
        [
            expression(202, "const_i64", 0),
            expression(206, "for_i64", {
                "start": result(202),
                "end_exclusive": parameter(201),
                "step": 1,
                "initial": result(202),
                "carried": "i64",
                "index_symbol": 203,
                "carried_symbol": 204,
                "body": yielding([
                    expression(205, "add_i64", {
                        "lhs": block_argument(204),
                        "rhs": block_argument(203),
                    }),
                ], result(205)),
            }),
        ],
        result(206),
    )


def target_bonus_function():
    return function(
        210,
        "target_bonus",
        [{"symbol": 211, "name": "target", "ty": nominal(local(120))}],
        "i64",
        [expression(212, "match_sum", {
            "scrutinee": parameter(211),
            "result": "i64",
            "arms": [
                arm(123, yielding([expression(215, "const_i64", 0)], result(215))),
                arm(121, yielding([expression(213, "const_i64", 10)], result(213))),
                arm(122, yielding([expression(214, "const_i64", 5)], result(214))),
            ],
        })],
        result(212),
    )


def mode_bonus_function():
    return function(
        220,
        "mode_bonus",
        [{"symbol": 221, "name": "mode", "ty": nominal(local(130))}],
        "i64",
        [expression(222, "match_sum", {
            "scrutinee": parameter(221),
            "result": "i64",
            "arms": [
                arm(133, yielding([expression(225, "const_i64", 3)], result(225))),
                arm(131, yielding([expression(223, "const_i64", 1)], result(223))),
                arm(132, yielding([expression(224, "const_i64", 2)], result(224))),
            ],
        })],
        result(222),
    )


def score_function():
    return function(
        230,
        "score",
        [
            {"symbol": 231, "name": "resources", "ty": nominal(local(100))},
            {"symbol": 232, "name": "target", "ty": nominal(local(120))},
            {"symbol": 233, "name": "mode", "ty": nominal(local(130))},
        ],
        "i64",
        [
            expression(234, "project_field", {"value": parameter(231), "field": local(101)}),
            expression(235, "call", {"function": local(200), "arguments": [result(234)]}),
            expression(236, "project_field", {"value": parameter(231), "field": local(102)}),
            expression(237, "add_i64", {"lhs": result(235), "rhs": result(236)}),
            expression(238, "call", {"function": local(210), "arguments": [parameter(232)]}),
            expression(239, "add_i64", {"lhs": result(237), "rhs": result(238)}),
            expression(240, "call", {"function": local(220), "arguments": [parameter(233)]}),
            expression(241, "hole", {"expected": "i64"}),
        ],
        result(241),
    )


def finalize_function():
    release_body = yielding([
        expression(265, "project_field", {"value": parameter(261), "field": local(103)}),
        expression(270, "if", {
            "condition": result(265),
            "result": nominal(local(160)),
            "then_body": yielding([
                expression(266, "call", {
                    "function": local(230),
                    "arguments": [parameter(261), parameter(262), parameter(263)],
                }),
                expression(267, "construct_variant", {
                    "variant": local(161), "payload": result(266),
                }),
            ], result(267)),
            "else_body": yielding([
                expression(268, "construct_variant", {"variant": local(154)}),
                expression(269, "construct_variant", {
                    "variant": local(162), "payload": result(268),
                }),
            ], result(269)),
        }),
    ], result(270))
    check_body = yielding([
        expression(271, "call", {
            "function": local(230),
            "arguments": [parameter(261), parameter(262), parameter(263)],
        }),
        expression(272, "construct_variant", {"variant": local(161), "payload": result(271)}),
    ], result(272))
    build_body = yielding([
        expression(273, "call", {
            "function": local(230),
            "arguments": [parameter(261), parameter(262), parameter(263)],
        }),
        expression(274, "construct_variant", {"variant": local(161), "payload": result(273)}),
    ], result(274))
    return function(
        260,
        "finalize",
        [
            {"symbol": 261, "name": "resources", "ty": nominal(local(100))},
            {"symbol": 262, "name": "target", "ty": nominal(local(120))},
            {"symbol": 263, "name": "mode", "ty": nominal(local(130))},
        ],
        nominal(local(160)),
        [expression(264, "match_sum", {
            "scrutinee": parameter(263),
            "result": nominal(local(160)),
            "arms": [arm(133, release_body), arm(131, check_body), arm(132, build_body)],
        })],
        result(264),
    )


def decide_function():
    target_match = expression(320, "match_sum", {
        "scrutinee": result(304),
        "result": nominal(local(160)),
        "arms": [
            arm(123, yielding([
                expression(316, "construct_variant", {"variant": local(153)}),
                expression(317, "construct_variant", {
                    "variant": local(162), "payload": result(316),
                }),
            ], result(317))),
            arm(121, yielding([
                expression(318, "call", {
                    "function": local(260),
                    "arguments": [result(303), result(304), result(305)],
                }),
            ], result(318))),
            arm(122, yielding([
                expression(319, "call", {
                    "function": local(260),
                    "arguments": [result(303), result(304), result(305)],
                }),
            ], result(319))),
        ],
    })
    memory_else = yielding([target_match], result(320))
    cpu_else = yielding([
        expression(311, "project_field", {"value": parameter(302), "field": local(112)}),
        expression(312, "project_field", {"value": result(303), "field": local(102)}),
        expression(313, "lt_i64", {"lhs": result(311), "rhs": result(312)}),
        expression(314, "construct_variant", {"variant": local(152)}),
        expression(315, "construct_variant", {"variant": local(162), "payload": result(314)}),
        expression(321, "if", {
            "condition": result(313),
            "result": nominal(local(160)),
            "then_body": yielding([], result(315)),
            "else_body": memory_else,
        }),
    ], result(321))
    return function(
        300,
        "decide",
        [
            {"symbol": 301, "name": "job", "ty": nominal(local(140))},
            {"symbol": 302, "name": "limits", "ty": nominal(local(110))},
        ],
        nominal(local(160)),
        [
            expression(303, "project_field", {"value": parameter(301), "field": local(141)}),
            expression(304, "project_field", {"value": parameter(301), "field": local(142)}),
            expression(305, "project_field", {"value": parameter(301), "field": local(143)}),
            expression(306, "project_field", {"value": parameter(302), "field": local(111)}),
            expression(307, "project_field", {"value": result(303), "field": local(101)}),
            expression(308, "lt_i64", {"lhs": result(306), "rhs": result(307)}),
            expression(309, "construct_variant", {"variant": local(151)}),
            expression(310, "construct_variant", {"variant": local(162), "payload": result(309)}),
            expression(322, "if", {
                "condition": result(308),
                "result": nominal(local(160)),
                "then_body": yielding([], result(310)),
                "else_body": cpu_else,
            }),
        ],
        result(322),
    )


def main_function():
    return function(
        400,
        "main",
        [],
        nominal(local(160)),
        [
            expression(401, "const_i64", 4),
            expression(402, "const_i64", 8),
            expression(403, "const_bool", True),
            expression(404, "construct_product", {
                "product": local(100),
                "fields": [field(101, result(401)), field(102, result(402)), field(103, result(403))],
            }),
            expression(405, "construct_variant", {"variant": local(121)}),
            expression(406, "construct_variant", {"variant": local(131)}),
            expression(407, "construct_product", {
                "product": local(140),
                "fields": [field(141, result(404)), field(142, result(405)), field(143, result(406))],
            }),
            expression(408, "const_i64", 8),
            expression(409, "const_i64", 16),
            expression(410, "construct_product", {
                "product": local(110),
                "fields": [field(111, result(408)), field(112, result(409))],
            }),
            expression(411, "call", {"function": local(300), "arguments": [result(407), result(410)]}),
        ],
        result(411),
    )


def application_operations():
    operations = [
        {"kind": "create_package", "data": {"symbol": draft_symbol(1), "name": "job-policy"}},
        {"kind": "create_module", "data": {"symbol": draft_symbol(2), "package": local(1), "name": "root"}},
        triangular_function(),
        target_bonus_function(),
        mode_bonus_function(),
        score_function(),
        finalize_function(),
        decide_function(),
        main_function(),
        {"kind": "create_product_type", "data": {
            "symbol": draft_symbol(100), "module": local(2), "name": "Resources", "fields": [
                {"symbol": draft_symbol(101), "name": "cpu", "ty": "i64"},
                {"symbol": draft_symbol(102), "name": "memory", "ty": "i64"},
                {"symbol": draft_symbol(103), "name": "trusted", "ty": "bool"},
            ],
        }},
        {"kind": "create_product_type", "data": {
            "symbol": draft_symbol(110), "module": local(2), "name": "Limits", "fields": [
                {"symbol": draft_symbol(111), "name": "cpu", "ty": "i64"},
                {"symbol": draft_symbol(112), "name": "memory", "ty": "i64"},
            ],
        }},
        {"kind": "create_sum_type", "data": {
            "symbol": draft_symbol(120), "module": local(2), "name": "Target", "variants": [
                {"symbol": draft_symbol(121), "name": "linux_x64"},
                {"symbol": draft_symbol(122), "name": "wasm"},
                {"symbol": draft_symbol(123), "name": "unsupported"},
            ],
        }},
        {"kind": "create_sum_type", "data": {
            "symbol": draft_symbol(130), "module": local(2), "name": "Mode", "variants": [
                {"symbol": draft_symbol(131), "name": "check"},
                {"symbol": draft_symbol(132), "name": "build"},
                {"symbol": draft_symbol(133), "name": "release"},
            ],
        }},
        {"kind": "create_product_type", "data": {
            "symbol": draft_symbol(140), "module": local(2), "name": "Job", "fields": [
                {"symbol": draft_symbol(141), "name": "resources", "ty": nominal(local(100))},
                {"symbol": draft_symbol(142), "name": "target", "ty": nominal(local(120))},
                {"symbol": draft_symbol(143), "name": "mode", "ty": nominal(local(130))},
            ],
        }},
        {"kind": "create_sum_type", "data": {
            "symbol": draft_symbol(150), "module": local(2), "name": "RejectReason", "variants": [
                {"symbol": draft_symbol(151), "name": "cpu_limit"},
                {"symbol": draft_symbol(152), "name": "memory_limit"},
                {"symbol": draft_symbol(153), "name": "unsupported_target"},
                {"symbol": draft_symbol(154), "name": "untrusted_release"},
            ],
        }},
        {"kind": "create_sum_type", "data": {
            "symbol": draft_symbol(160), "module": local(2), "name": "Decision", "variants": [
                {"symbol": draft_symbol(161), "name": "accept", "payload": "i64"},
                {"symbol": draft_symbol(162), "name": "reject", "payload": nominal(local(150))},
            ],
        }},
        {"kind": "set_entry_function", "data": {"package": local(1), "function": local(400)}},
    ]
    return operations


def selected_symbols():
    return [
        100, 101, 102, 103,
        110, 111, 112,
        120, 121, 122, 123,
        130, 131, 132, 133,
        140, 141, 142, 143,
        150, 151, 152, 153, 154,
        160, 161, 162,
        239, 240, 241, 300, 400,
    ]


def count_program_facts(value):
    symbols = set()
    expression_count = 0
    yield_count = 0

    def visit(item):
        nonlocal expression_count, yield_count
        if isinstance(item, dict):
            for key in ("symbol", "index_symbol", "carried_symbol", "payload_symbol"):
                candidate = item.get(key)
                if isinstance(candidate, str):
                    symbols.add(candidate)
            if "operation" in item and "symbol" in item:
                expression_count += 1
            if "yield_value" in item and "operations" in item:
                yield_count += 1
            for child in item.values():
                visit(child)
        elif isinstance(item, list):
            for child in item:
                visit(child)

    visit(value)
    function_count = sum(1 for item in value if item.get("kind") == "create_function")
    declaration_count = sum(
        1 for item in value if item.get("kind") in ("create_product_type", "create_sum_type")
    )
    return {
        "explicit_symbols": len(symbols),
        "explicit_expressions": expression_count,
        "canonical_operations": expression_count + yield_count + function_count,
        "declarations": declaration_count,
        "functions": function_count,
    }


def apply_request(workspace, base_revision, mode, operations, return_symbols=None, key=None):
    transaction = {
        "workspace": workspace,
        "base_revision": base_revision,
        "mode": mode,
        "operations": operations,
    }
    if key is not None:
        transaction["idempotency_key"] = key
    return {
        "kind": "apply_transaction",
        "data": {
            "transaction": transaction,
            "response": {
                "return_symbols": [draft_symbol(symbol) if isinstance(symbol, int) else symbol for symbol in (return_symbols or [])]
            },
        },
    }


def allocation_probe(workspace, purpose):
    return expect(rpc(apply_request(
        workspace,
        1,
        "validate_only",
        [{"kind": "create_package", "data": {"symbol": draft_symbol(900), "name": "allocation-probe"}}],
        [900],
    ), purpose), "transaction_receipt")


def invalid_repair_request(workspace, ids):
    return apply_request(workspace, 1, "commit", [{
        "kind": "refine_hole",
        "data": {
            "hole": existing(ids[241]),
            "replacement": {
                "kind": "construct_variant",
                "data": {"variant": existing(ids[161]), "payload": existing_result(ids[239])},
            },
        },
    }])


def valid_repair_request(workspace, ids):
    return apply_request(workspace, 1, "commit", [{
        "kind": "refine_hole",
        "data": {
            "hole": existing(ids[241]),
            "replacement": {
                "kind": "add_i64",
                "data": {"lhs": existing_result(ids[239]), "rhs": existing_result(ids[240])},
            },
        },
    }])


def assert_run_value(workspace, revision, entry, arguments, expected, purpose, fuel=1_000_000):
    result_data = expect(run(workspace, revision, entry, arguments, purpose, fuel=fuel), "run")
    if result_data["value"] != expected:
        raise RuntimeError(f"unexpected Run result for {purpose}: {result_data['value']}")
    return result_data


def policy_cases(ids):
    return [
        ("case_a_linux_check", job_value(ids, 4, 8, True, 121, 131), limits_value(ids, 8, 16), accepted_value(ids, 25)),
        ("case_b_wasm_build", job_value(ids, 3, 5, True, 122, 132), limits_value(ids, 8, 16), accepted_value(ids, 15)),
        ("case_c_cpu_rejection", job_value(ids, 9, 1, True, 121, 131), limits_value(ids, 8, 16), rejected_value(ids, 151)),
        ("case_d_memory_rejection", job_value(ids, 2, 20, True, 121, 131), limits_value(ids, 8, 16), rejected_value(ids, 152)),
        ("case_e_target_rejection", job_value(ids, 2, 4, True, 123, 132), limits_value(ids, 8, 16), rejected_value(ids, 153)),
        ("case_f_untrusted_release", job_value(ids, 2, 4, False, 121, 133), limits_value(ids, 8, 16), rejected_value(ids, 154)),
        ("case_g_trusted_release", job_value(ids, 2, 4, True, 121, 133), limits_value(ids, 8, 16), accepted_value(ids, 18)),
    ]


def run_policy_cases(workspace, revision, ids, prefix):
    results = []
    for name, job, limits, expected in policy_cases(ids):
        results.append(assert_run_value(
            workspace,
            revision,
            ids[300],
            [job, limits],
            expected,
            f"{prefix}_{name}",
        ))
    return results


def nominal_type_query(declaration):
    return {"kind": "nominal_type", "data": {"declaration": declaration, "page": {"limit": 32}}}


def member_by_id(nominal_result, member_id):
    for member in nominal_result["members"]["items"]:
        data = member.get("data", {})
        if data.get("field") == member_id or data.get("variant") == member_id:
            return member
    raise RuntimeError(f"member {member_id} absent from named type context")


def assert_selected_nodes(workspace, revision, ids, purpose):
    queries = [
        {"kind": "node", "data": {"node": ids[symbol], "expand": False}}
        for symbol in selected_symbols()
    ]
    outcomes = query_batch(workspace, revision, queries, purpose)
    for symbol, outcome in zip(selected_symbols(), outcomes):
        view = expect(outcome, "node")
        if view["summary"]["node"] != ids[symbol]:
            raise RuntimeError(f"persistent identity changed for symbol {symbol} at revision {revision}")


def measurement_summary():
    counted = [item for item in measurements if item["counted"]]
    return {
        "json_request_bytes": sum(item["json_request_bytes"] for item in counted),
        "json_response_bytes": sum(item["json_response_bytes"] for item in counted),
        "cli_launches": len(counted),
        "daemon_round_trips": len(counted),
        "lifecycle_cli_launches": len(measurements) - len(counted),
        "cli_daemon_wall_nanoseconds": sum(item["elapsed_nanoseconds"] for item in counted),
    }


def execute():
    global state
    with tempfile.TemporaryDirectory(prefix="lkjscript-job-policy-") as directory:
        state = pathlib.Path(directory)
        os.chmod(state, 0o700)
        start_daemon()

        manifest = expect(expect(rpc({"kind": "describe_schema", "data": {
            "projection": {"kind": "manifest"},
        }}, "schema_manifest"), "describe_schema"), "manifest")
        digest = manifest["digest"]
        roots = [
            "create_workspace",
            "apply_transaction",
            "query_workspace_summary",
            "query_node",
            "query_blockers",
            "query_body",
            "query_incoming_uses",
            "query_repair_context",
            "query_semantic_diff",
            "query_nominal_type",
            "run",
            "shutdown",
        ]
        selected_contract = expect(expect(rpc({"kind": "describe_schema", "data": {
            "projection": {"kind": "roots", "data": {"roots": roots}},
        }}, "task_contract_roots"), "describe_schema"), "roots")
        if selected_contract["digest"] != digest or len(selected_contract["roots"]) != len(roots):
            raise RuntimeError("task contract projection mismatch")
        if len(selected_contract["definitions"]) <= len(roots):
            raise RuntimeError("task contract closure is incomplete")
        unchanged = expect(expect(rpc({"kind": "describe_schema", "data": {
            "projection": {"kind": "full"}, "known_digest": digest,
        }}, "known_fingerprint_unchanged"), "describe_schema"), "unchanged")
        if unchanged["digest"] != digest:
            raise RuntimeError("known-fingerprint response mismatch")

        created_workspace = expect(rpc({"kind": "create_workspace"}, "workspace_creation"), "workspace_created")
        workspace = created_workspace["workspace"]
        operations = application_operations()
        receipt = expect(rpc(apply_request(
            workspace,
            0,
            "commit",
            operations,
            selected_symbols(),
            "91919191919191919191919191919191",
        ), "job_policy_incomplete_creation"), "transaction_receipt")
        ids = {
            int(symbol.removeprefix("draft_")): node
            for symbol, node in receipt["returned_bindings"]
        }
        if set(ids) != set(selected_symbols()):
            raise RuntimeError("selected binding set does not match the driver need")
        if receipt["revision"] != 1 or receipt["complete_after"]:
            raise RuntimeError("initial job policy must be saved and incomplete")

        context = expect(query(workspace, 1, {"kind": "repair_context", "data": {
            "target": {"kind": "hole", "data": ids[241]},
            "budget": {
                "body_before": 16,
                "body_after": 8,
                "visible_values": 32,
                "incoming_uses": 8,
                "include_incompatible": True,
            },
        }}, "score_repair_context"), "repair_context")
        if context["operation"] != ids[241] or context["expected_type"] != "i64":
            raise RuntimeError("score repair context target/type mismatch")
        visible_producers = {item["producer"] for item in context["visible_values"]["items"]}
        if ids[239] not in visible_producers or ids[240] not in visible_producers:
            raise RuntimeError("score repair context omits required visible values")
        add = [item for item in context["legal_constructors"] if item["code"] == "add_i64"]
        if len(add) != 1 or not add[0]["direct_refinement"]:
            raise RuntimeError("score repair context omits the legal addition refinement")
        before_location = (context["owner_block"], context["owner_function"], context["ordinal"])
        before_uses = context["incoming_uses"]["items"]

        predicted_before = allocation_probe(workspace, "allocator_probe_before_invalid_repair")
        invalid_error = expect_error(
            rpc(invalid_repair_request(workspace, ids), "invalid_score_repair"),
            "type_mismatch",
            ids[241],
        )
        if invalid_error.get("expected_type") != "i64" or invalid_error.get("actual_type") != {"nominal": ids[160]}:
            raise RuntimeError(f"invalid repair lacks exact type facts: {invalid_error}")
        predicted_after = allocation_probe(workspace, "allocator_probe_after_invalid_repair")
        probe_fields = ("revision", "hash", "created_count", "returned_bindings")
        if any(predicted_before[field] != predicted_after[field] for field in probe_fields):
            raise RuntimeError("invalid repair consumed identity or changed predicted state")
        summary_one = expect(query(
            workspace, 1, {"kind": "workspace_summary"}, "workspace_after_invalid_repair"
        ), "workspace_summary")
        if summary_one["revision"] != 1 or summary_one["complete"]:
            raise RuntimeError("invalid repair published or completed the workspace")
        expect_error(run(workspace, 1, ids[400], [], "incomplete_main_run"), "compile_incomplete")

        repaired = expect(rpc(
            valid_repair_request(workspace, ids), "valid_identity_preserving_score_repair"
        ), "transaction_receipt")
        if repaired["revision"] != 2 or repaired["created_count"] != 0 or not repaired["complete_after"]:
            raise RuntimeError("valid score repair failed its publication/identity contract")

        post = query_batch(workspace, 2, [
            {"kind": "node", "data": {"node": ids[241], "expand": True}},
            {"kind": "body", "data": {"block": before_location[0], "page": {"limit": 32}}},
            {"kind": "incoming_uses", "data": {
                "value": {"kind": "operation_result", "data": {"operation": ids[241], "output": 0}},
                "page": {"limit": 32},
            }},
            {"kind": "workspace_summary"},
        ], "post_repair_identity_context")
        repaired_node = expect(post[0], "node")
        repaired_body = expect(post[1], "body")
        repaired_uses = expect(post[2], "incoming_uses")
        summary_two = expect(post[3], "workspace_summary")
        if repaired_node["summary"]["node"] != ids[241] or repaired_node["summary"]["owner"] != before_location[0]:
            raise RuntimeError("score placeholder identity or owner changed")
        body_item = next((item for item in repaired_body["items"] if item["operation"] == ids[241]), None)
        if body_item is None or body_item["ordinal"] != before_location[2] or body_item["code"] != "add_i64":
            raise RuntimeError("score placeholder body position or replacement code changed")
        if repaired_uses["items"] != before_uses:
            raise RuntimeError("score placeholder incoming uses changed during refinement")

        refinement_diff = expect(query(workspace, 2, {"kind": "semantic_diff", "data": {
            "from": 1, "page": {"limit": 16},
        }}, "refinement_semantic_diff"), "semantic_diff")
        refinement_items = refinement_diff["page"]["items"]
        refinement_changes = [
            item for item in refinement_items
            if item["node"] == ids[241] and item["kind"]["kind"] == "operation_refined"
        ]
        target_kinds = sorted(
            item["kind"]["kind"] for item in refinement_items if item["node"] == ids[241]
        )
        other_kinds = [
            item["kind"]["kind"] for item in refinement_items if item["node"] != ids[241]
        ]
        if (
            refinement_diff["change_count"] != 4
            or refinement_diff["page"].get("total") != 4
            or "next" in refinement_diff["page"]
            or len(refinement_changes) != 1
            or target_kinds != ["operand_changed", "operand_changed", "operation_refined"]
            or other_kinds != ["completeness_changed"]
            or any(item["kind"]["kind"] in ("created", "deleted") for item in refinement_items)
        ):
            raise RuntimeError("semantic diff does not exactly describe identity-preserving refinement")

        main_two = assert_run_value(
            workspace, 2, ids[400], [], accepted_value(ids, 25), "revision_two_main_case_h"
        )
        case_results_two = run_policy_cases(workspace, 2, ids, "revision_two")
        # Large accepted-work inputs plus low fuel prove unselected target and trust branches do not
        # enter scoring. Either accidental eager path would exhaust fuel in triangular(100000).
        assert_run_value(
            workspace,
            2,
            ids[300],
            [job_value(ids, 100_000, 4, True, 123, 132), limits_value(ids, 100_000, 16)],
            rejected_value(ids, 153),
            "revision_two_lazy_unsupported_target",
            fuel=1_000,
        )
        assert_run_value(
            workspace,
            2,
            ids[300],
            [job_value(ids, 100_000, 4, False, 121, 133), limits_value(ids, 100_000, 16)],
            rejected_value(ids, 154),
            "revision_two_lazy_untrusted_release",
            fuel=1_000,
        )

        type_handles = [100, 110, 120, 130, 140, 150, 160]
        type_outcomes = query_batch(
            workspace,
            2,
            [nominal_type_query(ids[symbol]) for symbol in type_handles],
            "runtime_named_type_context",
        )
        type_contexts = [expect(outcome, "nominal_type") for outcome in type_outcomes]
        if [item["name"] for item in type_contexts] != [
            "Resources", "Limits", "Target", "Mode", "Job", "RejectReason", "Decision"
        ]:
            raise RuntimeError("named type context does not match the runtime value vocabulary")
        old_resources = type_contexts[0]
        old_memory = member_by_id(old_resources, ids[102])
        if old_memory["data"]["name"] != "memory":
            raise RuntimeError("revision two must retain the original field display name")

        renamed = expect(rpc(apply_request(workspace, 2, "commit", [{
            "kind": "rename_node",
            "data": {"node": existing(ids[102]), "name": "memory_units"},
        }]), "resources_memory_display_rename"), "transaction_receipt")
        if renamed["revision"] != 3 or renamed["created_count"] != 0:
            raise RuntimeError("display rename unexpectedly changed identity allocation")

        rename_outcomes = query_batch(workspace, 3, [
            {"kind": "semantic_diff", "data": {"from": 2, "page": {"limit": 8}}},
            nominal_type_query(ids[100]),
        ], "rename_diff_and_named_type")
        rename_diff = expect(rename_outcomes[0], "semantic_diff")
        new_resources = expect(rename_outcomes[1], "nominal_type")
        rename_changes = [
            item for item in rename_diff["page"]["items"]
            if item["node"] == ids[102] and item["kind"]["kind"] == "renamed"
        ]
        if (
            rename_diff["change_count"] != 1
            or rename_diff["page"].get("total") != 1
            or "next" in rename_diff["page"]
            or len(rename_changes) != 1
            or len(rename_diff["page"]["items"]) != 1
            or rename_changes[0]["kind"]["data"] != {
                "before": "memory", "after": "memory_units"
            }
        ):
            raise RuntimeError("semantic diff does not report only the exact display rename")
        new_memory = member_by_id(new_resources, ids[102])
        if new_memory["data"]["name"] != "memory_units":
            raise RuntimeError("renamed revision does not expose the new display name")
        for key in ("field", "ordinal", "ty", "offset", "cells"):
            if old_memory["data"].get(key) != new_memory["data"].get(key):
                raise RuntimeError(f"rename changed field contract {key}")
        assert_run_value(
            workspace, 3, ids[400], [], accepted_value(ids, 25), "renamed_revision_main"
        )

        stop_daemon("shutdown_before_restart")
        start_daemon()

        assert_selected_nodes(workspace, 1, ids, "restart_revision_one_identities")
        assert_selected_nodes(workspace, 2, ids, "restart_revision_two_identities")
        assert_selected_nodes(workspace, 3, ids, "restart_revision_three_identities")
        old_one = expect(query(
            workspace, 1, nominal_type_query(ids[100]), "restart_incomplete_revision_name"
        ), "nominal_type")
        old_two = expect(query(
            workspace, 2, nominal_type_query(ids[100]), "restart_repaired_revision_name"
        ), "nominal_type")
        current_three = expect(query(
            workspace, 3, nominal_type_query(ids[100]), "restart_renamed_revision_name"
        ), "nominal_type")
        if member_by_id(old_one, ids[102])["data"]["name"] != "memory":
            raise RuntimeError("incomplete revision lost its historical field name")
        if member_by_id(old_two, ids[102])["data"]["name"] != "memory":
            raise RuntimeError("repaired revision lost its historical field name")
        if member_by_id(current_three, ids[102])["data"]["name"] != "memory_units":
            raise RuntimeError("current revision lost its renamed field name")

        expect_error(run(workspace, 1, ids[400], [], "restart_incomplete_main"), "compile_incomplete")
        assert_run_value(
            workspace, 2, ids[400], [], accepted_value(ids, 25), "restart_repaired_main"
        )
        restart_case_results = run_policy_cases(workspace, 3, ids, "restart_current")
        current_main = assert_run_value(
            workspace, 3, ids[400], [], accepted_value(ids, 25), "restart_current_main_case_h"
        )

        workspace_directory = state / "workspaces" / workspace
        artifact_sizes = {
            str(revision): (workspace_directory / "revisions" / f"{revision:020d}.lkjscript").stat().st_size
            for revision in (1, 2, 3)
        }
        head_size = (workspace_directory / "HEAD").stat().st_size
        program_facts = count_program_facts(operations)
        stop_daemon("final_shutdown")
        summary = {
            "schema": {
                "manifest": True,
                "roots": len(roots),
                "definitions": len(selected_contract["definitions"]),
                "unchanged": True,
                "digest": digest,
            },
            "revisions": {"incomplete": 1, "repaired": 2, "renamed": 3},
            "repair": {
                "rejected_code": invalid_error["code"],
                "allocator_rollback": True,
                "placeholder": ids[241],
                "owner_function": before_location[1],
                "ordinal": before_location[2],
                "change": "operation_refined",
            },
            "rename": {"node": ids[102], "before": "memory", "after": "memory_units"},
            "oracles": {
                "cases_a_through_h": True,
                "lazy_unselected_work": True,
                "restart": True,
                "exact_named_ids": True,
            },
            "counts": {
                "transaction_operations": len(operations),
                "selected_bindings": len(ids),
                "created_nodes": receipt["created_count"],
                "implicit_nodes": receipt["created_count"] - program_facts["explicit_symbols"],
                "canonical_nodes": summary_two["node_count"],
                **program_facts,
                "expected_rejected_proposals": 1,
            },
            "artifacts": {"revision_bytes": artifact_sizes, "head_bytes": head_size},
            "timings": {
                "cold_readiness_nanoseconds": readiness_nanoseconds[0],
                "restart_readiness_nanoseconds": readiness_nanoseconds[1],
                "main_revision_two_compile_nanoseconds": main_two["compile_nanoseconds"],
                "main_revision_two_execute_nanoseconds": main_two["execute_nanoseconds"],
                "case_revision_two_compile_nanoseconds": sum(item["compile_nanoseconds"] for item in case_results_two),
                "case_revision_two_execute_nanoseconds": sum(item["execute_nanoseconds"] for item in case_results_two),
                "restart_current_compile_nanoseconds": current_main["compile_nanoseconds"] + sum(item["compile_nanoseconds"] for item in restart_case_results),
                "restart_current_execute_nanoseconds": current_main["execute_nanoseconds"] + sum(item["execute_nanoseconds"] for item in restart_case_results),
            },
            "interaction": measurement_summary(),
            "shutdown": "acknowledged",
        }
        if summary["interaction"]["lifecycle_cli_launches"] != 2:
            raise RuntimeError("workflow must complete exactly two typed shutdowns")
        # Agent-workflow totals intentionally exclude both typed shutdowns.
        if METRICS_PATH is not None:
            METRICS_PATH.write_text(json.dumps({
                "summary": summary,
                "measurements": measurements,
            }, separators=(",", ":")) + "\n")
        return summary


def main():
    summary = execute()
    print(json.dumps(summary, separators=(",", ":")))


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
