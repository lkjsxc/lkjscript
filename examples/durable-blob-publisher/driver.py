#!/usr/bin/env python3
"""Author and operate a durable immutable-blob publisher through public boundaries."""

import base64
import json
import pathlib
import shutil
import subprocess
import sys
import tempfile
import time


if len(sys.argv) not in (2, 3) or (len(sys.argv) == 3 and sys.argv[2] != "--runtime-session"):
    raise SystemExit("usage: driver.py LKJSCRIPT [--runtime-session]")

CLI = pathlib.Path(sys.argv[1]).resolve()
USE_RUNTIME_SESSION = len(sys.argv) == 3
measurements = []
request_id = 0
session = None
state_root = None
runtime_session = None
runtime_store = None
runtime_request_id = 0
runtime_inspection = None


def b64(value):
    return base64.urlsafe_b64encode(bytes(value)).rstrip(b"=").decode("ascii")


def decode_b64(value):
    encoded = value.encode("ascii")
    return base64.urlsafe_b64decode(encoded + b"=" * (-len(encoded) % 4))


def symbol(number):
    return f"draft_{number}"


def local(number):
    return {"kind": "draft", "data": symbol(number)}


def nominal(number):
    return {"nominal": local(number)}


def parameter(number):
    return {"kind": "function_parameter", "data": local(number)}


def block_argument(number):
    return {"kind": "block_argument", "data": local(number)}


def result(number):
    return {"kind": "operation_result", "data": {"operation": local(number), "output": 0}}


def expression(number, kind, data=None):
    operation = {"kind": kind}
    if data is not None:
        operation["data"] = data
    return {"symbol": symbol(number), "operation": operation}


def yielding(operations, value):
    return {"operations": operations, "yield_value": value}


def arm(variant, body, payload_symbol=None):
    value = {"variant": local(variant), "body": body}
    if payload_symbol is not None:
        value["payload_symbol"] = symbol(payload_symbol)
    return value


def function(number, name, parameters, result_type, operations, return_value):
    return {
        "kind": "create_function",
        "data": {
            "symbol": symbol(number),
            "module": local(2),
            "name": name,
            "parameters": [{**item, "symbol": symbol(item["symbol"])} for item in parameters],
            "result": result_type,
            "body": {"operations": operations, "return_value": return_value},
        },
    }


def call(number, function_id, arguments):
    return expression(number, "call", {"function": local(function_id), "arguments": arguments})


def put_decision(start, content, response):
    return yielding([
        expression(start, "construct_variant", {"variant": local(102), "payload": content}),
        expression(start + 1, "construct_variant", {"variant": local(301), "payload": content}),
        expression(start + 2, "construct_variant", {"variant": local(341), "payload": result(start + 1)}),
        expression(start + 3, "const_bytes", b64(response)),
        call(start + 4, 410, [result(start), result(start + 3), result(start + 2)]),
    ], result(start + 4))


def inspect_decision(start, content, digest, response):
    return yielding([
        expression(start, "construct_product", {
            "product": local(150),
            "fields": [
                {"field": local(151), "value": content},
                {"field": local(152), "value": digest},
            ],
        }),
        expression(start + 1, "construct_variant", {"variant": local(105), "payload": result(start)}),
        expression(start + 2, "construct_variant", {"variant": local(302), "payload": digest}),
        expression(start + 3, "construct_variant", {"variant": local(341), "payload": result(start + 2)}),
        expression(start + 4, "const_bytes", b64(response)),
        call(start + 5, 410, [result(start + 1), result(start + 4), result(start + 3)]),
    ], result(start + 5))


def completed_state(start, variant, payload, response):
    return yielding([
        expression(start, "construct_variant", {"variant": local(variant), "payload": payload}),
        expression(start + 1, "const_bytes", b64(response)),
        call(start + 2, 400, [result(start), result(start + 1)]),
    ], result(start + 2))


def state_content_function():
    arms = []
    for offset, variant in enumerate([101, 102, 103, 104, 107, 108]):
        payload = 450 + offset
        arms.append(arm(variant, yielding([], block_argument(payload)), payload))
    for offset, variant in enumerate([105, 106]):
        payload = 460 + offset
        operation = 470 + offset
        arms.append(arm(
            variant,
            yielding([
                expression(operation, "project_field", {"value": block_argument(payload), "field": local(151)}),
            ], result(operation)),
            payload,
        ))
    return function(
        440,
        "state_content",
        [{"symbol": 441, "name": "state", "ty": nominal(100)}],
        "bytes",
        [expression(442, "match_sum", {
            "scrutinee": parameter(441), "result": "bytes", "arms": arms,
        })],
        result(442),
    )


