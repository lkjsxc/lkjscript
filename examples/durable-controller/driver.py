#!/usr/bin/env python3
"""Author and operate the durable release controller through production CLI boundaries."""

import base64
import json
import os
import pathlib
import shutil
import subprocess
import sys
import tempfile
import time


CLI = pathlib.Path(sys.argv[1]).resolve()
request_id = 0
session = None
state = None
measurements = []


def b64(value):
    return base64.urlsafe_b64encode(bytes(value)).rstrip(b"=").decode("ascii")


def bytes_value(value):
    return {"kind": "bytes", "data": b64(value)}


def i64_value(value):
    return {"kind": "i64", "data": value}


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


def arm(variant, body, payload=None):
    value = {"variant": local(variant), "body": body}
    if payload is not None:
        value["payload_symbol"] = symbol(payload)
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


def construct_state(number, variant, payload):
    return expression(number, "construct_variant", {"variant": local(variant), "payload": payload})


def command_decision(start, variant, payload, response, command):
    operations = [
        construct_state(start, variant, payload),
        expression(start + 1, "const_bytes", b64(response)),
    ]
    if command == 0:
        operations.append(call(start + 4, 400, [result(start), result(start + 1)]))
    else:
        operations.extend([
            expression(start + 2, "construct_variant", {
                "variant": local({1: 301, 2: 302, 3: 303}[command]), "payload": payload,
            }),
            expression(start + 3, "construct_variant", {
                "variant": local(341), "payload": result(start + 2),
            }),
            call(start + 4, 410, [result(start), result(start + 1), result(start + 3)]),
        ])
    return yielding(operations, result(start + 4))


def unchanged_decision(start, state_value, response):
    operations = [expression(start, "const_bytes", b64(response)), call(start + 1, 400, [state_value, result(start)])]
    return yielding(operations, result(start + 1))


STATE_VARIANTS = [101, 102, 103, 104, 105, 106, 107, 108, 109]


def state_match_arms(base, special):
    arms = []
    for offset, variant in enumerate(STATE_VARIANTS):
        payload = base + offset * 20
        body = special(variant, payload, base + offset * 20 + 1)
        arms.append(arm(variant, body, payload))
    return arms


def event_function():
    request_body = command_decision(520, 102, block_argument(511), b"activation_requested", 0)

    def begin_case(variant, payload, start):
        if variant == 102:
            return command_decision(start, 103, block_argument(payload), b"validation_started", 1)
        return unchanged_decision(start, parameter(501), b"validation_not_allowed")

    begin_body = yielding([
        expression(530, "match_sum", {
            "scrutinee": parameter(501),
            "result": nominal(380),
            "arms": state_match_arms(1000, begin_case),
        })
    ], result(530))

    def retry_case(variant, payload, start):
        if variant == 106:
            return command_decision(start, 103, block_argument(payload), b"retry_validation", 1)
        if variant == 109:
            return command_decision(start, 109, block_argument(payload), b"retry_reconciliation", 3)
        return unchanged_decision(start, parameter(501), b"retry_not_allowed")

    retry_body = yielding([
        expression(560, "match_sum", {
            "scrutinee": parameter(501),
            "result": nominal(380),
            "arms": state_match_arms(1300, retry_case),
        })
    ], result(560))
    cancel_body = command_decision(570, 108, result(571), b"cancelled", 0)
    # command_decision needs the empty payload before constructing the state.
    cancel_body = yielding([
        expression(569, "const_bytes", b64(b"")),
        construct_state(570, 108, result(569)),
        expression(571, "const_bytes", b64(b"cancelled")),
        call(572, 400, [result(570), result(571)]),
    ], result(572))
    status_body = unchanged_decision(580, parameter(501), b"status")
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
                arm(201, request_body, 511),
                arm(202, begin_body),
                arm(203, retry_body),
                arm(204, cancel_body),
                arm(205, status_body),
            ],
        })],
        result(510),
    )


