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
        expression(start + 2, "const_i64", command),
    ]
    target = payload
    if command == 0:
        operations.append(expression(start + 3, "const_bytes", b64(b"")))
        target = result(start + 3)
    operations.append(call(start + 4, 400, [result(start), result(start + 1), result(start + 2), target]))
    return yielding(operations, result(start + 4))


def unchanged_decision(start, state_value, response):
    operations = [expression(start, "const_bytes", b64(response)), call(start + 1, 410, [state_value, result(start)])]
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
            "result": nominal(300),
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
            "result": nominal(300),
            "arms": state_match_arms(1300, retry_case),
        })
    ], result(560))
    cancel_body = command_decision(570, 108, result(571), b"cancelled", 0)
    # command_decision needs the empty payload before constructing the state.
    cancel_body = yielding([
        expression(569, "const_bytes", b64(b"")),
        construct_state(570, 108, result(569)),
        expression(571, "const_bytes", b64(b"cancelled")),
        call(572, 410, [result(570), result(571)]),
    ], result(572))
    status_body = unchanged_decision(580, parameter(501), b"status")
    return function(
        500,
        "transition_event",
        [
            {"symbol": 501, "name": "state", "ty": nominal(100)},
            {"symbol": 502, "name": "event", "ty": nominal(200)},
        ],
        nominal(300),
        [expression(510, "match_sum", {
            "scrutinee": parameter(502),
            "result": nominal(300),
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


def classify_validation(payload):
    success = command_decision(2100, 104, block_argument(payload), b"validation_succeeded", 2)
    failure = command_decision(2120, 106, block_argument(payload), b"validation_failed", 0)
    terminal = command_decision(2140, 107, block_argument(payload), b"validation_outcome_invalid", 0)
    return yielding([
        expression(2090, "const_i64", 2),
        expression(2091, "lt_i64", {"lhs": parameter(602), "rhs": result(2090)}),
        expression(2092, "if", {
            "condition": result(2091),
            "result": nominal(300),
            "then_body": success,
            "else_body": yielding([
                expression(2093, "const_i64", 3),
                expression(2094, "lt_i64", {"lhs": parameter(602), "rhs": result(2093)}),
                expression(2095, "if", {
                    "condition": result(2094),
                    "result": nominal(300),
                    "then_body": failure,
                    "else_body": terminal,
                }),
            ], result(2095)),
        }),
    ], result(2092))


def classify_activation(payload):
    success = command_decision(2200, 105, block_argument(payload), b"activation_succeeded", 0)
    failure = command_decision(2220, 106, block_argument(payload), b"activation_failed_before_visibility", 0)
    unknown = command_decision(2240, 109, block_argument(payload), b"activation_outcome_unknown", 3)
    return yielding([
        expression(2190, "const_i64", 2),
        expression(2191, "lt_i64", {"lhs": parameter(602), "rhs": result(2190)}),
        expression(2192, "if", {
            "condition": result(2191), "result": nominal(300), "then_body": success,
            "else_body": yielding([
                expression(2193, "const_i64", 3),
                expression(2194, "lt_i64", {"lhs": parameter(602), "rhs": result(2193)}),
                expression(2195, "if", {
                    "condition": result(2194), "result": nominal(300),
                    "then_body": failure, "else_body": unknown,
                }),
            ], result(2195)),
        }),
    ], result(2192))


def classify_reconciliation(payload):
    present = command_decision(2300, 105, block_argument(payload), b"reconciliation_found_active", 0)
    absent = command_decision(2320, 106, block_argument(payload), b"reconciliation_found_absent", 0)
    indeterminate = command_decision(2340, 109, block_argument(payload), b"reconciliation_indeterminate", 0)
    return yielding([
        expression(2290, "const_i64", 5),
        expression(2291, "lt_i64", {"lhs": parameter(602), "rhs": result(2290)}),
        expression(2292, "if", {
            "condition": result(2291), "result": nominal(300), "then_body": present,
            "else_body": yielding([
                expression(2293, "const_i64", 6),
                expression(2294, "lt_i64", {"lhs": parameter(602), "rhs": result(2293)}),
                expression(2295, "if", {
                    "condition": result(2294), "result": nominal(300),
                    "then_body": absent, "else_body": indeterminate,
                }),
            ], result(2295)),
        }),
    ], result(2292))


def resume_function():
    def resume_case(variant, payload, start):
        del start
        if variant == 103:
            return classify_validation(payload)
        if variant == 104:
            return classify_activation(payload)
        if variant == 109:
            return classify_reconciliation(payload)
        return unchanged_decision(2400 + variant * 2, parameter(601), b"unexpected_host_outcome")

    return function(
        600,
        "transition_resume",
        [
            {"symbol": 601, "name": "state", "ty": nominal(100)},
            {"symbol": 602, "name": "outcome", "ty": "i64"},
            {"symbol": 603, "name": "evidence", "ty": "bytes"},
        ],
        nominal(300),
        [expression(610, "match_sum", {
            "scrutinee": parameter(601),
            "result": nominal(300),
            "arms": state_match_arms(10000, resume_case),
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
        {"kind": "create_product_type", "data": {
            "symbol": symbol(300), "module": local(2), "name": "TransitionDecision",
            "fields": [
                {"symbol": symbol(301), "name": "state", "ty": nominal(100)},
                {"symbol": symbol(302), "name": "response", "ty": "bytes"},
                {"symbol": symbol(303), "name": "command", "ty": "i64"},
                {"symbol": symbol(304), "name": "target", "ty": "bytes"},
            ],
        }},
        function(
            400, "make_decision",
            [
                {"symbol": 401, "name": "state", "ty": nominal(100)},
                {"symbol": 402, "name": "response", "ty": "bytes"},
                {"symbol": 403, "name": "command", "ty": "i64"},
                {"symbol": 404, "name": "target", "ty": "bytes"},
            ],
            nominal(300),
            [expression(405, "construct_product", {
                "product": local(300),
                "fields": [
                    {"field": local(301), "value": parameter(401)},
                    {"field": local(302), "value": parameter(402)},
                    {"field": local(303), "value": parameter(403)},
                    {"field": local(304), "value": parameter(404)},
                ],
            })],
            result(405),
        ),
        function(
            410, "done_decision",
            [
                {"symbol": 411, "name": "state", "ty": nominal(100)},
                {"symbol": 412, "name": "response", "ty": "bytes"},
            ],
            nominal(300),
            [
                expression(413, "const_i64", 0),
                expression(414, "const_bytes", b64(b"")),
                call(415, 400, [parameter(411), parameter(412), result(413), result(414)]),
            ],
            result(415),
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


def workspace_decision(ids, state_variant, state_payload, response, command, target):
    return {
        "kind": "product",
        "data": {
            "ty": ids[300],
            "fields": [
                {"field": ids[301], "value": workspace_value(ids, 100, state_variant, state_payload)},
                {"field": ids[302], "value": bytes_value(response)},
                {"field": ids[303], "value": i64_value(command)},
                {"field": ids[304], "value": bytes_value(target)},
            ],
        },
    }


def app_target(release, item):
    return {"release": release, "item": item}


def app_value(release, types, ty_name, variant_name, payload=None):
    ty = types[ty_name]
    variant = ty["variants"][variant_name]
    data = {"ty": app_target(release, ty["target"]), "variant": app_target(release, variant)}
    if payload is not None:
        data["payload"] = bytes_value(payload)
    return {"kind": "sum", "data": data}


def app_decision(release, types, state_variant, state_payload, response, command, target):
    decision = types["decision"]
    fields = decision["fields"]
    return {
        "kind": "product",
        "data": {
            "ty": app_target(release, decision["target"]),
            "fields": [
                {"field": app_target(release, fields["state"]), "value": app_value(release, types, "state", state_variant, state_payload)},
                {"field": app_target(release, fields["response"]), "value": bytes_value(response)},
                {"field": app_target(release, fields["command"]), "value": i64_value(command)},
                {"field": app_target(release, fields["target"]), "value": bytes_value(target)},
            ],
        },
    }


def export_map(receipt):
    return {item["name"]: item for item in receipt["inspection"]["exports"]}


def build_types(exports):
    state = exports["state"]
    event = exports["event"]
    decision = exports["decision"]
    return {
        "state": {
            "target": state["target"],
            "variants": {item["name"]: item["target"] for item in state["signature"]["data"]["variants"]},
        },
        "event": {
            "target": event["target"],
            "variants": {item["name"]: item["target"] for item in event["signature"]["data"]["variants"]},
        },
        "decision": {
            "target": decision["target"],
            "fields": {item["name"]: item["target"] for item in decision["signature"]["data"]["fields"]},
        },
    }


def instance_event(instance, revision, key, event, mode="commit"):
    value = {"version": 1, "mode": mode, "instance": instance, "base_revision": revision, "event": event}
    if key is not None:
        value["event_key"] = key
    return value


def resume_request(instance, revision, key, mode="commit"):
    value = {"version": 1, "mode": mode, "instance": instance, "base_revision": revision}
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
        "version": 1, "name": "primary", "instance": instance,
        "executor": "production",
        "source_directory": str(root), "slot": str(slot),
    }
    idle = app_value(release, types, "state", "idle", b"")
    create = {
        "version": 1, "mode": "validate_only", "instance": instance,
        "initial_state": idle, "grant": grant,
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
    if command["kind"] != "validate_application":
        raise RuntimeError("controller did not suspend for exact validation")
    denied = dict(grant)
    denied["slot"] = str(slots / "other.lkja")
    host = {"version": 1, "instance": instance, "command": command["id"], "grant": denied, "source_application": str(payload_path)}
    expect_error(["instance", "validate-application", "--store", str(store)], host, "capability_denied")
    host["grant"] = grant
    expect_error(
        ["instance", "fake-outcome", "--store", str(store)],
        {
            "version": 1, "instance": instance, "command": command["id"], "grant": grant,
            "outcome": "known_success", "evidence": b64(digest),
        },
        "capability_denied",
    )
    validation = json_command(["instance", "validate-application", "--store", str(store)], host)
    if validation["outcome"] != "known_success":
        raise RuntimeError("exact payload validation did not succeed")
    if not json_command(["instance", "validate-application", "--store", str(store)], host)["replayed"]:
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
    if predicted["state_digest"] != receipt["state_digest"] or receipt["command"]["kind"] != "activate_application":
        raise RuntimeError("validation resume parity or activation suspension failed")
    replayed_resume = json_command(
        ["instance", "resume", "--store", str(store)],
        resume_request(instance, 2, "validation-1"),
    )
    if not replayed_resume["replayed"] or replayed_resume["next_revision"] != 3:
        raise RuntimeError("duplicate resume did not replay the retained receipt")
    activation_host = {"version": 1, "instance": instance, "command": receipt["command"]["id"], "grant": grant, "source_application": str(payload_path)}
    activation = json_command(["instance", "execute-activation", "--store", str(store)], activation_host)
    if activation["outcome"] != "known_success" or slot.read_bytes() != payload_path.read_bytes():
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
        **grant, "instance": other, "name": "secondary", "executor": "deterministic_fake",
        "slot": str(other_slot),
    }
    other_create = {"version": 1, "mode": "commit", "instance": other, "initial_state": idle, "grant": other_grant}
    json_command(["instance", "create", "--store", str(store), "--application", str(controller_path)], other_create)
    expect_error(
        ["instance", "execute-activation", "--store", str(store)],
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
        "version": 1, "instance": other, "command": r2["command"]["id"], "grant": other_grant,
        "outcome": "known_success", "evidence": b64(wrong_digest),
    }
    json_command(["instance", "fake-outcome", "--store", str(store)], fake)
    activation_request = json_command(
        ["instance", "resume", "--store", str(store)],
        resume_request(other, 2, "fake-validation"),
    )
    fake.update({
        "command": activation_request["command"]["id"],
        "outcome": "outcome_unknown",
        "evidence": b64(b""),
    })
    unknown = json_command(["instance", "fake-outcome", "--store", str(store)], fake)
    if unknown["outcome"] != "outcome_unknown":
        raise RuntimeError("fake activation did not retain an unknown outcome")
    reconcile_request = json_command(
        ["instance", "resume", "--store", str(store)],
        resume_request(other, 3, "fake-unknown"),
    )
    if reconcile_request["command"]["kind"] != "reconcile_activation":
        raise RuntimeError("unknown outcome did not suspend for reconciliation")
    fake.update({
        "command": reconcile_request["command"]["id"],
        "outcome": "reconciliation_absent",
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
        "outcome": "known_failure_before_visibility",
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
        {"version": 1, "instance": other, "base_revision": 8},
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
        returned = [1, 100, *STATE_VARIANTS, 200, 201, 202, 203, 204, 205, 300, 301, 302, 303, 304, 500, 600, 700]
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
                {"name": "decision", "target": ids[300]},
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
            "version": 3, "root_release": release, "entry": event_entry,
            "profile": {"kind": "stateful", "data": {
                "resume": resume_entry,
                "decision": app_target(release, types["decision"]["target"]),
                "state_field": app_target(release, types["decision"]["fields"]["state"]),
                "response_field": app_target(release, types["decision"]["fields"]["response"]),
                "command_field": app_target(release, types["decision"]["fields"]["command"]),
                "target_field": app_target(release, types["decision"]["fields"]["target"]),
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
                    "arguments": [app_value(release, types, "state", "validating", target_bytes), i64_value(1), bytes_value(target_bytes)],
                    "expected": {"kind": "value", "data": app_decision(release, types, "activating", target_bytes, b"validation_succeeded", 2, target_bytes)},
                    "policy": {"fuel": 100000, "maximum_frames": 128},
                },
            ],
        }
        controller_path = root / "controller.lkja"
        json_command(["app", "build", "--release", str(release_path), "--output", str(controller_path)], controller_request)
        identity = app_target(release, exports["identity"]["target"])
        payload_request = {
            "version": 3, "root_release": release, "entry": identity,
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
            "contract_versions": {"workspace": 10, "release": 1, "application": 3, "instance": 1},
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