def event_function():
    publish = yielding([
        expression(520, "bytes_len", {"value": block_argument(511)}),
        expression(521, "const_i64", 4097),
        expression(522, "lt_i64", {"lhs": result(520), "rhs": result(521)}),
        expression(523, "if", {
            "condition": result(522),
            "result": nominal(380),
            "then_body": put_decision(540, block_argument(511), b"blob_put_requested"),
            "else_body": completed_state(560, 108, block_argument(511), b"blob_too_large"),
        }),
    ], result(523))
    retry = yielding([
        call(580, 440, [parameter(501)]),
        expression(581, "const_bytes", b64(b"blob_put_retried")),
        expression(582, "construct_variant", {"variant": local(102), "payload": result(580)}),
        expression(583, "construct_variant", {"variant": local(301), "payload": result(580)}),
        expression(584, "construct_variant", {"variant": local(341), "payload": result(583)}),
        call(585, 410, [result(582), result(581), result(584)]),
    ], result(585))
    cancel = completed_state(600, 107, result(599), b"cancelled")
    cancel = yielding([
        expression(599, "const_bytes", b64(b"")),
        *cancel["operations"],
    ], cancel["yield_value"])
    status = yielding([
        expression(620, "const_bytes", b64(b"status")),
        call(621, 400, [parameter(501), result(620)]),
    ], result(621))
    return function(
        500,
        "transition_event",
        [
            {"symbol": 501, "name": "state", "ty": nominal(100)},
            {"symbol": 502, "name": "event", "ty": nominal(200)},
        ],
        nominal(380),
        [expression(510, "match_sum", {
            "scrutinee": parameter(502),
            "result": nominal(380),
            "arms": [
                arm(201, publish, 511),
                arm(202, retry),
                arm(203, cancel),
                arm(204, status),
            ],
        })],
        result(510),
    )


def resume_function():
    stored = completed_state(1000, 103, block_argument(721), b"blob_stored")
    already = completed_state(1030, 103, block_argument(722), b"blob_already_present")
    failed = yielding([
        call(1060, 440, [parameter(701)]),
        expression(1061, "construct_variant", {"variant": local(104), "payload": result(1060)}),
        expression(1062, "const_bytes", b64(b"blob_put_failed")),
        call(1063, 400, [result(1061), result(1062)]),
    ], result(1063))
    unknown = yielding([
        call(1090, 440, [parameter(701)]),
        *inspect_decision(1091, result(1090), block_argument(724), b"blob_visibility_unknown")["operations"],
    ], result(1096))
    present = completed_state(1130, 103, block_argument(725), b"blob_reconciled_present")
    absent = yielding([
        call(1160, 440, [parameter(701)]),
        expression(1161, "construct_variant", {"variant": local(104), "payload": result(1160)}),
        expression(1162, "const_bytes", b64(b"blob_reconciled_absent")),
        call(1163, 400, [result(1161), result(1162)]),
    ], result(1163))
    indeterminate = yielding([
        call(1190, 440, [parameter(701)]),
        expression(1191, "construct_product", {
            "product": local(150),
            "fields": [
                {"field": local(151), "value": result(1190)},
                {"field": local(152), "value": block_argument(727)},
            ],
        }),
        expression(1192, "construct_variant", {"variant": local(106), "payload": result(1191)}),
        expression(1193, "const_bytes", b64(b"blob_reconciliation_indeterminate")),
        call(1194, 400, [result(1192), result(1193)]),
    ], result(1194))
    interface_body = yielding([
        expression(715, "match_sum", {
            "scrutinee": block_argument(711),
            "result": nominal(380),
            "arms": [
                arm(321, stored, 721),
                arm(322, already, 722),
                arm(323, failed, 723),
                arm(324, unknown, 724),
                arm(325, present, 725),
                arm(326, absent, 726),
                arm(327, indeterminate, 727),
            ],
        }),
    ], result(715))
    return function(
        700,
        "transition_resume",
        [
            {"symbol": 701, "name": "state", "ty": nominal(100)},
            {"symbol": 702, "name": "outcome", "ty": nominal(350)},
        ],
        nominal(380),
        [expression(710, "match_sum", {
            "scrutinee": parameter(702),
            "result": nominal(380),
            "arms": [arm(351, interface_body, 711)],
        })],
        result(710),
    )