def resume_function():
    outcomes = [
        (321, 104, b"validation_succeeded", 2),
        (322, 106, b"validation_failed", 0),
        (323, 105, b"activation_succeeded", 0),
        (324, 106, b"activation_failed_before_visibility", 0),
        (325, 109, b"activation_outcome_unknown", 3),
        (326, 105, b"reconciliation_found_active", 0),
        (327, 106, b"reconciliation_found_absent", 0),
        (328, 109, b"reconciliation_indeterminate", 0),
    ]
    outcome_arms = []
    for offset, (outcome_variant, state_variant, response, command) in enumerate(outcomes):
        payload = 620 + offset
        outcome_arms.append(arm(
            outcome_variant,
            command_decision(3000 + offset * 20, state_variant, block_argument(payload), response, command),
            payload,
        ))
    activation_body = yielding([
        expression(615, "match_sum", {
            "scrutinee": block_argument(611),
            "result": nominal(380),
            "arms": outcome_arms,
        }),
    ], result(615))

    return function(
        600,
        "transition_resume",
        [
            {"symbol": 601, "name": "state", "ty": nominal(100)},
            {"symbol": 602, "name": "outcome", "ty": nominal(350)},
        ],
        nominal(380),
        [expression(610, "match_sum", {
            "scrutinee": parameter(602),
            "result": nominal(380),
            "arms": [arm(351, activation_body, 611)],
        })],
        result(610),
    )