def operations():
    return [
        {"kind": "create_package", "data": {"symbol": symbol(1), "name": "durable-blob-publisher"}},
        {"kind": "create_module", "data": {"symbol": symbol(2), "package": local(1), "name": "main"}},
        {"kind": "create_product_type", "data": {
            "symbol": symbol(150), "module": local(2), "name": "BlobCorrelation",
            "fields": [
                {"symbol": symbol(151), "name": "content", "ty": "bytes"},
                {"symbol": symbol(152), "name": "digest", "ty": "bytes"},
            ],
        }},
        {"kind": "create_sum_type", "data": {
            "symbol": symbol(100), "module": local(2), "name": "PublisherState",
            "variants": [
                {"symbol": symbol(101), "name": "idle", "payload": "bytes"},
                {"symbol": symbol(102), "name": "putting", "payload": "bytes"},
                {"symbol": symbol(103), "name": "published", "payload": "bytes"},
                {"symbol": symbol(104), "name": "retryable_failure", "payload": "bytes"},
                {"symbol": symbol(105), "name": "inspecting", "payload": nominal(150)},
                {"symbol": symbol(106), "name": "outcome_unknown", "payload": nominal(150)},
                {"symbol": symbol(107), "name": "cancelled", "payload": "bytes"},
                {"symbol": symbol(108), "name": "terminal_failure", "payload": "bytes"},
            ],
        }},
        {"kind": "create_sum_type", "data": {
            "symbol": symbol(200), "module": local(2), "name": "PublisherEvent",
            "variants": [
                {"symbol": symbol(201), "name": "publish", "payload": "bytes"},
                {"symbol": symbol(202), "name": "retry"},
                {"symbol": symbol(203), "name": "cancel"},
                {"symbol": symbol(204), "name": "status"},
            ],
        }},
        {"kind": "create_sum_type", "data": {
            "symbol": symbol(300), "module": local(2), "name": "BlobRequest",
            "variants": [
                {"symbol": symbol(301), "name": "put", "payload": "bytes"},
                {"symbol": symbol(302), "name": "inspect", "payload": "bytes"},
            ],
        }},
        {"kind": "create_sum_type", "data": {
            "symbol": symbol(320), "module": local(2), "name": "BlobOutcome",
            "variants": [
                {"symbol": symbol(321), "name": "stored", "payload": "bytes"},
                {"symbol": symbol(322), "name": "already_present", "payload": "bytes"},
                {"symbol": symbol(323), "name": "put_failed", "payload": "bytes"},
                {"symbol": symbol(324), "name": "put_unknown", "payload": "bytes"},
                {"symbol": symbol(325), "name": "inspect_present", "payload": "bytes"},
                {"symbol": symbol(326), "name": "inspect_absent", "payload": "bytes"},
                {"symbol": symbol(327), "name": "inspect_indeterminate", "payload": "bytes"},
            ],
        }},
        {"kind": "create_sum_type", "data": {
            "symbol": symbol(340), "module": local(2), "name": "PublisherCommand",
            "variants": [{"symbol": symbol(341), "name": "blob", "payload": nominal(300)}],
        }},
        {"kind": "create_sum_type", "data": {
            "symbol": symbol(350), "module": local(2), "name": "PublisherOutcome",
            "variants": [{"symbol": symbol(351), "name": "blob", "payload": nominal(320)}],
        }},
        {"kind": "create_product_type", "data": {
            "symbol": symbol(360), "module": local(2), "name": "CompletedTransition",
            "fields": [
                {"symbol": symbol(361), "name": "state", "ty": nominal(100)},
                {"symbol": symbol(362), "name": "response", "ty": "bytes"},
            ],
        }},
        {"kind": "create_product_type", "data": {
            "symbol": symbol(370), "module": local(2), "name": "SuspendedTransition",
            "fields": [
                {"symbol": symbol(371), "name": "state", "ty": nominal(100)},
                {"symbol": symbol(372), "name": "response", "ty": "bytes"},
                {"symbol": symbol(373), "name": "command", "ty": nominal(340)},
            ],
        }},
        {"kind": "create_sum_type", "data": {
            "symbol": symbol(380), "module": local(2), "name": "TransitionDecision",
            "variants": [
                {"symbol": symbol(381), "name": "completed", "payload": nominal(360)},
                {"symbol": symbol(382), "name": "suspended", "payload": nominal(370)},
            ],
        }},
        function(
            400, "make_completed",
            [
                {"symbol": 401, "name": "state", "ty": nominal(100)},
                {"symbol": 402, "name": "response", "ty": "bytes"},
            ],
            nominal(380),
            [
                expression(405, "construct_product", {
                    "product": local(360),
                    "fields": [
                        {"field": local(361), "value": parameter(401)},
                        {"field": local(362), "value": parameter(402)},
                    ],
                }),
                expression(406, "construct_variant", {"variant": local(381), "payload": result(405)}),
            ],
            result(406),
        ),
        function(
            410, "make_suspended",
            [
                {"symbol": 411, "name": "state", "ty": nominal(100)},
                {"symbol": 412, "name": "response", "ty": "bytes"},
                {"symbol": 413, "name": "command", "ty": nominal(340)},
            ],
            nominal(380),
            [
                expression(415, "construct_product", {
                    "product": local(370),
                    "fields": [
                        {"field": local(371), "value": parameter(411)},
                        {"field": local(372), "value": parameter(412)},
                        {"field": local(373), "value": parameter(413)},
                    ],
                }),
                expression(416, "construct_variant", {"variant": local(382), "payload": result(415)}),
            ],
            result(416),
        ),
        state_content_function(),
        event_function(),
        resume_function(),
        function(
            900, "identity",
            [{"symbol": 901, "name": "input", "ty": "bytes"}],
            "bytes", [], parameter(901),
        ),
        {"kind": "set_entry_function", "data": {"package": local(1), "function": local(500)}},
    ]


def bytes_value(value):
    return {"kind": "bytes", "data": b64(value)}


def run_process(arguments, value=None, expected=0):
    if runtime_session is not None and arguments and arguments[0] == "instance":
        requested_store = option_value(arguments, "--store")
        if requested_store is not None and pathlib.Path(requested_store).resolve() == runtime_store:
            completed = runtime_instance_process(arguments, value)
            if expected is not None and completed.returncode != expected:
                raise RuntimeError(
                    f"runtime session command {arguments} returned {completed.returncode}, "
                    f"expected {expected}: stdout={completed.stdout!r} stderr={completed.stderr!r}"
                )
            return completed
    encoded = b"" if value is None else json.dumps(value, separators=(",", ":")).encode()
    started = time.monotonic_ns()
    completed = subprocess.run(
        [str(CLI), *arguments],
        input=encoded,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        check=False,
        timeout=60,
    )
    measurements.append({
        "command": " ".join(arguments[:2]),
        "input_bytes": len(encoded),
        "output_bytes": len(completed.stdout),
        "diagnostic_bytes": len(completed.stderr),
        "elapsed_nanoseconds": time.monotonic_ns() - started,
        "exit": completed.returncode,
        "process_started": True,
    })
    if expected is not None and completed.returncode != expected:
        raise RuntimeError(
            f"command {arguments} returned {completed.returncode}, expected {expected}: "
            f"stdout={completed.stdout!r} stderr={completed.stderr!r}"
        )
    return completed


def option_value(arguments, option):
    try:
        index = arguments.index(option)
    except ValueError:
        return None
    if index + 1 >= len(arguments):
        return None
    return arguments[index + 1]


def start_runtime_session(store):
    global runtime_session, runtime_store
    runtime_store = store.resolve()
    runtime_session = subprocess.Popen(
        [str(CLI), "runtime", "session", "--store", str(runtime_store)],
        stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )


def runtime_exchange(request):
    global runtime_request_id
    runtime_request_id += 1
    envelope = {"version": 1, "request_id": runtime_request_id, "request": request}
    encoded = json.dumps(envelope, separators=(",", ":")).encode()
    started = time.monotonic_ns()
    runtime_session.stdin.write(encoded + b"\n")
    runtime_session.stdin.flush()
    response = runtime_session.stdout.readline()
    elapsed = time.monotonic_ns() - started
    if not response:
        diagnostic = runtime_session.stderr.read()
        raise RuntimeError(f"runtime session ended without a response: {diagnostic!r}")
    decoded = json.loads(response)
    if decoded.get("request_id") != runtime_request_id:
        raise RuntimeError("runtime response correlation failed")
    measurements.append({
        "command": "runtime session",
        "input_bytes": len(encoded),
        "output_bytes": len(response),
        "diagnostic_bytes": 0,
        "elapsed_nanoseconds": elapsed,
        "exit": 0 if "response" in decoded else 1,
        "process_started": False,
    })
    return decoded


def runtime_instance_process(arguments, value):
    command = arguments[1]
    if command == "create":
        request = {
            "kind": "create",
            "data": {
                "application": option_value(arguments, "--application"),
                "request": value,
            },
        }
    elif command in {
        "validate-event", "apply-event", "execute-host", "fake-outcome",
        "validate-resume", "resume", "delete",
    }:
        request = {"kind": command.replace("-", "_"), "data": value}
    elif command == "inspect":
        request = {
            "kind": "inspect_instance",
            "data": {"instance": option_value(arguments, "--instance")},
        }
    elif command == "history":
        request = {
            "kind": "history",
            "data": {
                "instance": option_value(arguments, "--instance"),
                "start_revision": int(option_value(arguments, "--start")),
                "limit": int(option_value(arguments, "--limit")),
            },
        }
    else:
        raise RuntimeError(f"unsupported runtime-session instance command: {arguments}")
    decoded = runtime_exchange(request)
    if "error" in decoded:
        output = json.dumps({"error": decoded["error"]}, separators=(",", ":")).encode()
        return subprocess.CompletedProcess(arguments, 1, output, b"")
    response = decoded["response"]
    output = json.dumps(response.get("data", {}), separators=(",", ":")).encode()
    return subprocess.CompletedProcess(arguments, 0, output, b"")


def inspect_runtime_session():
    decoded = runtime_exchange({"kind": "inspect_runtime"})
    if "response" not in decoded or decoded["response"].get("kind") != "runtime":
        raise RuntimeError(f"runtime inspection failed: {decoded}")
    return decoded["response"]["data"]


def stop_runtime_session():
    global runtime_session, runtime_store
    decoded = runtime_exchange({"kind": "shutdown"})
    if "response" not in decoded or decoded["response"].get("kind") != "shutdown":
        raise RuntimeError(f"runtime shutdown failed: {decoded}")
    runtime_session.stdin.close()
    if runtime_session.wait(timeout=10) != 0:
        raise RuntimeError("runtime session failed")
    diagnostic = runtime_session.stderr.read()
    if diagnostic:
        raise RuntimeError(f"runtime session diagnostic: {diagnostic!r}")
    runtime_session = None
    runtime_store = None


def json_command(arguments, value=None, expected=0):
    completed = run_process(arguments, value, expected)
    try:
        return json.loads(completed.stdout)
    except json.JSONDecodeError as error:
        raise RuntimeError(f"command returned invalid JSON: {arguments}") from error


def expect_error(arguments, value, code):
    completed = run_process(arguments, value, expected=None)
    if completed.returncode == 0:
        raise RuntimeError(f"command unexpectedly succeeded: {arguments}")
    decoded = json.loads(completed.stdout)
    if decoded.get("error", {}).get("code") != code:
        raise RuntimeError(f"expected {code}, received {decoded}")
    return decoded


def start_session():
    global session
    session = subprocess.Popen(
        [str(CLI), "--state", str(state_root), "session"],
        stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )


def stop_session():
    global session
    session.stdin.close()
    if session.wait(timeout=10) != 0:
        raise RuntimeError("authoring session failed")
    diagnostic = session.stderr.read()
    if diagnostic:
        raise RuntimeError(f"authoring session diagnostic: {diagnostic!r}")
    session = None


def rpc(request):
    global request_id
    request_id += 1
    envelope = {"version": 10, "request_id": request_id, "request": request}
    encoded = json.dumps(envelope, separators=(",", ":")).encode()
    started = time.monotonic_ns()
    session.stdin.write(encoded + b"\n")
    session.stdin.flush()
    response = session.stdout.readline()
    measurements.append({
        "command": "authoring session",
        "input_bytes": len(encoded),
        "output_bytes": len(response),
        "diagnostic_bytes": 0,
        "elapsed_nanoseconds": time.monotonic_ns() - started,
        "exit": 0,
        "process_started": False,
    })
    decoded = json.loads(response)
    if decoded.get("request_id") != request_id:
        raise RuntimeError("authoring response correlation failed")
    if "response" not in decoded:
        raise RuntimeError(f"authoring request failed: {decoded}")
    return decoded["response"]


def expect(response, kind):
    if response.get("kind") != kind:
        raise RuntimeError(f"expected {kind}, received {response}")
    return response.get("data")