def operations():
    return [
        {"kind": "create_package", "data": {"symbol": symbol(1), "name": "durable-controller"}},
        {"kind": "create_module", "data": {"symbol": symbol(2), "package": local(1), "name": "main"}},
        {"kind": "create_sum_type", "data": {
            "symbol": symbol(100), "module": local(2), "name": "ControllerState",
            "variants": [
                {"symbol": symbol(101), "name": "idle", "payload": "bytes"},
                {"symbol": symbol(102), "name": "requested", "payload": "bytes"},
                {"symbol": symbol(103), "name": "validating", "payload": "bytes"},
                {"symbol": symbol(104), "name": "activating", "payload": "bytes"},
                {"symbol": symbol(105), "name": "active", "payload": "bytes"},
                {"symbol": symbol(106), "name": "retryable_failure", "payload": "bytes"},
                {"symbol": symbol(107), "name": "terminal_failure", "payload": "bytes"},
                {"symbol": symbol(108), "name": "cancelled", "payload": "bytes"},
                {"symbol": symbol(109), "name": "outcome_unknown", "payload": "bytes"},
            ],
        }},
        {"kind": "create_sum_type", "data": {
            "symbol": symbol(200), "module": local(2), "name": "ControllerEvent",
            "variants": [
                {"symbol": symbol(201), "name": "request_activation", "payload": "bytes"},
                {"symbol": symbol(202), "name": "begin_validation"},
                {"symbol": symbol(203), "name": "retry_requested"},
                {"symbol": symbol(204), "name": "cancellation_requested"},
                {"symbol": symbol(205), "name": "status_inspection"},
            ],
        }},
        {"kind": "create_sum_type", "data": {
            "symbol": symbol(300), "module": local(2), "name": "ActivationRequest",
            "variants": [
                {"symbol": symbol(301), "name": "validate", "payload": "bytes"},
                {"symbol": symbol(302), "name": "activate", "payload": "bytes"},
                {"symbol": symbol(303), "name": "reconcile", "payload": "bytes"},
            ],
        }},
        {"kind": "create_sum_type", "data": {
            "symbol": symbol(320), "module": local(2), "name": "ActivationOutcome",
            "variants": [
                {"symbol": symbol(321), "name": "validation_succeeded", "payload": "bytes"},
                {"symbol": symbol(322), "name": "validation_failed", "payload": "bytes"},
                {"symbol": symbol(323), "name": "activation_succeeded", "payload": "bytes"},
                {"symbol": symbol(324), "name": "activation_failed", "payload": "bytes"},
                {"symbol": symbol(325), "name": "activation_unknown", "payload": "bytes"},
                {"symbol": symbol(326), "name": "reconciliation_present", "payload": "bytes"},
                {"symbol": symbol(327), "name": "reconciliation_absent", "payload": "bytes"},
                {"symbol": symbol(328), "name": "reconciliation_indeterminate", "payload": "bytes"},
            ],
        }},
        {"kind": "create_sum_type", "data": {
            "symbol": symbol(340), "module": local(2), "name": "ControllerCommand",
            "variants": [
                {"symbol": symbol(341), "name": "activation", "payload": nominal(300)},
            ],
        }},
        {"kind": "create_sum_type", "data": {
            "symbol": symbol(350), "module": local(2), "name": "ControllerOutcome",
            "variants": [
                {"symbol": symbol(351), "name": "activation", "payload": nominal(320)},
            ],
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
        event_function(),
        resume_function(),
        function(
            700, "identity",
            [{"symbol": 701, "name": "input", "ty": "bytes"}],
            "bytes", [], parameter(701),
        ),
        {"kind": "set_entry_function", "data": {"package": local(1), "function": local(500)}},
    ]


def run_process(arguments, value=None, expected=0):
    encoded = b"" if value is None else json.dumps(value, separators=(",", ":")).encode()
    started = time.monotonic_ns()
    completed = subprocess.run(
        [str(CLI), *arguments], input=encoded, stdout=subprocess.PIPE, stderr=subprocess.PIPE,
        check=False, timeout=60,
    )
    measurements.append({
        "command": " ".join(arguments[:2]), "input_bytes": len(encoded),
        "output_bytes": len(completed.stdout), "diagnostic_bytes": len(completed.stderr),
        "elapsed_nanoseconds": time.monotonic_ns() - started, "exit": completed.returncode,
    })
    if expected is not None and completed.returncode != expected:
        raise RuntimeError(
            f"command {arguments} returned {completed.returncode}, expected {expected}: "
            f"stdout={completed.stdout!r} stderr={completed.stderr!r}"
        )
    return completed


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
    result_value = json.loads(completed.stdout)
    if result_value.get("error", {}).get("code") != code:
        raise RuntimeError(f"expected {code}, received {result_value}")
    return result_value


def start_session():
    global session
    session = subprocess.Popen(
        [str(CLI), "--state", str(state), "session"],
        stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE,
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
        "command": "authoring session", "input_bytes": len(encoded), "output_bytes": len(response),
        "diagnostic_bytes": 0, "elapsed_nanoseconds": time.monotonic_ns() - started, "exit": 0,
    })
    decoded = json.loads(response)
    if decoded.get("request_id") != request_id:
        raise RuntimeError("authoring response correlation failed")
    return decoded["response"]


def expect(response, kind):
    if response.get("kind") != kind:
        raise RuntimeError(f"expected {kind}, received {response}")
    return response.get("data")


def workspace_value(ids, ty, variant, payload=None):
    data = {"ty": ids[ty], "variant": ids[variant]}
    if payload is not None:
        data["payload"] = bytes_value(payload)
    return {"kind": "sum", "data": data}


def app_target(release, item):
    return {"release": release, "item": item}


def app_value(release, types, ty_name, variant_name, payload=None):
    ty = types[ty_name]
    variant = ty["variants"][variant_name]
    data = {"ty": app_target(release, ty["target"]), "variant": app_target(release, variant)}
    if payload is not None:
        data["payload"] = bytes_value(payload)
    return {"kind": "sum", "data": data}


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


def app_decision(release, types, state_variant, state_payload, response, command, target):
    state = app_value(release, types, "state", state_variant, state_payload)
    if command == 0:
        payload = app_product(release, types, "completed_payload", [
            ("state", state), ("response", bytes_value(response)),
        ])
        return app_sum(release, types, "decision", "completed", payload)
    request_variant = {1: "validate", 2: "activate", 3: "reconcile"}[command]
    request = app_value(release, types, "activation_request", request_variant, target)
    application_command = app_sum(release, types, "command", "activation", request)
    payload = app_product(release, types, "suspended_payload", [
        ("state", state),
        ("response", bytes_value(response)),
        ("command", application_command),
    ])
    return app_sum(release, types, "decision", "suspended", payload)


def app_outcome(release, types, variant, evidence):
    interface_outcome = app_value(release, types, "activation_outcome", variant, evidence)
    return app_sum(release, types, "outcome", "activation", interface_outcome)


def export_map(receipt):
    return {item["name"]: item for item in receipt["inspection"]["exports"]}


def build_types(exports):
    types = {}
    for name in [
        "state", "event", "activation_request", "activation_outcome", "command", "outcome",
        "completed_payload", "suspended_payload", "decision",
    ]:
        exported = exports[name]
        data = exported["signature"]["data"]
        item = {"target": exported["target"]}
        if "variants" in data:
            item["variants"] = {entry["name"]: entry["target"] for entry in data["variants"]}
        if "fields" in data:
            item["fields"] = {entry["name"]: entry["target"] for entry in data["fields"]}
        types[name] = item
    return types


def instance_event(instance, revision, key, event, mode="commit"):
    value = {"version": 2, "mode": mode, "instance": instance, "base_revision": revision, "event": event}
    if key is not None:
        value["event_key"] = key
    return value


def resume_request(instance, revision, key, mode="commit"):
    value = {"version": 2, "mode": mode, "instance": instance, "base_revision": revision}
    if key is not None:
        value["event_key"] = key
    return value


def operate(root, controller_path, payload_path, release, types):
    store = root / "instances"
    store.mkdir(mode=0o700)
    slots = root / "slots"
    slots.mkdir(mode=0o700)
    instance = "11111111111111111111111111111111"
    other = "22222222222222222222222222222222"
    slot = slots / "active.lkja"
    grant = {
        "version": 2,
        "name": "primary",
        "instance": instance,
        "slot": "activation",
        "interface": "application_activation",
        "adapter": "production",
        "descriptor": {
            "kind": "application_activation",
            "data": {"source_directory": str(root), "activation_slot": str(slot)},
        },
    }
    idle = app_value(release, types, "state", "idle", b"")
    create = {
        "version": 2, "mode": "validate_only", "instance": instance,
        "initial_state": idle, "grants": [grant],
    }
    validated = json_command(["instance", "create", "--store", str(store), "--application", str(controller_path)], create)
    if validated["published"]:
        raise RuntimeError("validate-only instance creation published")
    create["mode"] = "commit"
    committed = json_command(["instance", "create", "--store", str(store), "--application", str(controller_path)], create)
    if not committed["published"]:
        raise RuntimeError("instance creation did not publish")

    digest = bytes.fromhex(json_command(["app", "inspect", "--artifact", str(payload_path)])["digest"])
    request_event_value = app_value(release, types, "event", "request_activation", digest)
    validate_request = instance_event(instance, 0, None, request_event_value, "validate_only")
    predicted = json_command(["instance", "validate-event", "--store", str(store)], validate_request)
    committed_request = instance_event(instance, 0, "request-1", request_event_value)
    receipt = json_command(["instance", "apply-event", "--store", str(store)], committed_request)
    if predicted["state_digest"] != receipt["state_digest"] or receipt["next_revision"] != 1:
        raise RuntimeError("event validate/apply parity failed")
    replay = json_command(["instance", "apply-event", "--store", str(store)], committed_request)
    if not replay["replayed"] or replay["next_revision"] != 1:
        raise RuntimeError("duplicate event did not replay the retained receipt")
    expect_error(
        ["instance", "apply-event", "--store", str(store)],
        instance_event(instance, 0, "stale-1", app_value(release, types, "event", "status_inspection")),
        "revision_conflict",
    )

    begin = app_value(release, types, "event", "begin_validation")
    receipt = json_command(
        ["instance", "apply-event", "--store", str(store)],
        instance_event(instance, 1, "begin-1", begin),
    )
    command = receipt["command"]
    if command["operation"] != "validate_application":
        raise RuntimeError("controller did not suspend for exact validation")
    denied = {
        **grant,
        "descriptor": {
            "kind": "application_activation",
            "data": {
                "source_directory": str(root),
                "activation_slot": str(slots / "other.lkja"),
            },
        },
    }
    host = {
        "version": 2,
        "instance": instance,
        "command": command["id"],
        "grant": denied,
        "input": {"kind": "application_source", "data": {"path": str(payload_path)}},
    }
    expect_error(["instance", "execute-host", "--store", str(store)], host, "capability_denied")
    host["grant"] = grant
    expect_error(
        ["instance", "fake-outcome", "--store", str(store)],
        {
            "version": 2, "instance": instance, "command": command["id"], "grant": grant,
            "class": "succeeded", "evidence": b64(digest),
        },
        "capability_denied",
    )
    validation = json_command(["instance", "execute-host", "--store", str(store)], host)
    if validation["class"] != "succeeded":
        raise RuntimeError("exact payload validation did not succeed")
    if not json_command(["instance", "execute-host", "--store", str(store)], host)["replayed"]:
        raise RuntimeError("host result replay was not exact")
    corrupt_outcome_store = root / "corrupt-outcome-instances"
    shutil.copytree(store, corrupt_outcome_store)
    outcome_path = corrupt_outcome_store / instance / "outcomes" / f"{command['id']}.lkio"
    outcome_bytes = bytearray(outcome_path.read_bytes())
    outcome_bytes[len(outcome_bytes) // 2] ^= 1
    outcome_path.write_bytes(outcome_bytes)
    expect_error(
        ["instance", "inspect", "--store", str(corrupt_outcome_store), "--instance", instance],
        None,
        "artifact_corrupt",
    )
    predicted = json_command(
        ["instance", "validate-resume", "--store", str(store)],
        resume_request(instance, 2, None, "validate_only"),
    )
    receipt = json_command(
        ["instance", "resume", "--store", str(store)],
        resume_request(instance, 2, "validation-1"),
    )
    if predicted["state_digest"] != receipt["state_digest"] or receipt["command"]["operation"] != "activate_application":
        raise RuntimeError("validation resume parity or activation suspension failed")
    replayed_resume = json_command(
        ["instance", "resume", "--store", str(store)],
        resume_request(instance, 2, "validation-1"),
    )
    if not replayed_resume["replayed"] or replayed_resume["next_revision"] != 3:
        raise RuntimeError("duplicate resume did not replay the retained receipt")
    activation_host = {
        "version": 2,
        "instance": instance,
        "command": receipt["command"]["id"],
        "grant": grant,
        "input": {"kind": "application_source", "data": {"path": str(payload_path)}},
    }
    activation = json_command(["instance", "execute-host", "--store", str(store)], activation_host)
    if activation["class"] != "succeeded" or slot.read_bytes() != payload_path.read_bytes():
        raise RuntimeError("production activation did not make the exact application visible")
    receipt = json_command(
        ["instance", "resume", "--store", str(store)],
        resume_request(instance, 3, "activation-1"),
    )
    if receipt["status"] != "completed" or receipt["next_revision"] != 4:
        raise RuntimeError("activation success did not complete the semantic transition")

    # A second instance has a distinct grant domain and cannot consume the first command or slot.
    other_slot = slots / "other-active.lkja"
    other_grant = {
        **grant,
        "instance": other,
        "name": "secondary",
        "adapter": "deterministic_fake",
        "descriptor": {
            "kind": "application_activation",
            "data": {"source_directory": str(root), "activation_slot": str(other_slot)},
        },
    }
    other_create = {"version": 2, "mode": "commit", "instance": other, "initial_state": idle, "grants": [other_grant]}
    json_command(["instance", "create", "--store", str(store), "--application", str(controller_path)], other_create)
    expect_error(
        ["instance", "execute-host", "--store", str(store)],
        {**activation_host, "instance": other, "grant": other_grant},
        "protocol_malformed",
    )

    # The deterministic fake host exercises unknown visibility, reconciliation, retry, and
    # cancellation without granting the fake instance production filesystem authority.
    wrong_digest = bytes([0xA5]) * 32
    wrong_event = app_value(release, types, "event", "request_activation", wrong_digest)
    json_command(["instance", "apply-event", "--store", str(store)], instance_event(other, 0, "wrong-request", wrong_event))
    r2 = json_command(["instance", "apply-event", "--store", str(store)], instance_event(other, 1, "wrong-begin", begin))
    fake = {
        "version": 2, "instance": other, "command": r2["command"]["id"], "grant": other_grant,
        "class": "succeeded", "evidence": b64(wrong_digest),
    }
    json_command(["instance", "fake-outcome", "--store", str(store)], fake)
    activation_request = json_command(
        ["instance", "resume", "--store", str(store)],
        resume_request(other, 2, "fake-validation"),
    )
    fake.update({
        "command": activation_request["command"]["id"],
        "class": "outcome_unknown",
        "evidence": b64(wrong_digest),
    })
    unknown = json_command(["instance", "fake-outcome", "--store", str(store)], fake)
    if unknown["class"] != "outcome_unknown":
        raise RuntimeError("fake activation did not retain an unknown outcome")
    reconcile_request = json_command(
        ["instance", "resume", "--store", str(store)],
        resume_request(other, 3, "fake-unknown"),
    )
    if reconcile_request["command"]["operation"] != "reconcile_activation":
        raise RuntimeError("unknown outcome did not suspend for reconciliation")
    fake.update({
        "command": reconcile_request["command"]["id"],
        "class": "reconciliation_absent",
    })
    json_command(["instance", "fake-outcome", "--store", str(store)], fake)
    json_command(
        ["instance", "resume", "--store", str(store)],
        resume_request(other, 4, "fake-reconciliation"),
    )
    retry = app_value(release, types, "event", "retry_requested")
    retry_receipt = json_command(["instance", "apply-event", "--store", str(store)], instance_event(other, 5, "retry-1", retry))
    fake.update({
        "command": retry_receipt["command"]["id"],
        "class": "known_failure_before_visibility",
    })
    json_command(["instance", "fake-outcome", "--store", str(store)], fake)
    json_command(["instance", "resume", "--store", str(store)], resume_request(other, 6, "retry-result"))
    cancel = app_value(release, types, "event", "cancellation_requested")
    json_command(["instance", "apply-event", "--store", str(store)], instance_event(other, 7, "cancel-1", cancel))

    history = json_command(["instance", "history", "--store", str(store), "--instance", instance, "--start", "0", "--limit", "2"])
    if history["complete"] or history["next_revision"] != 2:
        raise RuntimeError("history pagination is not bounded")
    inspection = json_command(["instance", "inspect", "--store", str(store), "--instance", instance])
    if inspection["revision"] != 4 or "pending_command" in inspection:
        raise RuntimeError("restart inspection did not reconstruct the active head")

    # Hostile retained bytes reject from an isolated copy without damaging the proof instance.
    corrupt_store = root / "corrupt-instances"
    shutil.copytree(store, corrupt_store)
    record = sorted((corrupt_store / instance / "records").iterdir())[-1]
    damaged = bytearray(record.read_bytes())
    damaged[len(damaged) // 2] ^= 1
    record.write_bytes(damaged)
    expect_error(["instance", "inspect", "--store", str(corrupt_store), "--instance", instance], None, "artifact_corrupt")

    deleted = json_command(
        ["instance", "delete", "--store", str(store)],
        {"version": 2, "instance": other, "base_revision": 8},
    )
    if not deleted["deleted"]:
        raise RuntimeError("instance tombstone was not durable")
    expect_error(
        ["instance", "create", "--store", str(store), "--application", str(controller_path)],
        other_create,
        "workspace_exists",
    )
    return {
        "primary_instance": instance,
        "secondary_instance": other,
        "primary_revision": inspection["revision"],
        "primary_state_digest": inspection["state_digest"],
        "history_records": inspection["history_records"],
        "history_bytes": inspection["history_bytes"],
        "secondary_revision": deleted["revision"],
        "unknown_outcome_reconciled": True,
        "activated_application_digest": digest.hex(),
        "slot_bytes": slot.stat().st_size,
    }


def execute():
    global state
    with tempfile.TemporaryDirectory(prefix="lkjscript-durable-controller-") as directory:
        root = pathlib.Path(directory).resolve()
        state = root / "workspace-state"
        state.mkdir(mode=0o700)
        start_session()
        workspace = expect(rpc({"kind": "create_workspace"}), "workspace_created")["workspace"]
        returned = [
            1,
            100, *STATE_VARIANTS,
            200, 201, 202, 203, 204, 205,
            300, 301, 302, 303,
            320, 321, 322, 323, 324, 325, 326, 327, 328,
            340, 341, 350, 351,
            360, 361, 362, 370, 371, 372, 373, 380, 381, 382,
            500, 600, 700,
        ]
        response = expect(rpc({
            "kind": "apply_transaction",
            "data": {
                "transaction": {"workspace": workspace, "base_revision": 0, "mode": "commit", "operations": operations()},
                "response": {"return_symbols": [symbol(item) for item in returned]},
            },
        }), "transaction_receipt")
        ids = {int(name.removeprefix("draft_")): node for name, node in response["returned_bindings"]}
        stop_session()

        target = bytes(range(32))
        release_request = {
            "version": 1, "workspace": workspace, "revision": response["revision"], "root": ids[1],
            "coordinate": "examples/durable-controller", "user_version": "1.0.0",
            "exports": [
                {"name": "state", "target": ids[100]},
                {"name": "event", "target": ids[200]},
                {"name": "activation_request", "target": ids[300]},
                {"name": "activation_outcome", "target": ids[320]},
                {"name": "command", "target": ids[340]},
                {"name": "outcome", "target": ids[350]},
                {"name": "completed_payload", "target": ids[360]},
                {"name": "suspended_payload", "target": ids[370]},
                {"name": "decision", "target": ids[380]},
                {"name": "transition_event", "target": ids[500]},
                {"name": "transition_resume", "target": ids[600]},
                {"name": "identity", "target": ids[700]},
            ],
            "dependencies": [], "imports": [],
            "tests": [{
                "name": "identity", "target": ids[700], "arguments": [bytes_value(b"payload")],
                "expected": {"kind": "value", "data": bytes_value(b"payload")},
                "policy": {"fuel": 1000, "maximum_frames": 32},
            }],
        }
        release_path = root / "controller.lkjr"
        release_receipt = json_command([
            "release", "build", "--state", str(state), "--output", str(release_path),
        ], release_request)
        release = release_receipt["inspection"]["release"]
        exports = export_map(release_receipt)
        types = build_types(exports)
        event_entry = app_target(release, exports["transition_event"]["target"])
        resume_entry = app_target(release, exports["transition_resume"]["target"])
        target_bytes = bytes(range(32))
        controller_request = {
            "version": 4, "root_release": release, "entry": event_entry,
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
                    "slot": "activation",
                    "interface": "application_activation",
                    "request": app_target(release, types["activation_request"]["target"]),
                    "outcome": app_target(release, types["activation_outcome"]["target"]),
                    "command_variant": app_target(release, types["command"]["variants"]["activation"]),
                    "outcome_variant": app_target(release, types["outcome"]["variants"]["activation"]),
                    "requests": [
                        {"variant": app_target(release, types["activation_request"]["variants"]["validate"]), "operation": "validate_application"},
                        {"variant": app_target(release, types["activation_request"]["variants"]["activate"]), "operation": "activate_application"},
                        {"variant": app_target(release, types["activation_request"]["variants"]["reconcile"]), "operation": "reconcile_activation"},
                    ],
                    "outcomes": [
                        {"operation": "validate_application", "class": "succeeded", "variant": app_target(release, types["activation_outcome"]["variants"]["validation_succeeded"])},
                        {"operation": "validate_application", "class": "known_failure_before_visibility", "variant": app_target(release, types["activation_outcome"]["variants"]["validation_failed"])},
                        {"operation": "validate_application", "class": "cancelled_before_action", "variant": app_target(release, types["activation_outcome"]["variants"]["validation_failed"])},
                        {"operation": "validate_application", "class": "timeout_before_action", "variant": app_target(release, types["activation_outcome"]["variants"]["validation_failed"])},
                        {"operation": "activate_application", "class": "succeeded", "variant": app_target(release, types["activation_outcome"]["variants"]["activation_succeeded"])},
                        {"operation": "activate_application", "class": "known_failure_before_visibility", "variant": app_target(release, types["activation_outcome"]["variants"]["activation_failed"])},
                        {"operation": "activate_application", "class": "outcome_unknown", "variant": app_target(release, types["activation_outcome"]["variants"]["activation_unknown"])},
                        {"operation": "activate_application", "class": "cancelled_before_action", "variant": app_target(release, types["activation_outcome"]["variants"]["activation_failed"])},
                        {"operation": "activate_application", "class": "timeout_before_action", "variant": app_target(release, types["activation_outcome"]["variants"]["activation_failed"])},
                        {"operation": "activate_application", "class": "timeout_after_possible_visibility", "variant": app_target(release, types["activation_outcome"]["variants"]["activation_unknown"])},
                        {"operation": "activate_application", "class": "cleanup_failure", "variant": app_target(release, types["activation_outcome"]["variants"]["activation_unknown"])},
                        {"operation": "reconcile_activation", "class": "reconciliation_present", "variant": app_target(release, types["activation_outcome"]["variants"]["reconciliation_present"])},
                        {"operation": "reconcile_activation", "class": "reconciliation_absent", "variant": app_target(release, types["activation_outcome"]["variants"]["reconciliation_absent"])},
                        {"operation": "reconcile_activation", "class": "reconciliation_indeterminate", "variant": app_target(release, types["activation_outcome"]["variants"]["reconciliation_indeterminate"])},
                    ],
                }],
            }},
            "policy": {"fuel": 100000, "maximum_frames": 128},
            "tests": [
                {
                    "name": "request_activation", "target": event_entry,
                    "arguments": [app_value(release, types, "state", "idle", b""), app_value(release, types, "event", "request_activation", target_bytes)],
                    "expected": {"kind": "value", "data": app_decision(release, types, "requested", target_bytes, b"activation_requested", 0, b"")},
                    "policy": {"fuel": 100000, "maximum_frames": 128},
                },
                {
                    "name": "validation_success", "target": resume_entry,
                    "arguments": [
                        app_value(release, types, "state", "validating", target_bytes),
                        app_outcome(release, types, "validation_succeeded", target_bytes),
                    ],
                    "expected": {"kind": "value", "data": app_decision(release, types, "activating", target_bytes, b"validation_succeeded", 2, target_bytes)},
                    "policy": {"fuel": 100000, "maximum_frames": 128},
                },
            ],
        }
        controller_path = root / "controller.lkja"
        json_command(["app", "build", "--release", str(release_path), "--output", str(controller_path)], controller_request)
        identity = app_target(release, exports["identity"]["target"])
        payload_request = {
            "version": 4, "root_release": release, "entry": identity,
            "profile": {"kind": "bytes_stream"},
            "policy": {"fuel": 10000, "maximum_frames": 32},
            "tests": [{
                "name": "payload_identity", "target": identity, "arguments": [bytes_value(b"payload")],
                "expected": {"kind": "value", "data": bytes_value(b"payload")},
                "policy": {"fuel": 10000, "maximum_frames": 32},
            }],
        }
        payload_path = root / "payload.lkja"
        json_command(["app", "build", "--release", str(release_path), "--output", str(payload_path)], payload_request)
        shutil.rmtree(state)
        release_path.unlink()
        proof = operate(root, controller_path, payload_path, release, types)
        return {
            "contract_versions": {"workspace": 10, "release": 1, "application": 4, "instance": 2},
            "source_workspace_deleted": not state.exists(),
            "source_release_deleted": not release_path.exists(),
            "controller_application_bytes": controller_path.stat().st_size,
            "payload_application_bytes": payload_path.stat().st_size,
            "proof": proof,
            "measurements": {
                "processes": sum(1 for item in measurements if item["command"] != "authoring session") + 1,
                "engine_opens": 1,
                "authoring_rpc_calls": request_id,
                "action_bytes": sum(item["input_bytes"] for item in measurements),
                "observation_bytes": sum(item["output_bytes"] + item["diagnostic_bytes"] for item in measurements),
                "boundary_elapsed_nanoseconds": sum(item["elapsed_nanoseconds"] for item in measurements),
                "provider_tokens": None,
            },
        }


if __name__ == "__main__":
    print(json.dumps(execute(), sort_keys=True, separators=(",", ":")))