def app_target(release, item):
    return {"release": release, "item": item}


def export_map(receipt):
    return {item["name"]: item for item in receipt["inspection"]["exports"]}


def build_types(exports):
    names = [
        "correlation", "state", "event", "blob_request", "blob_outcome", "command", "outcome",
        "completed_payload", "suspended_payload", "decision",
    ]
    types = {}
    for name in names:
        exported = exports[name]
        data = exported["signature"]["data"]
        item = {"target": exported["target"]}
        if "variants" in data:
            item["variants"] = {entry["name"]: entry["target"] for entry in data["variants"]}
        if "fields" in data:
            item["fields"] = {entry["name"]: entry["target"] for entry in data["fields"]}
        types[name] = item
    return types


def app_sum(release, types, ty_name, variant_name, payload=None):
    ty = types[ty_name]
    data = {
        "ty": app_target(release, ty["target"]),
        "variant": app_target(release, ty["variants"][variant_name]),
    }
    if payload is not None:
        data["payload"] = payload
    return {"kind": "sum", "data": data}


def app_product(release, types, ty_name, fields):
    ty = types[ty_name]
    return {
        "kind": "product",
        "data": {
            "ty": app_target(release, ty["target"]),
            "fields": [
                {"field": app_target(release, ty["fields"][name]), "value": value}
                for name, value in fields
            ],
        },
    }


def app_state(release, types, variant, payload):
    return app_sum(release, types, "state", variant, payload)


def app_bytes_state(release, types, variant, payload):
    return app_state(release, types, variant, bytes_value(payload))


def app_event(release, types, variant, payload=None):
    return app_sum(
        release,
        types,
        "event",
        variant,
        None if payload is None else bytes_value(payload),
    )


def app_command(release, types, operation, payload):
    request = app_sum(release, types, "blob_request", operation, bytes_value(payload))
    return app_sum(release, types, "command", "blob", request)


def app_completed(release, types, state, response):
    payload = app_product(release, types, "completed_payload", [
        ("state", state), ("response", bytes_value(response)),
    ])
    return app_sum(release, types, "decision", "completed", payload)


def app_suspended(release, types, state, response, command):
    payload = app_product(release, types, "suspended_payload", [
        ("state", state),
        ("response", bytes_value(response)),
        ("command", command),
    ])
    return app_sum(release, types, "decision", "suspended", payload)


def app_outcome(release, types, variant, evidence):
    interface_outcome = app_sum(
        release, types, "blob_outcome", variant, bytes_value(evidence),
    )
    return app_sum(release, types, "outcome", "blob", interface_outcome)


def instance_event(instance, revision, key, event, mode="commit"):
    request = {
        "version": 2,
        "mode": mode,
        "instance": instance,
        "base_revision": revision,
        "event": event,
    }
    if key is not None:
        request["event_key"] = key
    return request


def resume_request(instance, revision, key, mode="commit"):
    request = {
        "version": 2,
        "mode": mode,
        "instance": instance,
        "base_revision": revision,
    }
    if key is not None:
        request["event_key"] = key
    return request


def make_grant(instance, name, adapter, namespace):
    return {
        "version": 2,
        "name": name,
        "instance": instance,
        "slot": "objects",
        "interface": "immutable_blob",
        "adapter": adapter,
        "descriptor": {
            "kind": "immutable_blob",
            "data": {
                "namespace": str(namespace),
                "maximum_objects": 4,
                "maximum_bytes": 16384,
            },
        },
    }


def operate(root, application_path, release, types):
    global runtime_inspection
    store = root / "instances"
    store.mkdir(mode=0o700)
    if USE_RUNTIME_SESSION:
        start_runtime_session(store)
    primary_namespace = root / "primary-objects"
    primary_namespace.mkdir(mode=0o700)
    fake_namespace = root / "fake-objects"
    fake_namespace.mkdir(mode=0o700)
    denied_namespace = root / "denied-objects"
    denied_namespace.mkdir(mode=0o700)
    primary = "33333333333333333333333333333333"
    secondary = "44444444444444444444444444444444"
    primary_grant = make_grant(primary, "primary_objects", "production", primary_namespace)
    initial = app_bytes_state(release, types, "idle", b"")
    create = {
        "version": 2,
        "mode": "validate_only",
        "instance": primary,
        "initial_state": initial,
        "grants": [primary_grant],
    }
    validated = json_command(
        ["instance", "create", "--store", str(store), "--application", str(application_path)],
        create,
    )
    if validated["published"]:
        raise RuntimeError("validate-only blob instance creation published")
    create["mode"] = "commit"
    json_command(
        ["instance", "create", "--store", str(store), "--application", str(application_path)],
        create,
    )

    exact = json_command(
        ["instance", "validate-event", "--store", str(store)],
        instance_event(primary, 0, None, app_event(release, types, "publish", b"x" * 4096), "validate_only"),
    )
    one_over = json_command(
        ["instance", "validate-event", "--store", str(store)],
        instance_event(primary, 0, None, app_event(release, types, "publish", b"x" * 4097), "validate_only"),
    )
    if exact["status"] != "suspended" or one_over["status"] != "completed":
        raise RuntimeError("semantic blob business limit is not exact")

    content = b"durable immutable payload"
    publish = app_event(release, types, "publish", content)
    predicted = json_command(
        ["instance", "validate-event", "--store", str(store)],
        instance_event(primary, 0, None, publish, "validate_only"),
    )
    request = instance_event(primary, 0, "publish-1", publish)
    receipt = json_command(["instance", "apply-event", "--store", str(store)], request)
    if predicted["state_digest"] != receipt["state_digest"] or receipt["command"]["operation"] != "put_blob":
        raise RuntimeError("blob event validate/apply parity or routing failed")
    repeated = json_command(["instance", "apply-event", "--store", str(store)], request)
    if not repeated["replayed"] or repeated["next_revision"] != 1:
        raise RuntimeError("duplicate blob event did not replay")
    expect_error(
        ["instance", "apply-event", "--store", str(store)],
        instance_event(primary, 0, "stale", app_event(release, types, "status")),
        "revision_conflict",
    )
    command = receipt["command"]
    denied_grant = make_grant(primary, "primary_objects", "production", denied_namespace)
    denied_host = {
        "version": 2,
        "instance": primary,
        "command": command["id"],
        "grant": denied_grant,
        "input": {"kind": "none"},
    }
    expect_error(
        ["instance", "execute-host", "--store", str(store)],
        denied_host,
        "capability_denied",
    )
    host = {**denied_host, "grant": primary_grant}
    stored = json_command(["instance", "execute-host", "--store", str(store)], host)
    if stored["class"] != "succeeded":
        raise RuntimeError("production immutable blob put did not succeed")
    if not json_command(["instance", "execute-host", "--store", str(store)], host)["replayed"]:
        raise RuntimeError("duplicate blob host execution did not replay")
    digest = decode_b64(stored["evidence"])
    if len(digest) != 32:
        raise RuntimeError("blob adapter did not return an exact content identity")
    object_path = primary_namespace / f"{digest.hex()}.lkjb"
    if object_path.read_bytes() != content:
        raise RuntimeError("immutable blob object does not contain exact requested bytes")

    corrupt_store = root / "corrupt-outcome-instances"
    shutil.copytree(store, corrupt_store)
    outcome_path = corrupt_store / primary / "outcomes" / f"{command['id']}.lkio"
    damaged = bytearray(outcome_path.read_bytes())
    damaged[len(damaged) // 2] ^= 1
    outcome_path.write_bytes(damaged)
    expect_error(
        ["instance", "inspect", "--store", str(corrupt_store), "--instance", primary],
        None,
        "artifact_corrupt",
    )

    resume_prediction = json_command(
        ["instance", "validate-resume", "--store", str(store)],
        resume_request(primary, 1, None, "validate_only"),
    )
    completed = json_command(
        ["instance", "resume", "--store", str(store)],
        resume_request(primary, 1, "stored-1"),
    )
    if resume_prediction["state_digest"] != completed["state_digest"] or completed["status"] != "completed":
        raise RuntimeError("blob resume validate/apply parity failed")

    repeat_put = json_command(
        ["instance", "apply-event", "--store", str(store)],
        instance_event(primary, 2, "publish-2", publish),
    )
    host.update({"command": repeat_put["command"]["id"]})
    already = json_command(["instance", "execute-host", "--store", str(store)], host)
    if already["class"] != "already_present" or decode_b64(already["evidence"]) != digest:
        raise RuntimeError("content-addressed repeat did not prove exact already-present identity")
    json_command(
        ["instance", "resume", "--store", str(store)],
        resume_request(primary, 3, "stored-2"),
    )

    secondary_grant = make_grant(secondary, "secondary_objects", "deterministic_fake", fake_namespace)
    secondary_create = {
        "version": 2,
        "mode": "commit",
        "instance": secondary,
        "initial_state": initial,
        "grants": [secondary_grant],
    }
    json_command(
        ["instance", "create", "--store", str(store), "--application", str(application_path)],
        secondary_create,
    )
    fake_put = json_command(
        ["instance", "apply-event", "--store", str(store)],
        instance_event(secondary, 0, "fake-publish", publish),
    )
    expect_error(
        ["instance", "execute-host", "--store", str(store)],
        {**host, "instance": secondary, "grant": secondary_grant},
        "capability_denied",
    )
    fake = {
        "version": 2,
        "instance": secondary,
        "command": fake_put["command"]["id"],
        "grant": secondary_grant,
        "class": "outcome_unknown",
        "evidence": b64(digest),
    }
    unknown = json_command(["instance", "fake-outcome", "--store", str(store)], fake)
    if unknown["class"] != "outcome_unknown":
        raise RuntimeError("fake blob put did not retain unknown visibility")
    inspect = json_command(
        ["instance", "resume", "--store", str(store)],
        resume_request(secondary, 1, "unknown-1"),
    )
    if inspect["command"]["operation"] != "inspect_blob":
        raise RuntimeError("unknown blob visibility did not route to exact inspection")
    fake.update({
        "command": inspect["command"]["id"],
        "class": "reconciliation_absent",
    })
    json_command(["instance", "fake-outcome", "--store", str(store)], fake)
    absent = json_command(
        ["instance", "resume", "--store", str(store)],
        resume_request(secondary, 2, "absent-1"),
    )
    if absent["status"] != "completed":
        raise RuntimeError("absent blob reconciliation did not complete retryably")
    retry = json_command(
        ["instance", "apply-event", "--store", str(store)],
        instance_event(secondary, 3, "retry-1", app_event(release, types, "retry")),
    )
    fake.update({
        "command": retry["command"]["id"],
        "class": "known_failure_before_visibility",
    })
    json_command(["instance", "fake-outcome", "--store", str(store)], fake)
    json_command(
        ["instance", "resume", "--store", str(store)],
        resume_request(secondary, 4, "failed-1"),
    )
    json_command(
        ["instance", "apply-event", "--store", str(store)],
        instance_event(secondary, 5, "cancel-1", app_event(release, types, "cancel")),
    )

    history = json_command(
        ["instance", "history", "--store", str(store), "--instance", primary, "--start", "0", "--limit", "2"],
    )
    if history["complete"] or history["next_revision"] != 2:
        raise RuntimeError("blob history inspection is not bounded")
    primary_inspection = json_command(
        ["instance", "inspect", "--store", str(store), "--instance", primary],
    )
    secondary_inspection = json_command(
        ["instance", "inspect", "--store", str(store), "--instance", secondary],
    )
    if primary_inspection["revision"] != 4 or secondary_inspection["revision"] != 6:
        raise RuntimeError("restart reconstruction produced the wrong blob publisher revisions")
    deleted = json_command(
        ["instance", "delete", "--store", str(store)],
        {"version": 2, "instance": secondary, "base_revision": 6},
    )
    if not deleted["deleted"]:
        raise RuntimeError("blob publisher tombstone was not durable")
    expect_error(
        ["instance", "create", "--store", str(store), "--application", str(application_path)],
        secondary_create,
        "workspace_exists",
    )
    if USE_RUNTIME_SESSION:
        runtime_inspection = inspect_runtime_session()
        stop_runtime_session()
    return {
        "primary_instance": primary,
        "secondary_instance": secondary,
        "primary_revision": primary_inspection["revision"],
        "secondary_revision": deleted["revision"],
        "blob_digest": digest.hex(),
        "blob_bytes": object_path.stat().st_size,
        "history_records": primary_inspection["history_records"],
        "history_bytes": primary_inspection["history_bytes"],
        "already_present": True,
        "unknown_outcome_reconciled": True,
    }


def execute():
    global state_root
    with tempfile.TemporaryDirectory(prefix="lkjscript-durable-blob-publisher-") as directory:
        root = pathlib.Path(directory).resolve()
        state_root = root / "workspace-state"
        state_root.mkdir(mode=0o700)
        start_session()
        workspace = expect(rpc({"kind": "create_workspace"}), "workspace_created")["workspace"]
        returned = [
            1,
            100, 101, 102, 103, 104, 105, 106, 107, 108,
            150, 151, 152,
            200, 201, 202, 203, 204,
            300, 301, 302,
            320, 321, 322, 323, 324, 325, 326, 327,
            340, 341, 350, 351,
            360, 361, 362, 370, 371, 372, 373, 380, 381, 382,
            500, 700, 900,
        ]
        response = expect(rpc({
            "kind": "apply_transaction",
            "data": {
                "transaction": {
                    "workspace": workspace,
                    "base_revision": 0,
                    "mode": "commit",
                    "operations": operations(),
                },
                "response": {"return_symbols": [symbol(item) for item in returned]},
            },
        }), "transaction_receipt")
        ids = {int(name.removeprefix("draft_")): node for name, node in response["returned_bindings"]}
        stop_session()

        release_request = {
            "version": 1,
            "workspace": workspace,
            "revision": response["revision"],
            "root": ids[1],
            "coordinate": "examples/durable-blob-publisher",
            "user_version": "1.0.0",
            "exports": [
                {"name": "state", "target": ids[100]},
                {"name": "correlation", "target": ids[150]},
                {"name": "event", "target": ids[200]},
                {"name": "blob_request", "target": ids[300]},
                {"name": "blob_outcome", "target": ids[320]},
                {"name": "command", "target": ids[340]},
                {"name": "outcome", "target": ids[350]},
                {"name": "completed_payload", "target": ids[360]},
                {"name": "suspended_payload", "target": ids[370]},
                {"name": "decision", "target": ids[380]},
                {"name": "transition_event", "target": ids[500]},
                {"name": "transition_resume", "target": ids[700]},
                {"name": "identity", "target": ids[900]},
            ],
            "dependencies": [],
            "imports": [],
            "tests": [{
                "name": "identity",
                "target": ids[900],
                "arguments": [bytes_value(b"payload")],
                "expected": {"kind": "value", "data": bytes_value(b"payload")},
                "policy": {"fuel": 1000, "maximum_frames": 32},
            }],
        }
        release_path = root / "publisher.lkjr"
        release_receipt = json_command(
            ["release", "build", "--state", str(state_root), "--output", str(release_path)],
            release_request,
        )
        release = release_receipt["inspection"]["release"]
        exports = export_map(release_receipt)
        types = build_types(exports)
        event_entry = app_target(release, exports["transition_event"]["target"])
        resume_entry = app_target(release, exports["transition_resume"]["target"])
        content = b"application-test-content"
        digest = bytes(range(32))
        putting = app_bytes_state(release, types, "putting", content)
        correlation = app_product(release, types, "correlation", [
            ("content", bytes_value(content)), ("digest", bytes_value(digest)),
        ])
        inspecting = app_state(release, types, "inspecting", correlation)
        application_request = {
            "version": 4,
            "root_release": release,
            "entry": event_entry,
            "profile": {"kind": "stateful", "data": {
                "resume": resume_entry,
                "state": app_target(release, types["state"]["target"]),
                "event": app_target(release, types["event"]["target"]),
                "command": app_target(release, types["command"]["target"]),
                "outcome": app_target(release, types["outcome"]["target"]),
                "decision": app_target(release, types["decision"]["target"]),
                "completed_variant": app_target(release, types["decision"]["variants"]["completed"]),
                "completed_payload": app_target(release, types["completed_payload"]["target"]),
                "completed_state_field": app_target(release, types["completed_payload"]["fields"]["state"]),
                "completed_response_field": app_target(release, types["completed_payload"]["fields"]["response"]),
                "suspended_variant": app_target(release, types["decision"]["variants"]["suspended"]),
                "suspended_payload": app_target(release, types["suspended_payload"]["target"]),
                "suspended_state_field": app_target(release, types["suspended_payload"]["fields"]["state"]),
                "suspended_response_field": app_target(release, types["suspended_payload"]["fields"]["response"]),
                "suspended_command_field": app_target(release, types["suspended_payload"]["fields"]["command"]),
                "imports": [{
                    "slot": "objects",
                    "interface": "immutable_blob",
                    "request": app_target(release, types["blob_request"]["target"]),
                    "outcome": app_target(release, types["blob_outcome"]["target"]),
                    "command_variant": app_target(release, types["command"]["variants"]["blob"]),
                    "outcome_variant": app_target(release, types["outcome"]["variants"]["blob"]),
                    "requests": [
                        {"variant": app_target(release, types["blob_request"]["variants"]["put"]), "operation": "put_blob"},
                        {"variant": app_target(release, types["blob_request"]["variants"]["inspect"]), "operation": "inspect_blob"},
                    ],
                    "outcomes": [
                        {"operation": "put_blob", "class": "succeeded", "variant": app_target(release, types["blob_outcome"]["variants"]["stored"])},
                        {"operation": "put_blob", "class": "already_present", "variant": app_target(release, types["blob_outcome"]["variants"]["already_present"])},
                        {"operation": "put_blob", "class": "known_failure_before_visibility", "variant": app_target(release, types["blob_outcome"]["variants"]["put_failed"])},
                        {"operation": "put_blob", "class": "outcome_unknown", "variant": app_target(release, types["blob_outcome"]["variants"]["put_unknown"])},
                        {"operation": "put_blob", "class": "cancelled_before_action", "variant": app_target(release, types["blob_outcome"]["variants"]["put_failed"])},
                        {"operation": "put_blob", "class": "timeout_before_action", "variant": app_target(release, types["blob_outcome"]["variants"]["put_failed"])},
                        {"operation": "put_blob", "class": "timeout_after_possible_visibility", "variant": app_target(release, types["blob_outcome"]["variants"]["put_unknown"])},
                        {"operation": "put_blob", "class": "cleanup_failure", "variant": app_target(release, types["blob_outcome"]["variants"]["put_unknown"])},
                        {"operation": "inspect_blob", "class": "reconciliation_present", "variant": app_target(release, types["blob_outcome"]["variants"]["inspect_present"])},
                        {"operation": "inspect_blob", "class": "reconciliation_absent", "variant": app_target(release, types["blob_outcome"]["variants"]["inspect_absent"])},
                        {"operation": "inspect_blob", "class": "reconciliation_indeterminate", "variant": app_target(release, types["blob_outcome"]["variants"]["inspect_indeterminate"])},
                    ],
                }],
            }},
            "policy": {"fuel": 100000, "maximum_frames": 128},
            "tests": [
                {
                    "name": "publish_suspends_on_typed_put",
                    "target": event_entry,
                    "arguments": [
                        app_bytes_state(release, types, "idle", b""),
                        app_event(release, types, "publish", content),
                    ],
                    "expected": {"kind": "value", "data": app_suspended(
                        release,
                        types,
                        putting,
                        b"blob_put_requested",
                        app_command(release, types, "put", content),
                    )},
                    "policy": {"fuel": 100000, "maximum_frames": 128},
                },
                {
                    "name": "unknown_put_suspends_on_typed_inspection",
                    "target": resume_entry,
                    "arguments": [
                        putting,
                        app_outcome(release, types, "put_unknown", digest),
                    ],
                    "expected": {"kind": "value", "data": app_suspended(
                        release,
                        types,
                        inspecting,
                        b"blob_visibility_unknown",
                        app_command(release, types, "inspect", digest),
                    )},
                    "policy": {"fuel": 100000, "maximum_frames": 128},
                },
            ],
        }
        application_path = root / "publisher.lkja"
        json_command(
            ["app", "build", "--release", str(release_path), "--output", str(application_path)],
            application_request,
        )
        shutil.rmtree(state_root)
        release_path.unlink()
        proof = operate(root, application_path, release, types)
        return {
            "contract_versions": {"workspace": 10, "release": 1, "application": 4, "instance": 2},
            "source_workspace_deleted": not state_root.exists(),
            "source_release_deleted": not release_path.exists(),
            "application_bytes": application_path.stat().st_size,
            "proof": proof,
            "measurements": {
                "topology": "foreground_session" if USE_RUNTIME_SESSION else "one_shot",
                "processes": sum(1 for item in measurements if item["process_started"])
                + 1
                + (1 if USE_RUNTIME_SESSION else 0),
                "engine_opens": 1,
                "authoring_rpc_calls": request_id,
                "runtime_rpc_calls": runtime_request_id,
                "action_bytes": sum(item["input_bytes"] for item in measurements),
                "observation_bytes": sum(item["output_bytes"] + item["diagnostic_bytes"] for item in measurements),
                "boundary_elapsed_nanoseconds": sum(item["elapsed_nanoseconds"] for item in measurements),
                "runtime_inspection": runtime_inspection,
                "provider_tokens": None,
            },
        }


if __name__ == "__main__":
    print(json.dumps(execute(), sort_keys=True, separators=(",", ":")))
