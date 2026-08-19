#!/usr/bin/env python3
"""Reproduce the exact lkjwork release, application, and generated bindings.

This is a public-boundary authoring recipe: it submits one ordinary semantic
transaction, then uses the supported release and application commands.  It
does not import lkjscript implementation modules or construct private Rust
types.
"""

import argparse
import base64
import json
import pathlib
import subprocess
import tempfile


MACHINE_VERSION = 11
RELEASE_VERSION = 2
APPLICATION_VERSION = 5


def local(name):
    return {"kind": "draft", "data": name}


def nominal(name):
    return {"nominal": local(name)}


def parameter(name):
    return {"kind": "function_parameter", "data": local(name)}


def block_argument(name):
    return {"kind": "block_argument", "data": local(name)}


def result(name):
    return {"kind": "operation_result", "data": {"operation": local(name), "output": 0}}


def expression(name, kind, data=None):
    operation = {"kind": kind}
    if data is not None:
        operation["data"] = data
    return {"symbol": name, "operation": operation}


def yielding(operations, value):
    return {"operations": operations, "yield_value": value}


def arm(variant, body, payload=None):
    value = {"variant": local(variant), "body": body}
    if payload is not None:
        value["payload_symbol"] = payload
    return value


def function(name, parameters, result_type, operations, return_value):
    return {
        "kind": "create_function",
        "data": {
            "symbol": name,
            "module": local("main_module"),
            "name": name,
            "parameters": [
                {"symbol": symbol, "name": parameter_name, "ty": ty}
                for symbol, parameter_name, ty in parameters
            ],
            "result": result_type,
            "body": {"operations": operations, "return_value": return_value},
        },
    }


def call(name, target, arguments):
    return expression(name, "call", {"function": local(target), "arguments": arguments})


def product_type(name, fields):
    return {
        "kind": "create_product_type",
        "data": {
            "symbol": name,
            "module": local("main_module"),
            "name": name,
            "fields": [
                {"symbol": field, "name": field_name, "ty": ty}
                for field, field_name, ty in fields
            ],
        },
    }


def sum_type(name, variants):
    values = []
    for variant, variant_name, payload in variants:
        value = {"symbol": variant, "name": variant_name}
        if payload is not None:
            value["payload"] = payload
        values.append(value)
    return {
        "kind": "create_sum_type",
        "data": {
            "symbol": name,
            "module": local("main_module"),
            "name": name,
            "variants": values,
        },
    }


def sequence_type(name, element):
    return {
        "kind": "create_sequence_type",
        "data": {
            "symbol": name,
            "module": local("main_module"),
            "name": name,
            "element": element,
        },
    }


def project_fields(value, prefix):
    return [
        expression(f"{prefix}_name", "project_field", {"value": value, "field": local("project_name_field")}),
        expression(f"{prefix}_next_task", "project_field", {"value": value, "field": local("project_next_task_field")}),
        expression(f"{prefix}_next_note", "project_field", {"value": value, "field": local("project_next_note_field")}),
        expression(f"{prefix}_tasks", "project_field", {"value": value, "field": local("project_tasks_field")}),
        expression(f"{prefix}_activity", "project_field", {"value": value, "field": local("project_activity_field")}),
        expression(f"{prefix}_pending", "project_field", {"value": value, "field": local("project_pending_field")}),
    ]


def construct_project(name, prefix, name_value, next_task, next_note, tasks, activity, pending):
    return expression(name, "construct_product", {
        "product": local("project"),
        "fields": [
            {"field": local("project_name_field"), "value": name_value},
            {"field": local("project_next_task_field"), "value": next_task},
            {"field": local("project_next_note_field"), "value": next_note},
            {"field": local("project_tasks_field"), "value": tasks},
            {"field": local("project_activity_field"), "value": activity},
            {"field": local("project_pending_field"), "value": pending},
        ],
    })


def task_fields(value, prefix):
    field_names = [
        ("id", "task_id_field"),
        ("title", "task_title_field"),
        ("description", "task_description_field"),
        ("phase", "task_phase_field"),
        ("hold", "task_hold_field"),
        ("priority", "task_priority_field"),
        ("labels", "task_labels_field"),
        ("dependencies", "task_dependencies_field"),
        ("notes", "task_notes_field"),
        ("attachments", "task_attachments_field"),
        ("archived", "task_archived_field"),
    ]
    return [
        expression(f"{prefix}_{short}", "project_field", {"value": value, "field": local(field)})
        for short, field in field_names
    ]


def response_functions():
    operations = [
        expression("response_detail_make", "construct_product", {
            "product": local("response_detail"),
            "fields": [
                {"field": local("response_detail_task_field"), "value": parameter("response_detail_task")},
                {"field": local("response_detail_code_field"), "value": parameter("response_detail_code")},
            ],
        }),
    ]
    result_functions = [function(
        "make_response_detail",
        [("response_detail_task", "task", "i64"), ("response_detail_code", "code", "text")],
        nominal("response_detail"),
        operations,
        result("response_detail_make"),
    )]
    for kind in ["accepted", "conflict", "no_change"]:
        result_functions.append(function(
            f"make_response_{kind}",
            [(f"response_{kind}_task", "task", "i64"), (f"response_{kind}_code", "code", "text")],
            nominal("mutation_response"),
            [
                call(
                    f"response_{kind}_detail",
                    "make_response_detail",
                    [parameter(f"response_{kind}_task"), parameter(f"response_{kind}_code")],
                ),
                expression(f"response_{kind}_sum", "construct_variant", {
                    "variant": local(f"response_{kind}_variant"),
                    "payload": result(f"response_{kind}_detail"),
                }),
            ],
            result(f"response_{kind}_sum"),
        ))
    return result_functions


def decision_functions():
    functions = []
    for kind in ["declined", "unchanged"]:
        functions.append(function(
            f"make_{kind}",
            [(f"{kind}_response_parameter", "response", nominal("mutation_response"))],
            nominal("mutation_decision"),
            [
                expression(f"{kind}_payload_make", "construct_product", {
                    "product": local(f"{kind}_payload"),
                    "fields": [{
                        "field": local(f"{kind}_response_field"),
                        "value": parameter(f"{kind}_response_parameter"),
                    }],
                }),
                expression(f"{kind}_decision_make", "construct_variant", {
                    "variant": local(f"decision_{kind}_variant"),
                    "payload": result(f"{kind}_payload_make"),
                }),
            ],
            result(f"{kind}_decision_make"),
        ))
    functions.append(function(
        "make_completed",
        [
            ("completed_state_parameter", "state", nominal("project")),
            ("completed_response_parameter", "response", nominal("mutation_response")),
        ],
        nominal("mutation_decision"),
        [
            expression("completed_payload_make", "construct_product", {
                "product": local("completed_payload"),
                "fields": [
                    {"field": local("completed_state_field"), "value": parameter("completed_state_parameter")},
                    {"field": local("completed_response_field"), "value": parameter("completed_response_parameter")},
                ],
            }),
            expression("completed_decision_make", "construct_variant", {
                "variant": local("decision_completed_variant"),
                "payload": result("completed_payload_make"),
            }),
        ],
        result("completed_decision_make"),
    ))
    functions.append(function(
        "make_suspended",
        [
            ("suspended_state_parameter", "state", nominal("project")),
            ("suspended_response_parameter", "response", nominal("mutation_response")),
            ("suspended_command_parameter", "command", nominal("host_command")),
        ],
        nominal("mutation_decision"),
        [
            expression("suspended_payload_make", "construct_product", {
                "product": local("suspended_payload"),
                "fields": [
                    {"field": local("suspended_state_field"), "value": parameter("suspended_state_parameter")},
                    {"field": local("suspended_response_field"), "value": parameter("suspended_response_parameter")},
                    {"field": local("suspended_command_field"), "value": parameter("suspended_command_parameter")},
                ],
            }),
            expression("suspended_decision_make", "construct_variant", {
                "variant": local("decision_suspended_variant"),
                "payload": result("suspended_payload_make"),
            }),
        ],
        result("suspended_decision_make"),
    ))
    return functions


def unsupported_function():
    return function(
        "unsupported_event",
        [],
        nominal("mutation_decision"),
        [
            expression("unsupported_task", "const_i64", 0),
            expression("unsupported_code", "const_text", "operation_not_implemented"),
            call("unsupported_response", "make_response_conflict", [result("unsupported_task"), result("unsupported_code")]),
            call("unsupported_result", "make_declined", [result("unsupported_response")]),
        ],
        result("unsupported_result"),
    )


def create_task_body():
    return yielding([
        expression("create_title", "project_field", {"value": block_argument("event_create_task_payload"), "field": local("create_title_field")}),
        expression("create_title_len", "text_len", {"value": result("create_title")}),
        expression("create_zero", "const_i64", 0),
        expression("create_title_valid", "lt_i64", {"lhs": result("create_zero"), "rhs": result("create_title_len")}),
        expression("create_actor_input", "project_field", {
            "value": block_argument("event_create_task_payload"), "field": local("create_actor_field"),
        }),
        expression("create_actor_len", "text_len", {"value": result("create_actor_input")}),
        expression("create_actor_valid", "lt_i64", {
            "lhs": result("create_zero"), "rhs": result("create_actor_len"),
        }),
        expression("create_labels_input", "project_field", {
            "value": block_argument("event_create_task_payload"), "field": local("create_labels_field"),
        }),
        call("create_labels_valid", "labels_are_valid", [result("create_labels_input")]),
        expression("create_dependencies_input", "project_field", {
            "value": block_argument("event_create_task_payload"), "field": local("create_dependencies_field"),
        }),
        call("create_dependencies_unique", "id_sequence_is_unique", [result("create_dependencies_input")]),
        expression("create_existing_tasks", "project_field", {
            "value": parameter("transition_state"), "field": local("project_tasks_field"),
        }),
        call("create_dependencies_exist", "dependencies_exist", [
            result("create_existing_tasks"), result("create_dependencies_input"),
        ]),
        expression("create_next_id", "project_field", {
            "value": parameter("transition_state"), "field": local("project_next_task_field"),
        }),
        call("create_dependencies_self", "id_sequence_contains", [
            result("create_dependencies_input"), result("create_next_id"),
        ]),
        expression("create_dependencies_not_self", "not_bool", {
            "value": result("create_dependencies_self"),
        }),
        expression("create_valid_a", "and_bool", {
            "lhs": result("create_title_valid"), "rhs": result("create_actor_valid"),
        }),
        expression("create_valid_b", "and_bool", {
            "lhs": result("create_valid_a"), "rhs": result("create_labels_valid"),
        }),
        expression("create_valid_c", "and_bool", {
            "lhs": result("create_valid_b"), "rhs": result("create_dependencies_unique"),
        }),
        expression("create_valid_d", "and_bool", {
            "lhs": result("create_valid_c"), "rhs": result("create_dependencies_exist"),
        }),
        expression("create_valid", "and_bool", {
            "lhs": result("create_valid_d"), "rhs": result("create_dependencies_not_self"),
        }),
        expression("create_choice", "if", {
            "condition": result("create_valid"),
            "result": nominal("mutation_decision"),
            "then_body": create_task_valid_body(),
            "else_body": yielding([
                expression("create_empty_code", "if", {
                    "condition": result("create_title_valid"),
                    "result": "text",
                    "then_body": yielding([
                        expression("create_invalid_code", "const_text", "task_input_invalid"),
                    ], result("create_invalid_code")),
                    "else_body": yielding([
                        expression("create_title_empty_code", "const_text", "title_empty"),
                    ], result("create_title_empty_code")),
                }),
                call("create_empty_response", "make_response_conflict", [result("create_zero"), result("create_empty_code")]),
                call("create_empty_declined", "make_declined", [result("create_empty_response")]),
            ], result("create_empty_declined")),
        }),
    ], result("create_choice"))


def create_task_valid_body():
    return yielding([
        *project_fields(parameter("transition_state"), "create_project"),
        expression("create_description", "project_field", {"value": block_argument("event_create_task_payload"), "field": local("create_description_field")}),
        expression("create_priority", "project_field", {"value": block_argument("event_create_task_payload"), "field": local("create_priority_field")}),
        expression("create_labels", "project_field", {"value": block_argument("event_create_task_payload"), "field": local("create_labels_field")}),
        expression("create_dependencies", "project_field", {"value": block_argument("event_create_task_payload"), "field": local("create_dependencies_field")}),
        expression("create_actor", "project_field", {"value": block_argument("event_create_task_payload"), "field": local("create_actor_field")}),
        expression("create_phase", "construct_variant", {"variant": local("phase_planned_variant")}),
        expression("create_hold", "construct_variant", {"variant": local("hold_none_variant")}),
        expression("create_notes", "sequence_empty", {"sequence": local("note_sequence")}),
        expression("create_attachments", "sequence_empty", {"sequence": local("attachment_sequence")}),
        expression("create_archived", "const_bool", False),
        expression("create_task_make", "construct_product", {
            "product": local("task"),
            "fields": [
                {"field": local("task_id_field"), "value": result("create_project_next_task")},
                {"field": local("task_title_field"), "value": result("create_title")},
                {"field": local("task_description_field"), "value": result("create_description")},
                {"field": local("task_phase_field"), "value": result("create_phase")},
                {"field": local("task_hold_field"), "value": result("create_hold")},
                {"field": local("task_priority_field"), "value": result("create_priority")},
                {"field": local("task_labels_field"), "value": result("create_labels")},
                {"field": local("task_dependencies_field"), "value": result("create_dependencies")},
                {"field": local("task_notes_field"), "value": result("create_notes")},
                {"field": local("task_attachments_field"), "value": result("create_attachments")},
                {"field": local("task_archived_field"), "value": result("create_archived")},
            ],
        }),
        expression("create_tasks_append", "sequence_append", {
            "sequence": local("task_sequence"),
            "value": result("create_project_tasks"),
            "element": result("create_task_make"),
        }),
        expression("create_one", "const_i64", 1),
        expression("create_next_task", "add_i64", {"lhs": result("create_project_next_task"), "rhs": result("create_one")}),
        expression("create_activity_make", "construct_product", {
            "product": local("activity"),
            "fields": [
                {"field": local("activity_task_field"), "value": result("create_project_next_task")},
                {"field": local("activity_actor_field"), "value": result("create_actor")},
                {"field": local("activity_code_field"), "value": result("create_code")},
            ],
        }),
        expression("create_activity_append", "sequence_append", {
            "sequence": local("activity_sequence"),
            "value": result("create_project_activity"),
            "element": result("create_activity_make"),
        }),
        construct_project(
            "create_project_make",
            "create",
            result("create_project_name"),
            result("create_next_task"),
            result("create_project_next_note"),
            result("create_tasks_append"),
            result("create_activity_append"),
            result("create_project_pending"),
        ),
        call("create_response", "make_response_accepted", [result("create_project_next_task"), result("create_code")]),
        call("create_completed", "make_completed", [result("create_project_make"), result("create_response")]),
    ], result("create_completed"))


def rename_body():
    return yielding([
        expression("rename_len", "text_len", {"value": block_argument("event_rename_project_payload")}),
        expression("rename_zero", "const_i64", 0),
        expression("rename_valid", "lt_i64", {"lhs": result("rename_zero"), "rhs": result("rename_len")}),
        expression("rename_choice", "if", {
            "condition": result("rename_valid"),
            "result": nominal("mutation_decision"),
            "then_body": rename_valid_body(),
            "else_body": yielding([
                expression("rename_empty_code", "const_text", "project_name_empty"),
                call("rename_empty_response", "make_response_conflict", [result("rename_zero"), result("rename_empty_code")]),
                call("rename_empty_declined", "make_declined", [result("rename_empty_response")]),
            ], result("rename_empty_declined")),
        }),
    ], result("rename_choice"))


def rename_valid_body():
    return yielding([
        *project_fields(parameter("transition_state"), "rename_project"),
        expression("rename_equal", "text_equal", {"lhs": result("rename_project_name"), "rhs": block_argument("event_rename_project_payload")}),
        expression("rename_result", "if", {
            "condition": result("rename_equal"),
            "result": nominal("mutation_decision"),
            "then_body": yielding([
                expression("rename_same_code", "const_text", "project_name_unchanged"),
                call("rename_same_response", "make_response_no_change", [result("rename_zero"), result("rename_same_code")]),
                call("rename_same_decision", "make_unchanged", [result("rename_same_response")]),
            ], result("rename_same_decision")),
            "else_body": yielding([
                construct_project(
                    "rename_project_make",
                    "rename",
                    block_argument("event_rename_project_payload"),
                    result("rename_project_next_task"),
                    result("rename_project_next_note"),
                    result("rename_project_tasks"),
                    result("rename_project_activity"),
                    result("rename_project_pending"),
                ),
                expression("rename_code", "const_text", "project_renamed"),
                call("rename_response", "make_response_accepted", [result("rename_zero"), result("rename_code")]),
                call("rename_completed", "make_completed", [result("rename_project_make"), result("rename_response")]),
            ], result("rename_completed")),
        }),
    ], result("rename_result"))


def transition_function(event_variants):
    handlers = {
        "edit_task": "handle_edit_task",
        "start_task": "handle_start_task",
        "stop_task": "handle_stop_task",
        "complete_task": "handle_complete_task",
        "cancel_task": "handle_cancel_task",
        "reopen_task": "handle_reopen_task",
        "set_priority": "handle_set_priority",
        "hold_task": "handle_hold_task",
        "release_task": "handle_release_task",
        "add_label": "handle_add_label",
        "remove_label": "handle_remove_label",
        "add_dependency": "handle_add_dependency",
        "remove_dependency": "handle_remove_dependency",
        "add_note": "handle_add_note",
        "request_attachment": "handle_request_attachment",
        "archive_task": "handle_archive_task",
        "unarchive_task": "handle_unarchive_task",
    }
    event_arms = []
    for variant, variant_name, payload in event_variants:
        payload_symbol = f"event_{variant_name}_payload" if payload is not None else None
        if variant_name == "create_task":
            body = create_task_body()
        elif variant_name == "rename_project":
            body = rename_body()
        elif variant_name in handlers:
            call_name = f"dispatch_{variant_name}"
            body = yielding([
                call(call_name, handlers[variant_name], [
                    parameter("transition_state"), block_argument(payload_symbol),
                ]),
            ], result(call_name))
        else:
            call_name = f"unsupported_{variant_name}_call"
            body = yielding([call(call_name, "unsupported_event", [])], result(call_name))
        event_arms.append(arm(variant, body, payload_symbol))
    return function(
        "transition_event",
        [
            ("transition_state", "state", nominal("project")),
            ("transition_event_value", "event", nominal("mutation_event")),
        ],
        nominal("mutation_decision"),
        [
            expression("create_code", "const_text", "task_created"),
            expression("transition_match", "match_sum", {
                "scrutinee": parameter("transition_event_value"),
                "result": nominal("mutation_decision"),
                "arms": event_arms,
            }),
        ],
        result("transition_match"),
    )


def find_task_function():
    return function(
        "find_task_index",
        [("find_tasks", "tasks", nominal("task_sequence")), ("find_id", "task", "i64")],
        "i64",
        [
            expression("find_zero", "const_i64", 0),
            expression("find_one", "const_i64", 1),
            expression("find_count", "sequence_len", {
                "sequence": local("task_sequence"), "value": parameter("find_tasks"),
            }),
            expression("find_upper", "add_i64", {
                "lhs": result("find_count"), "rhs": result("find_one"),
            }),
            expression("find_positive", "lt_i64", {
                "lhs": result("find_zero"), "rhs": parameter("find_id"),
            }),
            expression("find_below_upper", "lt_i64", {
                "lhs": parameter("find_id"), "rhs": result("find_upper"),
            }),
            expression("find_in_range", "and_bool", {
                "lhs": result("find_positive"), "rhs": result("find_below_upper"),
            }),
            expression("find_missing", "const_i64", -1),
            expression("find_result", "if", {
                "condition": result("find_in_range"),
                "result": "i64",
                "then_body": yielding([
                    expression("find_index", "add_i64", {
                        "lhs": parameter("find_id"), "rhs": result("find_missing"),
                    }),
                    expression("find_task_get", "sequence_get", {
                        "sequence": local("task_sequence"),
                        "value": parameter("find_tasks"),
                        "index": result("find_index"),
                    }),
                    expression("find_task_id", "project_field", {
                        "value": result("find_task_get"), "field": local("task_id_field"),
                    }),
                    expression("find_equal", "equal_i64", {
                        "lhs": result("find_task_id"), "rhs": parameter("find_id"),
                    }),
                    expression("find_verified", "if", {
                        "condition": result("find_equal"),
                        "result": "i64",
                        "then_body": yielding([], result("find_index")),
                        "else_body": yielding([], result("find_missing")),
                    }),
                ], result("find_verified")),
                "else_body": yielding([], result("find_missing")),
            }),
        ],
        result("find_result"),
    )


TASK_FIELD_ORDER = [
    "id", "title", "description", "phase", "hold", "priority", "labels",
    "dependencies", "notes", "attachments", "archived",
]


def task_support_functions():
    functions = [function(
        "make_task",
        [(f"make_task_{name}", name, {
            "id": "i64",
            "title": "text",
            "description": "text",
            "phase": nominal("task_phase"),
            "hold": nominal("task_hold"),
            "priority": "i64",
            "labels": nominal("text_sequence"),
            "dependencies": nominal("id_sequence"),
            "notes": nominal("note_sequence"),
            "attachments": nominal("attachment_sequence"),
            "archived": "bool",
        }[name]) for name in TASK_FIELD_ORDER],
        nominal("task"),
        [expression("make_task_value", "construct_product", {
            "product": local("task"),
            "fields": [
                {"field": local(f"task_{name}_field"), "value": parameter(f"make_task_{name}")}
                for name in TASK_FIELD_ORDER
            ],
        })],
        result("make_task_value"),
    )]
    functions.append(function(
        "replace_project_task",
        [
            ("replace_state", "state", nominal("project")),
            ("replace_index", "index", "i64"),
            ("replace_task", "task", nominal("task")),
            ("replace_actor", "actor", "text"),
            ("replace_code", "code", "text"),
        ],
        nominal("project"),
        [
            *project_fields(parameter("replace_state"), "replace_project"),
            expression("replace_task_id", "project_field", {"value": parameter("replace_task"), "field": local("task_id_field")}),
            expression("replace_tasks_value", "sequence_replace", {
                "sequence": local("task_sequence"),
                "value": result("replace_project_tasks"),
                "index": parameter("replace_index"),
                "element": parameter("replace_task"),
            }),
            expression("replace_activity_value", "construct_product", {
                "product": local("activity"),
                "fields": [
                    {"field": local("activity_task_field"), "value": result("replace_task_id")},
                    {"field": local("activity_actor_field"), "value": parameter("replace_actor")},
                    {"field": local("activity_code_field"), "value": parameter("replace_code")},
                ],
            }),
            expression("replace_activity_append", "sequence_append", {
                "sequence": local("activity_sequence"),
                "value": result("replace_project_activity"),
                "element": result("replace_activity_value"),
            }),
            construct_project(
                "replace_project_value",
                "replace",
                result("replace_project_name"),
                result("replace_project_next_task"),
                result("replace_project_next_note"),
                result("replace_tasks_value"),
                result("replace_activity_append"),
                result("replace_project_pending"),
            ),
        ],
        result("replace_project_value"),
    ))
    for name, response_function, decision_function in [
        ("decline_code", "make_response_conflict", "make_declined"),
        ("unchanged_code", "make_response_no_change", "make_unchanged"),
    ]:
        functions.append(function(
            name,
            [(f"{name}_task", "task", "i64"), (f"{name}_text", "code", "text")],
            nominal("mutation_decision"),
            [
                call(f"{name}_response", response_function, [parameter(f"{name}_task"), parameter(f"{name}_text")]),
                call(f"{name}_decision", decision_function, [result(f"{name}_response")]),
            ],
            result(f"{name}_decision"),
        ))
    functions.append(function(
        "accept_task_change",
        [
            ("accept_state", "state", nominal("project")),
            ("accept_index", "index", "i64"),
            ("accept_task_value", "task_value", nominal("task")),
            ("accept_task_id", "task", "i64"),
            ("accept_actor", "actor", "text"),
            ("accept_code", "code", "text"),
        ],
        nominal("mutation_decision"),
        [
            call("accept_project", "replace_project_task", [
                parameter("accept_state"), parameter("accept_index"), parameter("accept_task_value"),
                parameter("accept_actor"), parameter("accept_code"),
            ]),
            call("accept_response", "make_response_accepted", [parameter("accept_task_id"), parameter("accept_code")]),
            call("accept_decision", "make_completed", [result("accept_project"), result("accept_response")]),
        ],
        result("accept_decision"),
    ))
    return functions


def predicate_function(name, owner, selected):
    variants = {
        "task_phase": ["planned", "active", "done", "cancelled"],
        "task_hold": ["none", "manual"],
    }[owner]
    arms = []
    for variant in variants:
        operation = f"{name}_{variant}_bool"
        payload = f"{name}_{variant}_payload" if owner == "task_hold" and variant == "manual" else None
        arms.append(arm(
            f"{owner.removeprefix('task_')}_{variant}_variant",
            yielding([expression(operation, "const_bool", variant in selected)], result(operation)),
            payload,
        ))
    return function(
        name,
        [(f"{name}_value", "value", nominal(owner))],
        "bool",
        [expression(f"{name}_match", "match_sum", {
            "scrutinee": parameter(f"{name}_value"),
            "result": "bool",
            "arms": arms,
        })],
        result(f"{name}_match"),
    )


def state_predicate_functions():
    return [
        predicate_function("phase_is_planned", "task_phase", {"planned"}),
        predicate_function("phase_is_active", "task_phase", {"active"}),
        predicate_function("phase_is_done", "task_phase", {"done"}),
        predicate_function("phase_is_cancelled", "task_phase", {"cancelled"}),
        predicate_function("phase_is_terminal", "task_phase", {"done", "cancelled"}),
        predicate_function("hold_is_none", "task_hold", {"none"}),
    ]


def id_contains_function():
    return function(
        "id_sequence_contains",
        [("id_contains_values", "values", nominal("id_sequence")), ("id_contains_target", "target", "i64")],
        "bool",
        [
            expression("id_contains_start", "const_i64", 0),
            expression("id_contains_end", "sequence_len", {"sequence": local("id_sequence"), "value": parameter("id_contains_values")}),
            expression("id_contains_initial", "const_bool", False),
            expression("id_contains_loop", "for_i64", {
                "start": result("id_contains_start"),
                "end_exclusive": result("id_contains_end"),
                "step": 1,
                "initial": result("id_contains_initial"),
                "carried": "bool",
                "index_symbol": "id_contains_index",
                "carried_symbol": "id_contains_found",
                "body": yielding([
                    expression("id_contains_item", "sequence_get", {
                        "sequence": local("id_sequence"), "value": parameter("id_contains_values"),
                        "index": block_argument("id_contains_index"),
                    }),
                    expression("id_contains_equal", "equal_i64", {
                        "lhs": result("id_contains_item"), "rhs": parameter("id_contains_target"),
                    }),
                    expression("id_contains_combined", "or_bool", {
                        "lhs": block_argument("id_contains_found"), "rhs": result("id_contains_equal"),
                    }),
                ], result("id_contains_combined")),
            }),
        ],
        result("id_contains_loop"),
    )


def text_contains_function():
    return function(
        "text_sequence_contains",
        [("text_contains_values", "values", nominal("text_sequence")), ("text_contains_target", "target", "text")],
        "bool",
        [
            expression("text_contains_start", "const_i64", 0),
            expression("text_contains_end", "sequence_len", {"sequence": local("text_sequence"), "value": parameter("text_contains_values")}),
            expression("text_contains_initial", "const_bool", False),
            expression("text_contains_loop", "for_i64", {
                "start": result("text_contains_start"),
                "end_exclusive": result("text_contains_end"),
                "step": 1,
                "initial": result("text_contains_initial"),
                "carried": "bool",
                "index_symbol": "text_contains_index",
                "carried_symbol": "text_contains_found",
                "body": yielding([
                    expression("text_contains_item", "sequence_get", {
                        "sequence": local("text_sequence"), "value": parameter("text_contains_values"),
                        "index": block_argument("text_contains_index"),
                    }),
                    expression("text_contains_equal", "text_equal", {
                        "lhs": result("text_contains_item"), "rhs": parameter("text_contains_target"),
                    }),
                    expression("text_contains_combined", "or_bool", {
                        "lhs": block_argument("text_contains_found"), "rhs": result("text_contains_equal"),
                    }),
                ], result("text_contains_combined")),
            }),
        ],
        result("text_contains_loop"),
    )


def collection_validation_functions():
    functions = []
    for name, sequence, element_type, equal_operation in [
        ("id_sequence_is_unique", "id_sequence", "i64", "equal_i64"),
        ("text_sequence_is_unique", "text_sequence", "text", "text_equal"),
    ]:
        functions.append(function(
            name,
            [(f"{name}_values", "values", nominal(sequence))],
            "bool",
            [
                expression(f"{name}_zero", "const_i64", 0),
                expression(f"{name}_end", "sequence_len", {
                    "sequence": local(sequence), "value": parameter(f"{name}_values"),
                }),
                expression(f"{name}_true", "const_bool", True),
                expression(f"{name}_loop", "for_i64", {
                    "start": result(f"{name}_zero"),
                    "end_exclusive": result(f"{name}_end"),
                    "step": 1,
                    "initial": result(f"{name}_true"),
                    "carried": "bool",
                    "index_symbol": f"{name}_index",
                    "carried_symbol": f"{name}_valid",
                    "body": yielding([
                        expression(f"{name}_item", "sequence_get", {
                            "sequence": local(sequence), "value": parameter(f"{name}_values"),
                            "index": block_argument(f"{name}_index"),
                        }),
                        expression(f"{name}_false", "const_bool", False),
                        expression(f"{name}_prior_loop", "for_i64", {
                            "start": result(f"{name}_zero"),
                            "end_exclusive": block_argument(f"{name}_index"),
                            "step": 1,
                            "initial": result(f"{name}_false"),
                            "carried": "bool",
                            "index_symbol": f"{name}_prior_index",
                            "carried_symbol": f"{name}_duplicate",
                            "body": yielding([
                                expression(f"{name}_prior", "sequence_get", {
                                    "sequence": local(sequence), "value": parameter(f"{name}_values"),
                                    "index": block_argument(f"{name}_prior_index"),
                                }),
                                expression(f"{name}_equal", equal_operation, {
                                    "lhs": result(f"{name}_prior"), "rhs": result(f"{name}_item"),
                                }),
                                expression(f"{name}_found", "or_bool", {
                                    "lhs": block_argument(f"{name}_duplicate"), "rhs": result(f"{name}_equal"),
                                }),
                            ], result(f"{name}_found")),
                        }),
                        expression(f"{name}_not_duplicate", "not_bool", {
                            "value": result(f"{name}_prior_loop"),
                        }),
                        expression(f"{name}_combined", "and_bool", {
                            "lhs": block_argument(f"{name}_valid"), "rhs": result(f"{name}_not_duplicate"),
                        }),
                    ], result(f"{name}_combined")),
                }),
            ],
            result(f"{name}_loop"),
        ))
    functions.append(function(
        "labels_are_valid",
        [("label_values", "labels", nominal("text_sequence"))],
        "bool",
        [
            call("label_unique", "text_sequence_is_unique", [parameter("label_values")]),
            expression("label_zero", "const_i64", 0),
            expression("label_end", "sequence_len", {
                "sequence": local("text_sequence"), "value": parameter("label_values"),
            }),
            expression("label_true", "const_bool", True),
            expression("label_nonempty_loop", "for_i64", {
                "start": result("label_zero"),
                "end_exclusive": result("label_end"),
                "step": 1,
                "initial": result("label_true"),
                "carried": "bool",
                "index_symbol": "label_index",
                "carried_symbol": "labels_nonempty",
                "body": yielding([
                    expression("label_item", "sequence_get", {
                        "sequence": local("text_sequence"), "value": parameter("label_values"),
                        "index": block_argument("label_index"),
                    }),
                    expression("label_length", "text_len", {"value": result("label_item")}),
                    expression("label_nonempty", "lt_i64", {
                        "lhs": result("label_zero"), "rhs": result("label_length"),
                    }),
                    expression("labels_nonempty_next", "and_bool", {
                        "lhs": block_argument("labels_nonempty"), "rhs": result("label_nonempty"),
                    }),
                ], result("labels_nonempty_next")),
            }),
            expression("labels_valid", "and_bool", {
                "lhs": result("label_unique"), "rhs": result("label_nonempty_loop"),
            }),
        ],
        result("labels_valid"),
    ))
    functions.append(function(
        "dependencies_exist",
        [
            ("dependency_exists_tasks", "tasks", nominal("task_sequence")),
            ("dependency_exists_values", "dependencies", nominal("id_sequence")),
        ],
        "bool",
        [
            expression("dependency_exists_zero", "const_i64", 0),
            expression("dependency_exists_end", "sequence_len", {
                "sequence": local("id_sequence"), "value": parameter("dependency_exists_values"),
            }),
            expression("dependency_exists_true", "const_bool", True),
            expression("dependency_exists_loop", "for_i64", {
                "start": result("dependency_exists_zero"),
                "end_exclusive": result("dependency_exists_end"),
                "step": 1,
                "initial": result("dependency_exists_true"),
                "carried": "bool",
                "index_symbol": "dependency_exists_index",
                "carried_symbol": "dependency_exists_valid",
                "body": yielding([
                    expression("dependency_exists_item", "sequence_get", {
                        "sequence": local("id_sequence"), "value": parameter("dependency_exists_values"),
                        "index": block_argument("dependency_exists_index"),
                    }),
                    call("dependency_exists_task_index", "find_task_index", [
                        parameter("dependency_exists_tasks"), result("dependency_exists_item"),
                    ]),
                    expression("dependency_exists_missing", "lt_i64", {
                        "lhs": result("dependency_exists_task_index"), "rhs": result("dependency_exists_zero"),
                    }),
                    expression("dependency_exists_present", "not_bool", {
                        "value": result("dependency_exists_missing"),
                    }),
                    expression("dependency_exists_next", "and_bool", {
                        "lhs": block_argument("dependency_exists_valid"),
                        "rhs": result("dependency_exists_present"),
                    }),
                ], result("dependency_exists_next")),
            }),
        ],
        result("dependency_exists_loop"),
    ))
    return functions


def dependency_reachability_function():
    return function(
        "dependency_reaches",
        [
            ("reach_tasks", "tasks", nominal("task_sequence")),
            ("reach_start", "start", "i64"),
            ("reach_target", "target", "i64"),
        ],
        "bool",
        [
            expression("reach_zero", "const_i64", 0),
            expression("reach_one", "const_i64", 1),
            expression("reach_false", "const_bool", False),
            expression("reach_work_empty", "sequence_empty", {"sequence": local("id_sequence")}),
            expression("reach_work_start", "sequence_append", {
                "sequence": local("id_sequence"), "value": result("reach_work_empty"),
                "element": parameter("reach_start"),
            }),
            expression("reach_initial", "construct_product", {
                "product": local("dependency_search_state"),
                "fields": [
                    {"field": local("dependency_search_work_field"), "value": result("reach_work_start")},
                    {"field": local("dependency_search_found_field"), "value": result("reach_false")},
                ],
            }),
            expression("reach_task_count", "sequence_len", {
                "sequence": local("task_sequence"), "value": parameter("reach_tasks"),
            }),
            expression("reach_end", "add_i64", {
                "lhs": result("reach_task_count"), "rhs": result("reach_one"),
            }),
            expression("reach_loop", "for_i64", {
                "start": result("reach_zero"),
                "end_exclusive": result("reach_end"),
                "step": 1,
                "initial": result("reach_initial"),
                "carried": nominal("dependency_search_state"),
                "index_symbol": "reach_index",
                "carried_symbol": "reach_state",
                "body": yielding([
                    expression("reach_work", "project_field", {
                        "value": block_argument("reach_state"), "field": local("dependency_search_work_field"),
                    }),
                    expression("reach_found", "project_field", {
                        "value": block_argument("reach_state"), "field": local("dependency_search_found_field"),
                    }),
                    expression("reach_work_len", "sequence_len", {
                        "sequence": local("id_sequence"), "value": result("reach_work"),
                    }),
                    expression("reach_has_item", "lt_i64", {
                        "lhs": block_argument("reach_index"), "rhs": result("reach_work_len"),
                    }),
                    expression("reach_not_found", "not_bool", {"value": result("reach_found")}),
                    expression("reach_process", "and_bool", {
                        "lhs": result("reach_has_item"), "rhs": result("reach_not_found"),
                    }),
                    expression("reach_step", "if", {
                        "condition": result("reach_process"),
                        "result": nominal("dependency_search_state"),
                        "then_body": yielding([
                            expression("reach_current", "sequence_get", {
                                "sequence": local("id_sequence"), "value": result("reach_work"),
                                "index": block_argument("reach_index"),
                            }),
                            expression("reach_equal", "equal_i64", {
                                "lhs": result("reach_current"), "rhs": parameter("reach_target"),
                            }),
                            expression("reach_target_choice", "if", {
                                "condition": result("reach_equal"),
                                "result": nominal("dependency_search_state"),
                                "then_body": yielding([
                                    expression("reach_found_state", "construct_product", {
                                        "product": local("dependency_search_state"),
                                        "fields": [
                                            {"field": local("dependency_search_work_field"), "value": result("reach_work")},
                                            {"field": local("dependency_search_found_field"), "value": result("reach_equal")},
                                        ],
                                    }),
                                ], result("reach_found_state")),
                                "else_body": yielding([
                                    call("reach_current_index", "find_task_index", [
                                        parameter("reach_tasks"), result("reach_current"),
                                    ]),
                                    expression("reach_current_missing", "lt_i64", {
                                        "lhs": result("reach_current_index"), "rhs": result("reach_zero"),
                                    }),
                                    expression("reach_expand_choice", "if", {
                                        "condition": result("reach_current_missing"),
                                        "result": nominal("dependency_search_state"),
                                        "then_body": yielding([], block_argument("reach_state")),
                                        "else_body": yielding([
                                            expression("reach_current_task", "sequence_get", {
                                                "sequence": local("task_sequence"), "value": parameter("reach_tasks"),
                                                "index": result("reach_current_index"),
                                            }),
                                            expression("reach_dependencies", "project_field", {
                                                "value": result("reach_current_task"), "field": local("task_dependencies_field"),
                                            }),
                                            expression("reach_dependency_count", "sequence_len", {
                                                "sequence": local("id_sequence"), "value": result("reach_dependencies"),
                                            }),
                                            expression("reach_expand_loop", "for_i64", {
                                                "start": result("reach_zero"),
                                                "end_exclusive": result("reach_dependency_count"),
                                                "step": 1,
                                                "initial": result("reach_work"),
                                                "carried": nominal("id_sequence"),
                                                "index_symbol": "reach_dependency_index",
                                                "carried_symbol": "reach_expanded_work",
                                                "body": yielding([
                                                    expression("reach_dependency", "sequence_get", {
                                                        "sequence": local("id_sequence"),
                                                        "value": result("reach_dependencies"),
                                                        "index": block_argument("reach_dependency_index"),
                                                    }),
                                                    call("reach_dependency_queued", "id_sequence_contains", [
                                                        block_argument("reach_expanded_work"), result("reach_dependency"),
                                                    ]),
                                                    expression("reach_queue_choice", "if", {
                                                        "condition": result("reach_dependency_queued"),
                                                        "result": nominal("id_sequence"),
                                                        "then_body": yielding([], block_argument("reach_expanded_work")),
                                                        "else_body": yielding([
                                                            expression("reach_queue_append", "sequence_append", {
                                                                "sequence": local("id_sequence"),
                                                                "value": block_argument("reach_expanded_work"),
                                                                "element": result("reach_dependency"),
                                                            }),
                                                        ], result("reach_queue_append")),
                                                    }),
                                                ], result("reach_queue_choice")),
                                            }),
                                            expression("reach_expanded_state", "construct_product", {
                                                "product": local("dependency_search_state"),
                                                "fields": [
                                                    {"field": local("dependency_search_work_field"), "value": result("reach_expand_loop")},
                                                    {"field": local("dependency_search_found_field"), "value": result("reach_found")},
                                                ],
                                            }),
                                        ], result("reach_expanded_state")),
                                    }),
                                ], result("reach_expand_choice")),
                            }),
                        ], result("reach_target_choice")),
                        "else_body": yielding([], block_argument("reach_state")),
                    }),
                ], result("reach_step")),
            }),
            expression("reach_result", "project_field", {
                "value": result("reach_loop"), "field": local("dependency_search_found_field"),
            }),
        ],
        result("reach_result"),
    )


def readiness_functions():
    functions = [function(
        "task_id_is_done",
        [("done_tasks", "tasks", nominal("task_sequence")), ("done_id", "task", "i64")],
        "bool",
        [
            call("done_index", "find_task_index", [parameter("done_tasks"), parameter("done_id")]),
            expression("done_zero", "const_i64", 0),
            expression("done_missing", "lt_i64", {"lhs": result("done_index"), "rhs": result("done_zero")}),
            expression("done_result", "if", {
                "condition": result("done_missing"),
                "result": "bool",
                "then_body": yielding([
                    expression("done_false", "const_bool", False),
                ], result("done_false")),
                "else_body": yielding([
                    expression("done_task", "sequence_get", {
                        "sequence": local("task_sequence"), "value": parameter("done_tasks"),
                        "index": result("done_index"),
                    }),
                    expression("done_phase", "project_field", {"value": result("done_task"), "field": local("task_phase_field")}),
                    call("done_phase_result", "phase_is_done", [result("done_phase")]),
                ], result("done_phase_result")),
            }),
        ],
        result("done_result"),
    )]
    functions.append(function(
        "dependencies_are_done",
        [
            ("dependencies_tasks", "tasks", nominal("task_sequence")),
            ("dependencies_values", "dependencies", nominal("id_sequence")),
        ],
        "bool",
        [
            expression("dependencies_start", "const_i64", 0),
            expression("dependencies_end", "sequence_len", {
                "sequence": local("id_sequence"), "value": parameter("dependencies_values"),
            }),
            expression("dependencies_initial", "const_bool", True),
            expression("dependencies_loop", "for_i64", {
                "start": result("dependencies_start"),
                "end_exclusive": result("dependencies_end"),
                "step": 1,
                "initial": result("dependencies_initial"),
                "carried": "bool",
                "index_symbol": "dependencies_index",
                "carried_symbol": "dependencies_ready",
                "body": yielding([
                    expression("dependencies_id", "sequence_get", {
                        "sequence": local("id_sequence"), "value": parameter("dependencies_values"),
                        "index": block_argument("dependencies_index"),
                    }),
                    call("dependencies_done", "task_id_is_done", [parameter("dependencies_tasks"), result("dependencies_id")]),
                    expression("dependencies_combined", "and_bool", {
                        "lhs": block_argument("dependencies_ready"), "rhs": result("dependencies_done"),
                    }),
                ], result("dependencies_combined")),
            }),
        ],
        result("dependencies_loop"),
    ))
    functions.append(function(
        "task_is_ready",
        [("ready_tasks", "tasks", nominal("task_sequence")), ("ready_task", "task", nominal("task"))],
        "bool",
        [
            expression("ready_phase", "project_field", {"value": parameter("ready_task"), "field": local("task_phase_field")}),
            expression("ready_hold", "project_field", {"value": parameter("ready_task"), "field": local("task_hold_field")}),
            expression("ready_archived", "project_field", {"value": parameter("ready_task"), "field": local("task_archived_field")}),
            expression("ready_dependencies", "project_field", {"value": parameter("ready_task"), "field": local("task_dependencies_field")}),
            call("ready_planned", "phase_is_planned", [result("ready_phase")]),
            call("ready_unheld", "hold_is_none", [result("ready_hold")]),
            expression("ready_visible", "not_bool", {"value": result("ready_archived")}),
            call("ready_dependencies_done", "dependencies_are_done", [parameter("ready_tasks"), result("ready_dependencies")]),
            expression("ready_a", "and_bool", {"lhs": result("ready_planned"), "rhs": result("ready_unheld")}),
            expression("ready_b", "and_bool", {"lhs": result("ready_a"), "rhs": result("ready_visible")}),
            expression("ready_result", "and_bool", {"lhs": result("ready_b"), "rhs": result("ready_dependencies_done")}),
        ],
        result("ready_result"),
    ))
    return functions


def query_support_functions():
    functions = []
    functions.append(function(
        "task_blockers",
        [
            ("blocker_tasks", "tasks", nominal("task_sequence")),
            ("blocker_dependencies", "dependencies", nominal("id_sequence")),
        ],
        nominal("id_sequence"),
        [
            expression("blocker_zero", "const_i64", 0),
            expression("blocker_end", "sequence_len", {
                "sequence": local("id_sequence"), "value": parameter("blocker_dependencies"),
            }),
            expression("blocker_empty", "sequence_empty", {"sequence": local("id_sequence")}),
            expression("blocker_loop", "for_i64", {
                "start": result("blocker_zero"),
                "end_exclusive": result("blocker_end"),
                "step": 1,
                "initial": result("blocker_empty"),
                "carried": nominal("id_sequence"),
                "index_symbol": "blocker_index",
                "carried_symbol": "blocker_output",
                "body": yielding([
                    expression("blocker_id", "sequence_get", {
                        "sequence": local("id_sequence"), "value": parameter("blocker_dependencies"),
                        "index": block_argument("blocker_index"),
                    }),
                    call("blocker_done", "task_id_is_done", [
                        parameter("blocker_tasks"), result("blocker_id"),
                    ]),
                    expression("blocker_select", "if", {
                        "condition": result("blocker_done"),
                        "result": nominal("id_sequence"),
                        "then_body": yielding([], block_argument("blocker_output")),
                        "else_body": yielding([
                            expression("blocker_append", "sequence_append", {
                                "sequence": local("id_sequence"), "value": block_argument("blocker_output"),
                                "element": result("blocker_id"),
                            }),
                        ], result("blocker_append")),
                    }),
                ], result("blocker_select")),
            }),
        ],
        result("blocker_loop"),
    ))
    functions.append(function(
        "make_task_view",
        [
            ("view_tasks", "tasks", nominal("task_sequence")),
            ("view_task", "task", nominal("task")),
        ],
        nominal("task_view"),
        [
            expression("view_dependencies", "project_field", {
                "value": parameter("view_task"), "field": local("task_dependencies_field"),
            }),
            call("view_ready", "task_is_ready", [parameter("view_tasks"), parameter("view_task")]),
            call("view_blockers", "task_blockers", [parameter("view_tasks"), result("view_dependencies")]),
            expression("view_value", "construct_product", {
                "product": local("task_view"),
                "fields": [
                    {"field": local("task_view_task_field"), "value": parameter("view_task")},
                    {"field": local("task_view_ready_field"), "value": result("view_ready")},
                    {"field": local("task_view_blockers_field"), "value": result("view_blockers")},
                ],
            }),
        ],
        result("view_value"),
    ))
    functions.append(function(
        "task_precedes",
        [
            ("precedes_left", "left", nominal("task")),
            ("precedes_right", "right", nominal("task")),
        ],
        "bool",
        [
            expression("precedes_left_priority", "project_field", {
                "value": parameter("precedes_left"), "field": local("task_priority_field"),
            }),
            expression("precedes_right_priority", "project_field", {
                "value": parameter("precedes_right"), "field": local("task_priority_field"),
            }),
            expression("precedes_higher", "lt_i64", {
                "lhs": result("precedes_right_priority"), "rhs": result("precedes_left_priority"),
            }),
            expression("precedes_priority_equal", "equal_i64", {
                "lhs": result("precedes_left_priority"), "rhs": result("precedes_right_priority"),
            }),
            expression("precedes_left_id", "project_field", {
                "value": parameter("precedes_left"), "field": local("task_id_field"),
            }),
            expression("precedes_right_id", "project_field", {
                "value": parameter("precedes_right"), "field": local("task_id_field"),
            }),
            expression("precedes_lower_id", "lt_i64", {
                "lhs": result("precedes_left_id"), "rhs": result("precedes_right_id"),
            }),
            expression("precedes_tie", "and_bool", {
                "lhs": result("precedes_priority_equal"), "rhs": result("precedes_lower_id"),
            }),
            expression("precedes_result", "or_bool", {
                "lhs": result("precedes_higher"), "rhs": result("precedes_tie"),
            }),
        ],
        result("precedes_result"),
    ))
    functions.append(function(
        "sort_tasks",
        [
            ("sort_values", "tasks", nominal("task_sequence")),
            ("sort_maximum", "maximum", "i64"),
        ],
        nominal("task_sequence"),
        [
            expression("sort_zero", "const_i64", 0),
            expression("sort_missing", "const_i64", -1),
            expression("sort_count", "sequence_len", {
                "sequence": local("task_sequence"), "value": parameter("sort_values"),
            }),
            expression("sort_maximum_less", "lt_i64", {
                "lhs": parameter("sort_maximum"), "rhs": result("sort_count"),
            }),
            expression("sort_end", "if", {
                "condition": result("sort_maximum_less"),
                "result": "i64",
                "then_body": yielding([], parameter("sort_maximum")),
                "else_body": yielding([], result("sort_count")),
            }),
            expression("sort_empty", "sequence_empty", {"sequence": local("task_sequence")}),
            expression("sort_initial", "construct_product", {
                "product": local("task_sort_state"),
                "fields": [
                    {"field": local("task_sort_previous_field"), "value": result("sort_missing")},
                    {"field": local("task_sort_output_field"), "value": result("sort_empty")},
                ],
            }),
            expression("sort_loop", "for_i64", {
                "start": result("sort_zero"),
                "end_exclusive": result("sort_end"),
                "step": 1,
                "initial": result("sort_initial"),
                "carried": nominal("task_sort_state"),
                "index_symbol": "sort_outer_index",
                "carried_symbol": "sort_state",
                "body": yielding([
                    expression("sort_previous", "project_field", {
                        "value": block_argument("sort_state"), "field": local("task_sort_previous_field"),
                    }),
                    expression("sort_output", "project_field", {
                        "value": block_argument("sort_state"), "field": local("task_sort_output_field"),
                    }),
                    expression("sort_best_loop", "for_i64", {
                        "start": result("sort_zero"),
                        "end_exclusive": result("sort_count"),
                        "step": 1,
                        "initial": result("sort_missing"),
                        "carried": "i64",
                        "index_symbol": "sort_candidate_index",
                        "carried_symbol": "sort_best_index",
                        "body": yielding([
                            expression("sort_candidate", "sequence_get", {
                                "sequence": local("task_sequence"), "value": parameter("sort_values"),
                                "index": block_argument("sort_candidate_index"),
                            }),
                            expression("sort_previous_missing", "lt_i64", {
                                "lhs": result("sort_previous"), "rhs": result("sort_zero"),
                            }),
                            expression("sort_candidate_follows", "if", {
                                "condition": result("sort_previous_missing"),
                                "result": "bool",
                                "then_body": yielding([
                                    expression("sort_first_candidate", "const_bool", True),
                                ], result("sort_first_candidate")),
                                "else_body": yielding([
                                    expression("sort_previous_task", "sequence_get", {
                                        "sequence": local("task_sequence"), "value": parameter("sort_values"),
                                        "index": result("sort_previous"),
                                    }),
                                    call("sort_after_previous", "task_precedes", [
                                        result("sort_previous_task"), result("sort_candidate"),
                                    ]),
                                ], result("sort_after_previous")),
                            }),
                            expression("sort_select_index", "if", {
                                "condition": result("sort_candidate_follows"),
                                "result": "i64",
                                "then_body": yielding([
                                    expression("sort_best_missing", "lt_i64", {
                                        "lhs": block_argument("sort_best_index"), "rhs": result("sort_zero"),
                                    }),
                                    expression("sort_choose_candidate", "if", {
                                        "condition": result("sort_best_missing"),
                                        "result": "i64",
                                        "then_body": yielding([], block_argument("sort_candidate_index")),
                                        "else_body": yielding([
                                            expression("sort_best", "sequence_get", {
                                                "sequence": local("task_sequence"), "value": parameter("sort_values"),
                                                "index": block_argument("sort_best_index"),
                                            }),
                                            call("sort_candidate_precedes", "task_precedes", [
                                                result("sort_candidate"), result("sort_best"),
                                            ]),
                                            expression("sort_compare_index", "if", {
                                                "condition": result("sort_candidate_precedes"),
                                                "result": "i64",
                                                "then_body": yielding([], block_argument("sort_candidate_index")),
                                                "else_body": yielding([], block_argument("sort_best_index")),
                                            }),
                                        ], result("sort_compare_index")),
                                    }),
                                ], result("sort_choose_candidate")),
                                "else_body": yielding([], block_argument("sort_best_index")),
                            }),
                        ], result("sort_select_index")),
                    }),
                    expression("sort_selected", "sequence_get", {
                        "sequence": local("task_sequence"), "value": parameter("sort_values"),
                        "index": result("sort_best_loop"),
                    }),
                    expression("sort_next_output", "sequence_append", {
                        "sequence": local("task_sequence"), "value": result("sort_output"),
                        "element": result("sort_selected"),
                    }),
                    expression("sort_next_state", "construct_product", {
                        "product": local("task_sort_state"),
                        "fields": [
                            {"field": local("task_sort_previous_field"), "value": result("sort_best_loop")},
                            {"field": local("task_sort_output_field"), "value": result("sort_next_output")},
                        ],
                    }),
                ], result("sort_next_state")),
            }),
            expression("sort_result", "project_field", {
                "value": result("sort_loop"), "field": local("task_sort_output_field"),
            }),
        ],
        result("sort_result"),
    ))
    for name, mode in [
        ("visible_tasks", "visible"),
        ("actionable_tasks", "actionable"),
        ("active_tasks", "active"),
    ]:
        operations = [
            expression(f"{name}_zero", "const_i64", 0),
            expression(f"{name}_end", "sequence_len", {
                "sequence": local("task_sequence"), "value": parameter(f"{name}_values"),
            }),
            expression(f"{name}_empty", "sequence_empty", {"sequence": local("task_sequence")}),
            expression(f"{name}_loop", "for_i64", {
                "start": result(f"{name}_zero"),
                "end_exclusive": result(f"{name}_end"),
                "step": 1,
                "initial": result(f"{name}_empty"),
                "carried": nominal("task_sequence"),
                "index_symbol": f"{name}_index",
                "carried_symbol": f"{name}_output",
                "body": yielding([
                    expression(f"{name}_task", "sequence_get", {
                        "sequence": local("task_sequence"), "value": parameter(f"{name}_values"),
                        "index": block_argument(f"{name}_index"),
                    }),
                    expression(f"{name}_archived", "project_field", {
                        "value": result(f"{name}_task"), "field": local("task_archived_field"),
                    }),
                    expression(f"{name}_not_archived", "not_bool", {"value": result(f"{name}_archived")}),
                ] + (
                    [call(f"{name}_selected", "task_is_ready", [
                        parameter(f"{name}_values"), result(f"{name}_task"),
                    ])] if mode == "actionable" else
                    [
                        expression(f"{name}_phase", "project_field", {
                            "value": result(f"{name}_task"), "field": local("task_phase_field"),
                        }),
                        call(f"{name}_is_active", "phase_is_active", [result(f"{name}_phase")]),
                        expression(f"{name}_selected", "and_bool", {
                            "lhs": result(f"{name}_is_active"), "rhs": result(f"{name}_not_archived"),
                        }),
                    ] if mode == "active" else
                    [expression(f"{name}_selected", "not_bool", {"value": result(f"{name}_archived")})]
                ) + [
                    expression(f"{name}_select", "if", {
                        "condition": result(f"{name}_selected"),
                        "result": nominal("task_sequence"),
                        "then_body": yielding([
                            expression(f"{name}_append", "sequence_append", {
                                "sequence": local("task_sequence"), "value": block_argument(f"{name}_output"),
                                "element": result(f"{name}_task"),
                            }),
                        ], result(f"{name}_append")),
                        "else_body": yielding([], block_argument(f"{name}_output")),
                    }),
                ], result(f"{name}_select")),
            }),
        ]
        functions.append(function(
            name,
            [(f"{name}_values", "tasks", nominal("task_sequence"))],
            nominal("task_sequence"),
            operations,
            result(f"{name}_loop"),
        ))
    functions.append(function(
        "append_task_sequences",
        [
            ("append_tasks_left", "left", nominal("task_sequence")),
            ("append_tasks_right", "right", nominal("task_sequence")),
        ],
        nominal("task_sequence"),
        [
            expression("append_tasks_zero", "const_i64", 0),
            expression("append_tasks_end", "sequence_len", {
                "sequence": local("task_sequence"), "value": parameter("append_tasks_right"),
            }),
            expression("append_tasks_loop", "for_i64", {
                "start": result("append_tasks_zero"),
                "end_exclusive": result("append_tasks_end"),
                "step": 1,
                "initial": parameter("append_tasks_left"),
                "carried": nominal("task_sequence"),
                "index_symbol": "append_tasks_index",
                "carried_symbol": "append_tasks_output",
                "body": yielding([
                    expression("append_tasks_item", "sequence_get", {
                        "sequence": local("task_sequence"), "value": parameter("append_tasks_right"),
                        "index": block_argument("append_tasks_index"),
                    }),
                    expression("append_tasks_next", "sequence_append", {
                        "sequence": local("task_sequence"), "value": block_argument("append_tasks_output"),
                        "element": result("append_tasks_item"),
                    }),
                ], result("append_tasks_next")),
            }),
        ],
        result("append_tasks_loop"),
    ))
    functions.append(function(
        "context_all_candidates",
        [("context_all_tasks", "tasks", nominal("task_sequence"))],
        nominal("task_sequence"),
        [
            call("context_all_active", "active_tasks", [parameter("context_all_tasks")]),
            call("context_all_ready", "actionable_tasks", [parameter("context_all_tasks")]),
            call("context_all_result", "append_task_sequences", [
                result("context_all_active"), result("context_all_ready"),
            ]),
        ],
        result("context_all_result"),
    ))
    for name, field, sequence in [
        ("context_note_count", "task_notes_field", "note_sequence"),
        ("context_dependency_count", "task_dependencies_field", "id_sequence"),
    ]:
        functions.append(function(
            name,
            [(f"{name}_tasks", "tasks", nominal("task_sequence"))],
            "i64",
            [
                expression(f"{name}_zero", "const_i64", 0),
                expression(f"{name}_end", "sequence_len", {
                    "sequence": local("task_sequence"), "value": parameter(f"{name}_tasks"),
                }),
                expression(f"{name}_loop", "for_i64", {
                    "start": result(f"{name}_zero"),
                    "end_exclusive": result(f"{name}_end"),
                    "step": 1,
                    "initial": result(f"{name}_zero"),
                    "carried": "i64",
                    "index_symbol": f"{name}_index",
                    "carried_symbol": f"{name}_total",
                    "body": yielding([
                        expression(f"{name}_task", "sequence_get", {
                            "sequence": local("task_sequence"),
                            "value": parameter(f"{name}_tasks"),
                            "index": block_argument(f"{name}_index"),
                        }),
                        expression(f"{name}_values", "project_field", {
                            "value": result(f"{name}_task"), "field": local(field),
                        }),
                        expression(f"{name}_item_count", "sequence_len", {
                            "sequence": local(sequence), "value": result(f"{name}_values"),
                        }),
                        expression(f"{name}_next", "add_i64", {
                            "lhs": block_argument(f"{name}_total"),
                            "rhs": result(f"{name}_item_count"),
                        }),
                    ], result(f"{name}_next")),
                }),
            ],
            result(f"{name}_loop"),
        ))
    functions.append(function(
        "context_candidates",
        [
            ("context_candidate_tasks", "tasks", nominal("task_sequence")),
            ("context_candidate_maximum", "maximum", "i64"),
        ],
        nominal("task_sequence"),
        [
            call("context_candidate_active", "active_tasks", [parameter("context_candidate_tasks")]),
            call("context_candidate_active_sorted", "sort_tasks", [
                result("context_candidate_active"), parameter("context_candidate_maximum"),
            ]),
            call("context_candidate_ready", "actionable_tasks", [parameter("context_candidate_tasks")]),
            call("context_candidate_ready_sorted", "sort_tasks", [
                result("context_candidate_ready"), parameter("context_candidate_maximum"),
            ]),
            call("context_candidate_result", "append_task_sequences", [
                result("context_candidate_active_sorted"), result("context_candidate_ready_sorted"),
            ]),
        ],
        result("context_candidate_result"),
    ))
    functions.append(function(
        "phase_filter_matches",
        [
            ("phase_match_filter", "filter", nominal("phase_filter")),
            ("phase_match_phase", "phase", nominal("task_phase")),
        ],
        "bool",
        [expression("phase_match_result", "match_sum", {
            "scrutinee": parameter("phase_match_filter"),
            "result": "bool",
            "arms": [
                arm("phase_filter_any_variant", yielding([
                    expression("phase_match_any", "const_bool", True),
                ], result("phase_match_any"))),
                *[
                    arm(f"phase_filter_{name}_variant", yielding([
                        call(f"phase_match_{name}", f"phase_is_{name}", [parameter("phase_match_phase")]),
                    ], result(f"phase_match_{name}")))
                    for name in ["planned", "active", "done", "cancelled"]
                ],
            ],
        })],
        result("phase_match_result"),
    ))
    functions.append(function(
        "readiness_filter_matches",
        [
            ("readiness_match_filter", "filter", nominal("readiness_filter")),
            ("readiness_match_ready", "ready", "bool"),
        ],
        "bool",
        [expression("readiness_match_result", "match_sum", {
            "scrutinee": parameter("readiness_match_filter"),
            "result": "bool",
            "arms": [
                arm("readiness_filter_any_variant", yielding([
                    expression("readiness_match_any", "const_bool", True),
                ], result("readiness_match_any"))),
                arm("readiness_filter_ready_variant", yielding([], parameter("readiness_match_ready"))),
                arm("readiness_filter_blocked_variant", yielding([
                    expression("readiness_match_blocked", "not_bool", {
                        "value": parameter("readiness_match_ready"),
                    }),
                ], result("readiness_match_blocked"))),
            ],
        })],
        result("readiness_match_result"),
    ))
    functions.append(function(
        "label_filter_matches",
        [
            ("label_match_filter", "filter", nominal("label_filter")),
            ("label_match_labels", "labels", nominal("text_sequence")),
        ],
        "bool",
        [expression("label_match_result", "match_sum", {
            "scrutinee": parameter("label_match_filter"),
            "result": "bool",
            "arms": [
                arm("label_filter_any_variant", yielding([
                    expression("label_match_any", "const_bool", True),
                ], result("label_match_any"))),
                arm("label_filter_exact_variant", yielding([
                    call("label_match_exact", "text_sequence_contains", [
                        parameter("label_match_labels"), block_argument("label_match_payload"),
                    ]),
                ], result("label_match_exact")), "label_match_payload"),
            ],
        })],
        result("label_match_result"),
    ))
    functions.append(function(
        "archive_filter_matches",
        [
            ("archive_match_filter", "filter", nominal("archive_filter")),
            ("archive_match_archived", "archived", "bool"),
        ],
        "bool",
        [expression("archive_match_result", "match_sum", {
            "scrutinee": parameter("archive_match_filter"),
            "result": "bool",
            "arms": [
                arm("archive_filter_default_variant", yielding([
                    expression("archive_match_default", "not_bool", {
                        "value": parameter("archive_match_archived"),
                    }),
                ], result("archive_match_default"))),
                arm("archive_filter_archived_variant", yielding([], parameter("archive_match_archived"))),
                arm("archive_filter_all_variant", yielding([
                    expression("archive_match_all", "const_bool", True),
                ], result("archive_match_all"))),
            ],
        })],
        result("archive_match_result"),
    ))
    functions.append(function(
        "filter_tasks",
        [
            ("filter_task_values", "tasks", nominal("task_sequence")),
            ("filter_task_request", "request", nominal("list_request")),
        ],
        nominal("task_sequence"),
        [
            expression("filter_task_phase_filter", "project_field", {
                "value": parameter("filter_task_request"), "field": local("list_phase_field"),
            }),
            expression("filter_task_readiness_filter", "project_field", {
                "value": parameter("filter_task_request"), "field": local("list_readiness_field"),
            }),
            expression("filter_task_label_filter", "project_field", {
                "value": parameter("filter_task_request"), "field": local("list_label_field"),
            }),
            expression("filter_task_archive_filter", "project_field", {
                "value": parameter("filter_task_request"), "field": local("list_archive_field"),
            }),
            expression("filter_task_zero", "const_i64", 0),
            expression("filter_task_end", "sequence_len", {
                "sequence": local("task_sequence"), "value": parameter("filter_task_values"),
            }),
            expression("filter_task_empty", "sequence_empty", {"sequence": local("task_sequence")}),
            expression("filter_task_loop", "for_i64", {
                "start": result("filter_task_zero"),
                "end_exclusive": result("filter_task_end"),
                "step": 1,
                "initial": result("filter_task_empty"),
                "carried": nominal("task_sequence"),
                "index_symbol": "filter_task_index",
                "carried_symbol": "filter_task_output",
                "body": yielding([
                    expression("filter_task_value", "sequence_get", {
                        "sequence": local("task_sequence"), "value": parameter("filter_task_values"),
                        "index": block_argument("filter_task_index"),
                    }),
                    expression("filter_task_phase", "project_field", {
                        "value": result("filter_task_value"), "field": local("task_phase_field"),
                    }),
                    expression("filter_task_labels", "project_field", {
                        "value": result("filter_task_value"), "field": local("task_labels_field"),
                    }),
                    expression("filter_task_archived", "project_field", {
                        "value": result("filter_task_value"), "field": local("task_archived_field"),
                    }),
                    call("filter_task_ready", "task_is_ready", [
                        parameter("filter_task_values"), result("filter_task_value"),
                    ]),
                    call("filter_task_phase_matches", "phase_filter_matches", [
                        result("filter_task_phase_filter"), result("filter_task_phase"),
                    ]),
                    call("filter_task_readiness_matches", "readiness_filter_matches", [
                        result("filter_task_readiness_filter"), result("filter_task_ready"),
                    ]),
                    call("filter_task_label_matches", "label_filter_matches", [
                        result("filter_task_label_filter"), result("filter_task_labels"),
                    ]),
                    call("filter_task_archive_matches", "archive_filter_matches", [
                        result("filter_task_archive_filter"), result("filter_task_archived"),
                    ]),
                    expression("filter_task_match_a", "and_bool", {
                        "lhs": result("filter_task_phase_matches"),
                        "rhs": result("filter_task_readiness_matches"),
                    }),
                    expression("filter_task_match_b", "and_bool", {
                        "lhs": result("filter_task_label_matches"),
                        "rhs": result("filter_task_archive_matches"),
                    }),
                    expression("filter_task_match", "and_bool", {
                        "lhs": result("filter_task_match_a"), "rhs": result("filter_task_match_b"),
                    }),
                    expression("filter_task_select", "if", {
                        "condition": result("filter_task_match"),
                        "result": nominal("task_sequence"),
                        "then_body": yielding([
                            expression("filter_task_append", "sequence_append", {
                                "sequence": local("task_sequence"), "value": block_argument("filter_task_output"),
                                "element": result("filter_task_value"),
                            }),
                        ], result("filter_task_append")),
                        "else_body": yielding([], block_argument("filter_task_output")),
                    }),
                ], result("filter_task_select")),
            }),
        ],
        result("filter_task_loop"),
    ))
    functions.append(function(
        "order_tasks",
        [
            ("order_task_order", "order", nominal("task_order")),
            ("order_task_values", "tasks", nominal("task_sequence")),
            ("order_task_maximum", "maximum", "i64"),
        ],
        nominal("task_sequence"),
        [expression("order_task_match", "match_sum", {
            "scrutinee": parameter("order_task_order"),
            "result": nominal("task_sequence"),
            "arms": [
                arm("task_order_id_variant", yielding([], parameter("order_task_values"))),
                arm("task_order_priority_variant", yielding([
                    call("order_task_sorted", "sort_tasks", [
                        parameter("order_task_values"), parameter("order_task_maximum"),
                    ]),
                ], result("order_task_sorted"))),
            ],
        })],
        result("order_task_match"),
    ))
    functions.append(function(
        "page_request_valid",
        [
            ("page_valid_after", "after", "i64"),
            ("page_valid_limit", "limit", "i64"),
            ("page_valid_total", "total", "i64"),
        ],
        "bool",
        [
            expression("page_valid_zero", "const_i64", 0),
            expression("page_valid_max_plus_one", "const_i64", 101),
            expression("page_valid_after_negative", "lt_i64", {
                "lhs": parameter("page_valid_after"), "rhs": result("page_valid_zero"),
            }),
            expression("page_valid_after_nonnegative", "not_bool", {
                "value": result("page_valid_after_negative"),
            }),
            expression("page_valid_limit_positive", "lt_i64", {
                "lhs": result("page_valid_zero"), "rhs": parameter("page_valid_limit"),
            }),
            expression("page_valid_limit_bounded", "lt_i64", {
                "lhs": parameter("page_valid_limit"), "rhs": result("page_valid_max_plus_one"),
            }),
            expression("page_valid_after_excessive", "lt_i64", {
                "lhs": parameter("page_valid_total"), "rhs": parameter("page_valid_after"),
            }),
            expression("page_valid_after_bounded", "not_bool", {
                "value": result("page_valid_after_excessive"),
            }),
            expression("page_valid_a", "and_bool", {
                "lhs": result("page_valid_after_nonnegative"), "rhs": result("page_valid_limit_positive"),
            }),
            expression("page_valid_b", "and_bool", {
                "lhs": result("page_valid_a"), "rhs": result("page_valid_limit_bounded"),
            }),
            expression("page_valid_result", "and_bool", {
                "lhs": result("page_valid_b"), "rhs": result("page_valid_after_bounded"),
            }),
        ],
        result("page_valid_result"),
    ))
    functions.append(function(
        "subtract_nonnegative",
        [
            ("subtract_total", "total", "i64"),
            ("subtract_amount", "amount", "i64"),
        ],
        "i64",
        [
            expression("subtract_zero", "const_i64", 0),
            expression("subtract_negative_one", "const_i64", -1),
            expression("subtract_loop", "for_i64", {
                "start": result("subtract_zero"),
                "end_exclusive": parameter("subtract_amount"),
                "step": 1,
                "initial": parameter("subtract_total"),
                "carried": "i64",
                "index_symbol": "subtract_index",
                "carried_symbol": "subtract_value",
                "body": yielding([
                    expression("subtract_next", "add_i64", {
                        "lhs": block_argument("subtract_value"),
                        "rhs": result("subtract_negative_one"),
                    }),
                ], result("subtract_next")),
            }),
        ],
        result("subtract_loop"),
    ))
    functions.append(function(
        "make_task_page",
        [
            ("make_page_all", "all_tasks", nominal("task_sequence")),
            ("make_page_selected", "selected_tasks", nominal("task_sequence")),
            ("make_page_after", "after", "i64"),
            ("make_page_limit", "limit", "i64"),
            ("make_page_full_total", "total", "i64"),
        ],
        nominal("task_page"),
        [
            expression("make_page_zero", "const_i64", 0),
            expression("make_page_one", "const_i64", 1),
            expression("make_page_selected_total", "sequence_len", {
                "sequence": local("task_sequence"), "value": parameter("make_page_selected"),
            }),
            expression("make_page_empty", "sequence_empty", {"sequence": local("task_view_sequence")}),
            expression("make_page_initial", "construct_product", {
                "product": local("page_build_state"),
                "fields": [
                    {"field": local("page_build_views_field"), "value": result("make_page_empty")},
                    {"field": local("page_build_omitted_field"), "value": result("make_page_zero")},
                ],
            }),
            expression("make_page_loop", "for_i64", {
                "start": result("make_page_zero"),
                "end_exclusive": result("make_page_selected_total"),
                "step": 1,
                "initial": result("make_page_initial"),
                "carried": nominal("page_build_state"),
                "index_symbol": "make_page_index",
                "carried_symbol": "make_page_state",
                "body": yielding([
                    expression("make_page_views", "project_field", {
                        "value": block_argument("make_page_state"), "field": local("page_build_views_field"),
                    }),
                    expression("make_page_omitted", "project_field", {
                        "value": block_argument("make_page_state"), "field": local("page_build_omitted_field"),
                    }),
                    expression("make_page_before", "lt_i64", {
                        "lhs": block_argument("make_page_index"), "rhs": parameter("make_page_after"),
                    }),
                    expression("make_page_view_count", "sequence_len", {
                        "sequence": local("task_view_sequence"), "value": result("make_page_views"),
                    }),
                    expression("make_page_has_space", "lt_i64", {
                        "lhs": result("make_page_view_count"), "rhs": parameter("make_page_limit"),
                    }),
                    expression("make_page_not_before", "not_bool", {"value": result("make_page_before")}),
                    expression("make_page_include", "and_bool", {
                        "lhs": result("make_page_not_before"), "rhs": result("make_page_has_space"),
                    }),
                    expression("make_page_next_state", "if", {
                        "condition": result("make_page_include"),
                        "result": nominal("page_build_state"),
                        "then_body": yielding([
                            expression("make_page_task", "sequence_get", {
                                "sequence": local("task_sequence"), "value": parameter("make_page_selected"),
                                "index": block_argument("make_page_index"),
                            }),
                            call("make_page_view", "make_task_view", [
                                parameter("make_page_all"), result("make_page_task"),
                            ]),
                            expression("make_page_append", "sequence_append", {
                                "sequence": local("task_view_sequence"), "value": result("make_page_views"),
                                "element": result("make_page_view"),
                            }),
                            expression("make_page_included_state", "construct_product", {
                                "product": local("page_build_state"),
                                "fields": [
                                    {"field": local("page_build_views_field"), "value": result("make_page_append")},
                                    {"field": local("page_build_omitted_field"), "value": result("make_page_omitted")},
                                ],
                            }),
                        ], result("make_page_included_state")),
                        "else_body": yielding([
                            expression("make_page_omitted_next", "add_i64", {
                                "lhs": result("make_page_omitted"), "rhs": result("make_page_one"),
                            }),
                            expression("make_page_omitted_state", "construct_product", {
                                "product": local("page_build_state"),
                                "fields": [
                                    {"field": local("page_build_views_field"), "value": result("make_page_views")},
                                    {"field": local("page_build_omitted_field"), "value": result("make_page_omitted_next")},
                                ],
                            }),
                        ], result("make_page_omitted_state")),
                    }),
                ], result("make_page_next_state")),
            }),
            expression("make_page_result_views", "project_field", {
                "value": result("make_page_loop"), "field": local("page_build_views_field"),
            }),
            expression("make_page_result_count", "sequence_len", {
                "sequence": local("task_view_sequence"), "value": result("make_page_result_views"),
            }),
            call("make_page_result_omitted", "subtract_nonnegative", [
                parameter("make_page_full_total"), result("make_page_result_count"),
            ]),
            expression("make_page_next_after", "add_i64", {
                "lhs": parameter("make_page_after"), "rhs": result("make_page_result_count"),
            }),
            expression("make_page_value", "construct_product", {
                "product": local("task_page"),
                "fields": [
                    {"field": local("task_page_tasks_field"), "value": result("make_page_result_views")},
                    {"field": local("task_page_total_field"), "value": parameter("make_page_full_total")},
                    {"field": local("task_page_omitted_field"), "value": result("make_page_result_omitted")},
                    {"field": local("task_page_next_field"), "value": result("make_page_next_after")},
                ],
            }),
        ],
        result("make_page_value"),
    ))
    functions.append(summary_function())
    return functions


def summary_function():
    phase_names = ["planned", "active", "done", "cancelled"]
    operations = [
        expression("summary_zero", "const_i64", 0),
        expression("summary_one", "const_i64", 1),
        expression("summary_end", "sequence_len", {
            "sequence": local("task_sequence"), "value": parameter("summary_tasks"),
        }),
        expression("summary_initial", "construct_product", {
            "product": local("project_summary"),
            "fields": [
                {"field": local(f"summary_{name}_field"), "value": result("summary_zero")}
                for name in phase_names + ["actionable", "archived"]
            ],
        }),
        expression("summary_loop", "for_i64", {
            "start": result("summary_zero"),
            "end_exclusive": result("summary_end"),
            "step": 1,
            "initial": result("summary_initial"),
            "carried": nominal("project_summary"),
            "index_symbol": "summary_index",
            "carried_symbol": "summary_state",
            "body": yielding([
                *[
                    expression(f"summary_current_{name}", "project_field", {
                        "value": block_argument("summary_state"), "field": local(f"summary_{name}_field"),
                    }) for name in phase_names + ["actionable", "archived"]
                ],
                expression("summary_task", "sequence_get", {
                    "sequence": local("task_sequence"), "value": parameter("summary_tasks"),
                    "index": block_argument("summary_index"),
                }),
                expression("summary_phase", "project_field", {
                    "value": result("summary_task"), "field": local("task_phase_field"),
                }),
                *[
                    call(f"summary_is_{name}", f"phase_is_{name}", [result("summary_phase")])
                    for name in phase_names
                ],
                call("summary_is_actionable", "task_is_ready", [parameter("summary_tasks"), result("summary_task")]),
                expression("summary_is_archived", "project_field", {
                    "value": result("summary_task"), "field": local("task_archived_field"),
                }),
                *[
                    expression(f"summary_next_{name}", "if", {
                        "condition": result(f"summary_is_{name}"),
                        "result": "i64",
                        "then_body": yielding([
                            expression(f"summary_increment_{name}", "add_i64", {
                                "lhs": result(f"summary_current_{name}"), "rhs": result("summary_one"),
                            }),
                        ], result(f"summary_increment_{name}")),
                        "else_body": yielding([], result(f"summary_current_{name}")),
                    }) for name in phase_names + ["actionable", "archived"]
                ],
                expression("summary_next", "construct_product", {
                    "product": local("project_summary"),
                    "fields": [
                        {"field": local(f"summary_{name}_field"), "value": result(f"summary_next_{name}")}
                        for name in phase_names + ["actionable", "archived"]
                    ],
                }),
            ], result("summary_next")),
        }),
    ]
    return function(
        "summarize_tasks",
        [("summary_tasks", "tasks", nominal("task_sequence"))],
        nominal("project_summary"),
        operations,
        result("summary_loop"),
    )


def context_budget_functions():
    functions = []
    functions.append(function(
        "text_values_bytes",
        [("text_bytes_values", "values", nominal("text_sequence"))],
        "i64",
        [
            expression("text_bytes_zero", "const_i64", 0),
            expression("text_bytes_end", "sequence_len", {
                "sequence": local("text_sequence"), "value": parameter("text_bytes_values"),
            }),
            expression("text_bytes_loop", "for_i64", {
                "start": result("text_bytes_zero"),
                "end_exclusive": result("text_bytes_end"),
                "step": 1,
                "initial": result("text_bytes_zero"),
                "carried": "i64",
                "index_symbol": "text_bytes_index",
                "carried_symbol": "text_bytes_total",
                "body": yielding([
                    expression("text_bytes_item", "sequence_get", {
                        "sequence": local("text_sequence"), "value": parameter("text_bytes_values"),
                        "index": block_argument("text_bytes_index"),
                    }),
                    expression("text_bytes_length", "text_len", {"value": result("text_bytes_item")}),
                    expression("text_bytes_next", "add_i64", {
                        "lhs": block_argument("text_bytes_total"), "rhs": result("text_bytes_length"),
                    }),
                ], result("text_bytes_next")),
            }),
        ],
        result("text_bytes_loop"),
    ))
    functions.append(function(
        "note_values_bytes",
        [("note_bytes_values", "values", nominal("note_sequence"))],
        "i64",
        [
            expression("note_bytes_zero", "const_i64", 0),
            expression("note_bytes_end", "sequence_len", {
                "sequence": local("note_sequence"), "value": parameter("note_bytes_values"),
            }),
            expression("note_bytes_loop", "for_i64", {
                "start": result("note_bytes_zero"),
                "end_exclusive": result("note_bytes_end"),
                "step": 1,
                "initial": result("note_bytes_zero"),
                "carried": "i64",
                "index_symbol": "note_bytes_index",
                "carried_symbol": "note_bytes_total",
                "body": yielding([
                    expression("note_bytes_item", "sequence_get", {
                        "sequence": local("note_sequence"), "value": parameter("note_bytes_values"),
                        "index": block_argument("note_bytes_index"),
                    }),
                    expression("note_bytes_actor", "project_field", {
                        "value": result("note_bytes_item"), "field": local("note_actor_field"),
                    }),
                    expression("note_bytes_body", "project_field", {
                        "value": result("note_bytes_item"), "field": local("note_body_field"),
                    }),
                    expression("note_bytes_actor_len", "text_len", {"value": result("note_bytes_actor")}),
                    expression("note_bytes_body_len", "text_len", {"value": result("note_bytes_body")}),
                    expression("note_bytes_item_len", "add_i64", {
                        "lhs": result("note_bytes_actor_len"), "rhs": result("note_bytes_body_len"),
                    }),
                    expression("note_bytes_next", "add_i64", {
                        "lhs": block_argument("note_bytes_total"), "rhs": result("note_bytes_item_len"),
                    }),
                ], result("note_bytes_next")),
            }),
        ],
        result("note_bytes_loop"),
    ))
    functions.append(function(
        "attachment_values_bytes",
        [("attachment_bytes_values", "values", nominal("attachment_sequence"))],
        "i64",
        [
            expression("attachment_bytes_zero", "const_i64", 0),
            expression("attachment_bytes_end", "sequence_len", {
                "sequence": local("attachment_sequence"), "value": parameter("attachment_bytes_values"),
            }),
            expression("attachment_bytes_loop", "for_i64", {
                "start": result("attachment_bytes_zero"),
                "end_exclusive": result("attachment_bytes_end"),
                "step": 1,
                "initial": result("attachment_bytes_zero"),
                "carried": "i64",
                "index_symbol": "attachment_bytes_index",
                "carried_symbol": "attachment_bytes_total",
                "body": yielding([
                    expression("attachment_bytes_item", "sequence_get", {
                        "sequence": local("attachment_sequence"), "value": parameter("attachment_bytes_values"),
                        "index": block_argument("attachment_bytes_index"),
                    }),
                    expression("attachment_bytes_name", "project_field", {
                        "value": result("attachment_bytes_item"), "field": local("attachment_name_field"),
                    }),
                    expression("attachment_bytes_actor", "project_field", {
                        "value": result("attachment_bytes_item"), "field": local("attachment_actor_field"),
                    }),
                    expression("attachment_bytes_name_len", "text_len", {"value": result("attachment_bytes_name")}),
                    expression("attachment_bytes_actor_len", "text_len", {"value": result("attachment_bytes_actor")}),
                    expression("attachment_bytes_item_len", "add_i64", {
                        "lhs": result("attachment_bytes_name_len"), "rhs": result("attachment_bytes_actor_len"),
                    }),
                    expression("attachment_bytes_next", "add_i64", {
                        "lhs": block_argument("attachment_bytes_total"), "rhs": result("attachment_bytes_item_len"),
                    }),
                ], result("attachment_bytes_next")),
            }),
        ],
        result("attachment_bytes_loop"),
    ))
    functions.append(function(
        "hold_value_bytes",
        [("hold_bytes_value", "hold", nominal("task_hold"))],
        "i64",
        [expression("hold_bytes_match", "match_sum", {
            "scrutinee": parameter("hold_bytes_value"),
            "result": "i64",
            "arms": [
                arm("hold_none_variant", yielding([
                    expression("hold_bytes_zero", "const_i64", 0),
                ], result("hold_bytes_zero"))),
                arm("hold_manual_variant", yielding([
                    expression("hold_bytes_length", "text_len", {"value": block_argument("hold_bytes_payload")}),
                ], result("hold_bytes_length")), "hold_bytes_payload"),
            ],
        })],
        result("hold_bytes_match"),
    ))
    functions.append(function(
        "task_text_bytes",
        [("task_bytes_value", "task", nominal("task"))],
        "i64",
        [
            *task_fields(parameter("task_bytes_value"), "task_bytes_field"),
            expression("task_bytes_title", "text_len", {"value": result("task_bytes_field_title")}),
            expression("task_bytes_description", "text_len", {"value": result("task_bytes_field_description")}),
            call("task_bytes_hold", "hold_value_bytes", [result("task_bytes_field_hold")]),
            call("task_bytes_labels", "text_values_bytes", [result("task_bytes_field_labels")]),
            call("task_bytes_notes", "note_values_bytes", [result("task_bytes_field_notes")]),
            call("task_bytes_attachments", "attachment_values_bytes", [result("task_bytes_field_attachments")]),
            expression("task_bytes_a", "add_i64", {
                "lhs": result("task_bytes_title"), "rhs": result("task_bytes_description"),
            }),
            expression("task_bytes_b", "add_i64", {
                "lhs": result("task_bytes_a"), "rhs": result("task_bytes_hold"),
            }),
            expression("task_bytes_c", "add_i64", {
                "lhs": result("task_bytes_b"), "rhs": result("task_bytes_labels"),
            }),
            expression("task_bytes_d", "add_i64", {
                "lhs": result("task_bytes_c"), "rhs": result("task_bytes_notes"),
            }),
            expression("task_bytes_result", "add_i64", {
                "lhs": result("task_bytes_d"), "rhs": result("task_bytes_attachments"),
            }),
        ],
        result("task_bytes_result"),
    ))
    functions.append(context_result_function())
    return functions


def context_result_function():
    state_fields = [
        "views", "notes_used", "dependencies_used", "text_used", "text_truncated",
    ]
    return function(
        "make_context_result",
        [
            ("make_context_all", "all_tasks", nominal("task_sequence")),
            ("make_context_full_candidates", "full_candidates", nominal("task_sequence")),
            ("make_context_candidates", "candidates", nominal("task_sequence")),
            ("make_context_max_tasks", "maximum_tasks", "i64"),
            ("make_context_max_notes", "maximum_notes", "i64"),
            ("make_context_max_dependencies", "maximum_dependencies", "i64"),
            ("make_context_max_text", "maximum_text_bytes", "i64"),
        ],
        nominal("context_result"),
        [
            expression("make_context_zero", "const_i64", 0),
            expression("make_context_false", "const_bool", False),
            expression("make_context_empty", "sequence_empty", {"sequence": local("task_view_sequence")}),
            expression("make_context_initial", "construct_product", {
                "product": local("context_build_state"),
                "fields": [
                    {"field": local("context_build_views_field"), "value": result("make_context_empty")},
                    *[
                        {"field": local(f"context_build_{name}_field"), "value": result("make_context_zero")}
                        for name in [
                            "notes_used", "dependencies_used", "text_used",
                        ]
                    ],
                    {"field": local("context_build_text_truncated_field"), "value": result("make_context_false")},
                ],
            }),
            expression("make_context_selected_total", "sequence_len", {
                "sequence": local("task_sequence"), "value": parameter("make_context_candidates"),
            }),
            expression("make_context_total", "sequence_len", {
                "sequence": local("task_sequence"),
                "value": parameter("make_context_full_candidates"),
            }),
            call("make_context_total_notes", "context_note_count", [
                parameter("make_context_full_candidates"),
            ]),
            call("make_context_total_dependencies", "context_dependency_count", [
                parameter("make_context_full_candidates"),
            ]),
            expression("make_context_loop", "for_i64", {
                "start": result("make_context_zero"),
                "end_exclusive": result("make_context_selected_total"),
                "step": 1,
                "initial": result("make_context_initial"),
                "carried": nominal("context_build_state"),
                "index_symbol": "make_context_index",
                "carried_symbol": "make_context_state",
                "body": yielding([
                    *[
                        expression(f"make_context_current_{name}", "project_field", {
                            "value": block_argument("make_context_state"),
                            "field": local(f"context_build_{name}_field"),
                        }) for name in state_fields
                    ],
                    expression("make_context_task", "sequence_get", {
                        "sequence": local("task_sequence"), "value": parameter("make_context_candidates"),
                        "index": block_argument("make_context_index"),
                    }),
                    expression("make_context_notes", "project_field", {
                        "value": result("make_context_task"), "field": local("task_notes_field"),
                    }),
                    expression("make_context_dependencies", "project_field", {
                        "value": result("make_context_task"), "field": local("task_dependencies_field"),
                    }),
                    expression("make_context_note_count", "sequence_len", {
                        "sequence": local("note_sequence"), "value": result("make_context_notes"),
                    }),
                    expression("make_context_dependency_count", "sequence_len", {
                        "sequence": local("id_sequence"), "value": result("make_context_dependencies"),
                    }),
                    call("make_context_task_text", "task_text_bytes", [result("make_context_task")]),
                    expression("make_context_notes_proposed", "add_i64", {
                        "lhs": result("make_context_current_notes_used"), "rhs": result("make_context_note_count"),
                    }),
                    expression("make_context_dependencies_proposed", "add_i64", {
                        "lhs": result("make_context_current_dependencies_used"),
                        "rhs": result("make_context_dependency_count"),
                    }),
                    expression("make_context_text_proposed", "add_i64", {
                        "lhs": result("make_context_current_text_used"), "rhs": result("make_context_task_text"),
                    }),
                    expression("make_context_view_count", "sequence_len", {
                        "sequence": local("task_view_sequence"), "value": result("make_context_current_views"),
                    }),
                    expression("make_context_task_space", "lt_i64", {
                        "lhs": result("make_context_view_count"), "rhs": parameter("make_context_max_tasks"),
                    }),
                    expression("make_context_notes_exceed", "lt_i64", {
                        "lhs": parameter("make_context_max_notes"), "rhs": result("make_context_notes_proposed"),
                    }),
                    expression("make_context_dependencies_exceed", "lt_i64", {
                        "lhs": parameter("make_context_max_dependencies"),
                        "rhs": result("make_context_dependencies_proposed"),
                    }),
                    expression("make_context_text_exceed", "lt_i64", {
                        "lhs": parameter("make_context_max_text"), "rhs": result("make_context_text_proposed"),
                    }),
                    expression("make_context_notes_fit", "not_bool", {"value": result("make_context_notes_exceed")}),
                    expression("make_context_dependencies_fit", "not_bool", {
                        "value": result("make_context_dependencies_exceed"),
                    }),
                    expression("make_context_text_fit", "not_bool", {"value": result("make_context_text_exceed")}),
                    expression("make_context_fit_a", "and_bool", {
                        "lhs": result("make_context_task_space"), "rhs": result("make_context_notes_fit"),
                    }),
                    expression("make_context_fit_b", "and_bool", {
                        "lhs": result("make_context_dependencies_fit"), "rhs": result("make_context_text_fit"),
                    }),
                    expression("make_context_include", "and_bool", {
                        "lhs": result("make_context_fit_a"), "rhs": result("make_context_fit_b"),
                    }),
                    expression("make_context_next_state", "if", {
                        "condition": result("make_context_include"),
                        "result": nominal("context_build_state"),
                        "then_body": yielding([
                            call("make_context_view", "make_task_view", [
                                parameter("make_context_all"), result("make_context_task"),
                            ]),
                            expression("make_context_append", "sequence_append", {
                                "sequence": local("task_view_sequence"),
                                "value": result("make_context_current_views"),
                                "element": result("make_context_view"),
                            }),
                            expression("make_context_included", "construct_product", {
                                "product": local("context_build_state"),
                                "fields": [
                                    {"field": local("context_build_views_field"), "value": result("make_context_append")},
                                    {"field": local("context_build_notes_used_field"), "value": result("make_context_notes_proposed")},
                                    {"field": local("context_build_dependencies_used_field"), "value": result("make_context_dependencies_proposed")},
                                    {"field": local("context_build_text_used_field"), "value": result("make_context_text_proposed")},
                                    {"field": local("context_build_text_truncated_field"), "value": result("make_context_current_text_truncated")},
                                ],
                            }),
                        ], result("make_context_included")),
                        "else_body": yielding([
                            expression("make_context_text_truncated_next", "or_bool", {
                                "lhs": result("make_context_current_text_truncated"),
                                "rhs": result("make_context_text_exceed"),
                            }),
                            expression("make_context_omitted", "construct_product", {
                                "product": local("context_build_state"),
                                "fields": [
                                    {"field": local("context_build_views_field"), "value": result("make_context_current_views")},
                                    {"field": local("context_build_notes_used_field"), "value": result("make_context_current_notes_used")},
                                    {"field": local("context_build_dependencies_used_field"), "value": result("make_context_current_dependencies_used")},
                                    {"field": local("context_build_text_used_field"), "value": result("make_context_current_text_used")},
                                    {"field": local("context_build_text_truncated_field"), "value": result("make_context_text_truncated_next")},
                                ],
                            }),
                        ], result("make_context_omitted")),
                    }),
                ], result("make_context_next_state")),
            }),
            *[
                expression(f"make_context_result_{name}", "project_field", {
                    "value": result("make_context_loop"),
                    "field": local(f"context_build_{name}_field"),
                }) for name in state_fields
            ],
            expression("make_context_result_count", "sequence_len", {
                "sequence": local("task_view_sequence"),
                "value": result("make_context_result_views"),
            }),
            call("make_context_result_tasks_omitted", "subtract_nonnegative", [
                result("make_context_total"), result("make_context_result_count"),
            ]),
            call("make_context_result_notes_omitted", "subtract_nonnegative", [
                result("make_context_total_notes"), result("make_context_result_notes_used"),
            ]),
            call("make_context_result_dependencies_omitted", "subtract_nonnegative", [
                result("make_context_total_dependencies"),
                result("make_context_result_dependencies_used"),
            ]),
            expression("make_context_page", "construct_product", {
                "product": local("task_page"),
                "fields": [
                    {"field": local("task_page_tasks_field"), "value": result("make_context_result_views")},
                    {"field": local("task_page_total_field"), "value": result("make_context_total")},
                    {"field": local("task_page_omitted_field"), "value": result("make_context_result_tasks_omitted")},
                    {"field": local("task_page_next_field"), "value": result("make_context_total")},
                ],
            }),
            expression("make_context_result_value", "construct_product", {
                "product": local("context_result"),
                "fields": [
                    {"field": local("context_page_field"), "value": result("make_context_page")},
                    {"field": local("context_notes_omitted_field"), "value": result("make_context_result_notes_omitted")},
                    {"field": local("context_dependencies_omitted_field"), "value": result("make_context_result_dependencies_omitted")},
                    {"field": local("context_text_truncated_field"), "value": result("make_context_result_text_truncated")},
                ],
            }),
        ],
        result("make_context_result_value"),
    )


def activity_query_functions():
    return [
        function(
            "filter_activities",
            [
                ("filter_activity_values", "activities", nominal("activity_sequence")),
                ("filter_activity_filter", "filter", nominal("activity_filter")),
            ],
            nominal("activity_sequence"),
            [
                expression("filter_activity_zero", "const_i64", 0),
                expression("filter_activity_end", "sequence_len", {
                    "sequence": local("activity_sequence"), "value": parameter("filter_activity_values"),
                }),
                expression("filter_activity_empty", "sequence_empty", {"sequence": local("activity_sequence")}),
                expression("filter_activity_loop", "for_i64", {
                    "start": result("filter_activity_zero"),
                    "end_exclusive": result("filter_activity_end"),
                    "step": 1,
                    "initial": result("filter_activity_empty"),
                    "carried": nominal("activity_sequence"),
                    "index_symbol": "filter_activity_index",
                    "carried_symbol": "filter_activity_output",
                    "body": yielding([
                        expression("filter_activity_value", "sequence_get", {
                            "sequence": local("activity_sequence"), "value": parameter("filter_activity_values"),
                            "index": block_argument("filter_activity_index"),
                        }),
                        expression("filter_activity_task", "project_field", {
                            "value": result("filter_activity_value"), "field": local("activity_task_field"),
                        }),
                        expression("filter_activity_matches", "match_sum", {
                            "scrutinee": parameter("filter_activity_filter"),
                            "result": "bool",
                            "arms": [
                                arm("activity_filter_all_variant", yielding([
                                    expression("filter_activity_all", "const_bool", True),
                                ], result("filter_activity_all"))),
                                arm("activity_filter_task_variant", yielding([
                                    expression("filter_activity_equal", "equal_i64", {
                                        "lhs": result("filter_activity_task"),
                                        "rhs": block_argument("filter_activity_task_payload"),
                                    }),
                                ], result("filter_activity_equal")), "filter_activity_task_payload"),
                            ],
                        }),
                        expression("filter_activity_select", "if", {
                            "condition": result("filter_activity_matches"),
                            "result": nominal("activity_sequence"),
                            "then_body": yielding([
                                expression("filter_activity_append", "sequence_append", {
                                    "sequence": local("activity_sequence"),
                                    "value": block_argument("filter_activity_output"),
                                    "element": result("filter_activity_value"),
                                }),
                            ], result("filter_activity_append")),
                            "else_body": yielding([], block_argument("filter_activity_output")),
                        }),
                    ], result("filter_activity_select")),
                }),
            ],
            result("filter_activity_loop"),
        ),
        function(
            "make_activity_page",
            [
                ("make_activity_values", "activities", nominal("activity_sequence")),
                ("make_activity_after", "after", "i64"),
                ("make_activity_limit", "limit", "i64"),
            ],
            nominal("activity_page"),
            [
                expression("make_activity_zero", "const_i64", 0),
                expression("make_activity_one", "const_i64", 1),
                expression("make_activity_total", "sequence_len", {
                    "sequence": local("activity_sequence"), "value": parameter("make_activity_values"),
                }),
                expression("make_activity_empty", "sequence_empty", {"sequence": local("activity_sequence")}),
                expression("make_activity_initial", "construct_product", {
                    "product": local("activity_page_state"),
                    "fields": [
                        {"field": local("activity_page_state_items_field"), "value": result("make_activity_empty")},
                        {"field": local("activity_page_state_omitted_field"), "value": result("make_activity_zero")},
                    ],
                }),
                expression("make_activity_loop", "for_i64", {
                    "start": result("make_activity_zero"),
                    "end_exclusive": result("make_activity_total"),
                    "step": 1,
                    "initial": result("make_activity_initial"),
                    "carried": nominal("activity_page_state"),
                    "index_symbol": "make_activity_index",
                    "carried_symbol": "make_activity_state",
                    "body": yielding([
                        expression("make_activity_items", "project_field", {
                            "value": block_argument("make_activity_state"),
                            "field": local("activity_page_state_items_field"),
                        }),
                        expression("make_activity_omitted", "project_field", {
                            "value": block_argument("make_activity_state"),
                            "field": local("activity_page_state_omitted_field"),
                        }),
                        expression("make_activity_before", "lt_i64", {
                            "lhs": block_argument("make_activity_index"), "rhs": parameter("make_activity_after"),
                        }),
                        expression("make_activity_count", "sequence_len", {
                            "sequence": local("activity_sequence"), "value": result("make_activity_items"),
                        }),
                        expression("make_activity_space", "lt_i64", {
                            "lhs": result("make_activity_count"), "rhs": parameter("make_activity_limit"),
                        }),
                        expression("make_activity_not_before", "not_bool", {
                            "value": result("make_activity_before"),
                        }),
                        expression("make_activity_include", "and_bool", {
                            "lhs": result("make_activity_not_before"), "rhs": result("make_activity_space"),
                        }),
                        expression("make_activity_next", "if", {
                            "condition": result("make_activity_include"),
                            "result": nominal("activity_page_state"),
                            "then_body": yielding([
                                expression("make_activity_item", "sequence_get", {
                                    "sequence": local("activity_sequence"),
                                    "value": parameter("make_activity_values"),
                                    "index": block_argument("make_activity_index"),
                                }),
                                expression("make_activity_append", "sequence_append", {
                                    "sequence": local("activity_sequence"), "value": result("make_activity_items"),
                                    "element": result("make_activity_item"),
                                }),
                                expression("make_activity_included", "construct_product", {
                                    "product": local("activity_page_state"),
                                    "fields": [
                                        {"field": local("activity_page_state_items_field"), "value": result("make_activity_append")},
                                        {"field": local("activity_page_state_omitted_field"), "value": result("make_activity_omitted")},
                                    ],
                                }),
                            ], result("make_activity_included")),
                            "else_body": yielding([
                                expression("make_activity_omitted_next", "add_i64", {
                                    "lhs": result("make_activity_omitted"), "rhs": result("make_activity_one"),
                                }),
                                expression("make_activity_skipped", "construct_product", {
                                    "product": local("activity_page_state"),
                                    "fields": [
                                        {"field": local("activity_page_state_items_field"), "value": result("make_activity_items")},
                                        {"field": local("activity_page_state_omitted_field"), "value": result("make_activity_omitted_next")},
                                    ],
                                }),
                            ], result("make_activity_skipped")),
                        }),
                    ], result("make_activity_next")),
                }),
                expression("make_activity_result_items", "project_field", {
                    "value": result("make_activity_loop"), "field": local("activity_page_state_items_field"),
                }),
                expression("make_activity_result_omitted", "project_field", {
                    "value": result("make_activity_loop"), "field": local("activity_page_state_omitted_field"),
                }),
                expression("make_activity_result_count", "sequence_len", {
                    "sequence": local("activity_sequence"), "value": result("make_activity_result_items"),
                }),
                expression("make_activity_next_after", "add_i64", {
                    "lhs": parameter("make_activity_after"), "rhs": result("make_activity_result_count"),
                }),
                expression("make_activity_page_value", "construct_product", {
                    "product": local("activity_page"),
                    "fields": [
                        {"field": local("activity_page_items_field"), "value": result("make_activity_result_items")},
                        {"field": local("activity_page_total_field"), "value": result("make_activity_total")},
                        {"field": local("activity_page_omitted_field"), "value": result("make_activity_result_omitted")},
                        {"field": local("activity_page_next_field"), "value": result("make_activity_next_after")},
                    ],
                }),
            ],
            result("make_activity_page_value"),
        ),
    ]


def task_rebuild_call(prefix, fields, changes):
    arguments = []
    for name in TASK_FIELD_ORDER:
        arguments.append(changes.get(name, fields[name]))
    return call(f"{prefix}_rebuilt", "make_task", arguments)


def accept_rebuilt_body(prefix, state_parameter, index, task_id, fields, changes, code):
    return yielding([
        task_rebuild_call(prefix, fields, changes),
        expression(f"{prefix}_actor", "const_text", "user"),
        expression(f"{prefix}_code", "const_text", code),
        call(f"{prefix}_accepted", "accept_task_change", [
            state_parameter, index, result(f"{prefix}_rebuilt"), task_id,
            result(f"{prefix}_actor"), result(f"{prefix}_code"),
        ]),
    ], result(f"{prefix}_accepted"))


def code_decision_body(prefix, function_name, task_id, code):
    return yielding([
        expression(f"{prefix}_code", "const_text", code),
        call(f"{prefix}_decision", function_name, [task_id, result(f"{prefix}_code")]),
    ], result(f"{prefix}_decision"))


def task_handler(name, payload_type, id_expression, action):
    state_parameter = parameter(f"{name}_state")
    payload_parameter = parameter(f"{name}_payload")
    id_operations, task_id = id_expression(name, payload_parameter)
    prefix = name
    fields_prefix = f"{prefix}_field"
    fields = {field: result(f"{fields_prefix}_{field}") for field in TASK_FIELD_ORDER}
    present_body = yielding([
        expression(f"{prefix}_task_value", "sequence_get", {
            "sequence": local("task_sequence"),
            "value": result(f"{prefix}_tasks"),
            "index": result(f"{prefix}_index"),
        }),
        *task_fields(result(f"{prefix}_task_value"), fields_prefix),
        *action(prefix, state_parameter, payload_parameter, task_id, result(f"{prefix}_index"), fields),
    ], result(f"{prefix}_choice"))
    return function(
        name,
        [(f"{name}_state", "state", nominal("project")), (f"{name}_payload", "payload", payload_type)],
        nominal("mutation_decision"),
        [
            expression(f"{prefix}_tasks", "project_field", {"value": state_parameter, "field": local("project_tasks_field")}),
            *id_operations,
            call(f"{prefix}_index", "find_task_index", [result(f"{prefix}_tasks"), task_id]),
            expression(f"{prefix}_zero", "const_i64", 0),
            expression(f"{prefix}_missing", "lt_i64", {"lhs": result(f"{prefix}_index"), "rhs": result(f"{prefix}_zero")}),
            expression(f"{prefix}_action", "if", {
                "condition": result(f"{prefix}_missing"),
                "result": nominal("mutation_decision"),
                "then_body": code_decision_body(f"{prefix}_missing", "decline_code", task_id, "task_not_found"),
                "else_body": present_body,
            }),
        ],
        result(f"{prefix}_action"),
    )


def direct_id(name, payload):
    return ([], payload)


def product_id(field):
    def build(name, payload):
        operation = f"{name}_task_id"
        return ([expression(operation, "project_field", {"value": payload, "field": local(field)})], result(operation))
    return build


def lifecycle_handlers():
    handlers = []

    def start_action(prefix, state, payload, task_id, index, fields):
        return [
            call(f"{prefix}_planned", "phase_is_planned", [fields["phase"]]),
            call(f"{prefix}_ready", "task_is_ready", [result(f"{prefix}_tasks"), result(f"{prefix}_task_value")]),
            expression(f"{prefix}_choice", "if", {
                "condition": result(f"{prefix}_ready"),
                "result": nominal("mutation_decision"),
                "then_body": accept_rebuilt_body(
                    f"{prefix}_accept", state, index, task_id, fields,
                    {"phase": result(f"{prefix}_active_phase")}, "task_started",
                ),
                "else_body": yielding([
                    expression(f"{prefix}_blocked_choice", "if", {
                        "condition": result(f"{prefix}_planned"),
                        "result": nominal("mutation_decision"),
                        "then_body": code_decision_body(f"{prefix}_blocked", "decline_code", task_id, "task_blocked"),
                        "else_body": code_decision_body(f"{prefix}_wrong_phase", "decline_code", task_id, "start_requires_planned"),
                    }),
                ], result(f"{prefix}_blocked_choice")),
            }),
        ]

    # Phase constants are constructed outside the conditional so every branch can capture them.
    def start_action_with_phase(prefix, state, payload, task_id, index, fields):
        return [
            expression(f"{prefix}_active_phase", "construct_variant", {"variant": local("phase_active_variant")}),
            *start_action(prefix, state, payload, task_id, index, fields),
        ]

    handlers.append(task_handler("handle_start_task", "i64", direct_id, start_action_with_phase))

    def simple_phase_action(required, replacement, accepted_code, wrong_code, unchanged_phase=None):
        def build(prefix, state, payload, task_id, index, fields):
            operations = [
                call(f"{prefix}_required", required, [fields["phase"]]),
                expression(f"{prefix}_replacement", "construct_variant", {"variant": local(replacement)}),
            ]
            if unchanged_phase is not None:
                operations.append(call(f"{prefix}_unchanged_phase", unchanged_phase, [fields["phase"]]))
                fallback = yielding([
                    expression(f"{prefix}_fallback", "if", {
                        "condition": result(f"{prefix}_unchanged_phase"),
                        "result": nominal("mutation_decision"),
                        "then_body": code_decision_body(f"{prefix}_same", "unchanged_code", task_id, f"{accepted_code}_unchanged"),
                        "else_body": code_decision_body(f"{prefix}_invalid", "decline_code", task_id, wrong_code),
                    }),
                ], result(f"{prefix}_fallback"))
            else:
                fallback = code_decision_body(f"{prefix}_invalid", "decline_code", task_id, wrong_code)
            operations.append(expression(f"{prefix}_choice", "if", {
                "condition": result(f"{prefix}_required"),
                "result": nominal("mutation_decision"),
                "then_body": accept_rebuilt_body(
                    f"{prefix}_accept", state, index, task_id, fields,
                    {"phase": result(f"{prefix}_replacement")}, accepted_code,
                ),
                "else_body": fallback,
            }))
            return operations
        return build

    handlers.append(task_handler(
        "handle_stop_task", "i64", direct_id,
        simple_phase_action("phase_is_active", "phase_planned_variant", "task_stopped", "stop_requires_active", "phase_is_planned"),
    ))
    def complete_action(prefix, state, payload, task_id, index, fields):
        return [
            call(f"{prefix}_active", "phase_is_active", [fields["phase"]]),
            call(f"{prefix}_done", "phase_is_done", [fields["phase"]]),
            call(f"{prefix}_unheld", "hold_is_none", [fields["hold"]]),
            call(f"{prefix}_dependencies_done", "dependencies_are_done", [result(f"{prefix}_tasks"), fields["dependencies"]]),
            expression(f"{prefix}_eligible", "and_bool", {
                "lhs": result(f"{prefix}_active"), "rhs": result(f"{prefix}_unheld"),
            }),
            expression(f"{prefix}_eligible_all", "and_bool", {
                "lhs": result(f"{prefix}_eligible"), "rhs": result(f"{prefix}_dependencies_done"),
            }),
            expression(f"{prefix}_replacement", "construct_variant", {"variant": local("phase_done_variant")}),
            expression(f"{prefix}_choice", "if", {
                "condition": result(f"{prefix}_eligible_all"),
                "result": nominal("mutation_decision"),
                "then_body": accept_rebuilt_body(
                    f"{prefix}_accept", state, index, task_id, fields,
                    {"phase": result(f"{prefix}_replacement")}, "task_completed",
                ),
                "else_body": yielding([
                    expression(f"{prefix}_done_choice", "if", {
                        "condition": result(f"{prefix}_done"),
                        "result": nominal("mutation_decision"),
                        "then_body": code_decision_body(f"{prefix}_same", "unchanged_code", task_id, "task_completed_unchanged"),
                        "else_body": code_decision_body(f"{prefix}_invalid", "decline_code", task_id, "complete_requires_eligible_active"),
                    }),
                ], result(f"{prefix}_done_choice")),
            }),
        ]

    handlers.append(task_handler("handle_complete_task", "i64", direct_id, complete_action))

    def cancel_action(prefix, state, payload, task_id, index, fields):
        return [
            call(f"{prefix}_planned", "phase_is_planned", [fields["phase"]]),
            call(f"{prefix}_active", "phase_is_active", [fields["phase"]]),
            expression(f"{prefix}_allowed", "or_bool", {"lhs": result(f"{prefix}_planned"), "rhs": result(f"{prefix}_active")}),
            call(f"{prefix}_cancelled", "phase_is_cancelled", [fields["phase"]]),
            expression(f"{prefix}_replacement", "construct_variant", {"variant": local("phase_cancelled_variant")}),
            expression(f"{prefix}_choice", "if", {
                "condition": result(f"{prefix}_allowed"),
                "result": nominal("mutation_decision"),
                "then_body": accept_rebuilt_body(
                    f"{prefix}_accept", state, index, task_id, fields,
                    {"phase": result(f"{prefix}_replacement")}, "task_cancelled",
                ),
                "else_body": yielding([
                    expression(f"{prefix}_fallback", "if", {
                        "condition": result(f"{prefix}_cancelled"),
                        "result": nominal("mutation_decision"),
                        "then_body": code_decision_body(f"{prefix}_same", "unchanged_code", task_id, "task_already_cancelled"),
                        "else_body": code_decision_body(f"{prefix}_invalid", "decline_code", task_id, "cancel_requires_nonterminal"),
                    }),
                ], result(f"{prefix}_fallback")),
            }),
        ]

    handlers.append(task_handler("handle_cancel_task", "i64", direct_id, cancel_action))

    def reopen_action(prefix, state, payload, task_id, index, fields):
        return [
            call(f"{prefix}_terminal", "phase_is_terminal", [fields["phase"]]),
            expression(f"{prefix}_not_archived", "not_bool", {"value": fields["archived"]}),
            expression(f"{prefix}_allowed", "and_bool", {"lhs": result(f"{prefix}_terminal"), "rhs": result(f"{prefix}_not_archived")}),
            expression(f"{prefix}_replacement", "construct_variant", {"variant": local("phase_planned_variant")}),
            expression(f"{prefix}_choice", "if", {
                "condition": result(f"{prefix}_allowed"),
                "result": nominal("mutation_decision"),
                "then_body": accept_rebuilt_body(
                    f"{prefix}_accept", state, index, task_id, fields,
                    {"phase": result(f"{prefix}_replacement")}, "task_reopened",
                ),
                "else_body": code_decision_body(f"{prefix}_invalid", "decline_code", task_id, "reopen_requires_unarchived_terminal"),
            }),
        ]

    handlers.append(task_handler("handle_reopen_task", "i64", direct_id, reopen_action))

    def priority_action(prefix, state, payload, task_id, index, fields):
        return [
            expression(f"{prefix}_value", "project_field", {"value": payload, "field": local("priority_value_field")}),
            expression(f"{prefix}_equal", "equal_i64", {"lhs": fields["priority"], "rhs": result(f"{prefix}_value")}),
            expression(f"{prefix}_choice", "if", {
                "condition": result(f"{prefix}_equal"),
                "result": nominal("mutation_decision"),
                "then_body": code_decision_body(f"{prefix}_same", "unchanged_code", task_id, "priority_unchanged"),
                "else_body": accept_rebuilt_body(
                    f"{prefix}_accept", state, index, task_id, fields,
                    {"priority": result(f"{prefix}_value")}, "priority_set",
                ),
            }),
        ]

    handlers.append(task_handler(
        "handle_set_priority", nominal("priority_input"), product_id("priority_task_field"), priority_action,
    ))
    return handlers


def collection_transform_functions():
    functions = []
    for name, sequence, element_type, equal_operation in [
        ("remove_text_value", "text_sequence", "text", "text_equal"),
        ("remove_id_value", "id_sequence", "i64", "equal_i64"),
    ]:
        functions.append(function(
            name,
            [(f"{name}_values", "values", nominal(sequence)), (f"{name}_target", "target", element_type)],
            nominal(sequence),
            [
                expression(f"{name}_start", "const_i64", 0),
                expression(f"{name}_end", "sequence_len", {"sequence": local(sequence), "value": parameter(f"{name}_values")}),
                expression(f"{name}_initial", "sequence_empty", {"sequence": local(sequence)}),
                expression(f"{name}_loop", "for_i64", {
                    "start": result(f"{name}_start"),
                    "end_exclusive": result(f"{name}_end"),
                    "step": 1,
                    "initial": result(f"{name}_initial"),
                    "carried": nominal(sequence),
                    "index_symbol": f"{name}_index",
                    "carried_symbol": f"{name}_output",
                    "body": yielding([
                        expression(f"{name}_item", "sequence_get", {
                            "sequence": local(sequence), "value": parameter(f"{name}_values"),
                            "index": block_argument(f"{name}_index"),
                        }),
                        expression(f"{name}_equal", equal_operation, {
                            "lhs": result(f"{name}_item"), "rhs": parameter(f"{name}_target"),
                        }),
                        expression(f"{name}_select", "if", {
                            "condition": result(f"{name}_equal"),
                            "result": nominal(sequence),
                            "then_body": yielding([], block_argument(f"{name}_output")),
                            "else_body": yielding([
                                expression(f"{name}_append", "sequence_append", {
                                    "sequence": local(sequence), "value": block_argument(f"{name}_output"),
                                    "element": result(f"{name}_item"),
                                }),
                            ], result(f"{name}_append")),
                        }),
                    ], result(f"{name}_select")),
                }),
            ],
            result(f"{name}_loop"),
        ))
    functions.append(function(
        "hold_reason_equal",
        [("hold_equal_hold", "hold", nominal("task_hold")), ("hold_equal_reason", "reason", "text")],
        "bool",
        [expression("hold_equal_match", "match_sum", {
            "scrutinee": parameter("hold_equal_hold"),
            "result": "bool",
            "arms": [
                arm("hold_none_variant", yielding([
                    expression("hold_equal_none", "const_bool", False),
                ], result("hold_equal_none"))),
                arm("hold_manual_variant", yielding([
                    expression("hold_equal_manual", "text_equal", {
                        "lhs": block_argument("hold_equal_payload"), "rhs": parameter("hold_equal_reason"),
                    }),
                ], result("hold_equal_manual")), "hold_equal_payload"),
            ],
        })],
        result("hold_equal_match"),
    ))
    return functions


def relation_and_annotation_handlers():
    handlers = []

    def hold_action(prefix, state, payload, task_id, index, fields):
        return [
            expression(f"{prefix}_reason", "project_field", {"value": payload, "field": local("hold_reason_field")}),
            expression(f"{prefix}_reason_len", "text_len", {"value": result(f"{prefix}_reason")}),
            expression(f"{prefix}_reason_valid", "lt_i64", {"lhs": result(f"{prefix}_zero"), "rhs": result(f"{prefix}_reason_len")}),
            call(f"{prefix}_done", "phase_is_done", [fields["phase"]]),
            expression(f"{prefix}_not_done", "not_bool", {"value": result(f"{prefix}_done")}),
            expression(f"{prefix}_not_archived", "not_bool", {"value": fields["archived"]}),
            expression(f"{prefix}_eligible_a", "and_bool", {"lhs": result(f"{prefix}_reason_valid"), "rhs": result(f"{prefix}_not_done")}),
            expression(f"{prefix}_eligible", "and_bool", {"lhs": result(f"{prefix}_eligible_a"), "rhs": result(f"{prefix}_not_archived")}),
            call(f"{prefix}_same", "hold_reason_equal", [fields["hold"], result(f"{prefix}_reason")]),
            expression(f"{prefix}_replacement", "construct_variant", {
                "variant": local("hold_manual_variant"), "payload": result(f"{prefix}_reason"),
            }),
            expression(f"{prefix}_choice", "if", {
                "condition": result(f"{prefix}_eligible"),
                "result": nominal("mutation_decision"),
                "then_body": yielding([
                    expression(f"{prefix}_eligible_choice", "if", {
                        "condition": result(f"{prefix}_same"),
                        "result": nominal("mutation_decision"),
                        "then_body": code_decision_body(f"{prefix}_same_hold", "unchanged_code", task_id, "hold_unchanged"),
                        "else_body": accept_rebuilt_body(
                            f"{prefix}_accept", state, index, task_id, fields,
                            {"hold": result(f"{prefix}_replacement")}, "task_held",
                        ),
                    }),
                ], result(f"{prefix}_eligible_choice")),
                "else_body": code_decision_body(f"{prefix}_invalid", "decline_code", task_id, "hold_requires_reason_and_open_task"),
            }),
        ]

    handlers.append(task_handler("handle_hold_task", nominal("hold_input"), product_id("hold_task_field"), hold_action))

    def release_action(prefix, state, payload, task_id, index, fields):
        return [
            call(f"{prefix}_none", "hold_is_none", [fields["hold"]]),
            expression(f"{prefix}_replacement", "construct_variant", {"variant": local("hold_none_variant")}),
            expression(f"{prefix}_choice", "if", {
                "condition": result(f"{prefix}_none"),
                "result": nominal("mutation_decision"),
                "then_body": code_decision_body(f"{prefix}_same", "unchanged_code", task_id, "hold_absent"),
                "else_body": accept_rebuilt_body(
                    f"{prefix}_accept", state, index, task_id, fields,
                    {"hold": result(f"{prefix}_replacement")}, "task_released",
                ),
            }),
        ]

    handlers.append(task_handler("handle_release_task", "i64", direct_id, release_action))

    def archive_action(prefix, state, payload, task_id, index, fields):
        return [
            call(f"{prefix}_terminal", "phase_is_terminal", [fields["phase"]]),
            expression(f"{prefix}_true", "const_bool", True),
            expression(f"{prefix}_choice", "if", {
                "condition": fields["archived"],
                "result": nominal("mutation_decision"),
                "then_body": code_decision_body(f"{prefix}_same", "unchanged_code", task_id, "task_already_archived"),
                "else_body": yielding([
                    expression(f"{prefix}_terminal_choice", "if", {
                        "condition": result(f"{prefix}_terminal"),
                        "result": nominal("mutation_decision"),
                        "then_body": accept_rebuilt_body(
                            f"{prefix}_accept", state, index, task_id, fields,
                            {"archived": result(f"{prefix}_true")}, "task_archived",
                        ),
                        "else_body": code_decision_body(f"{prefix}_invalid", "decline_code", task_id, "archive_requires_terminal"),
                    }),
                ], result(f"{prefix}_terminal_choice")),
            }),
        ]

    handlers.append(task_handler("handle_archive_task", "i64", direct_id, archive_action))

    def unarchive_action(prefix, state, payload, task_id, index, fields):
        return [
            expression(f"{prefix}_false", "const_bool", False),
            expression(f"{prefix}_choice", "if", {
                "condition": fields["archived"],
                "result": nominal("mutation_decision"),
                "then_body": accept_rebuilt_body(
                    f"{prefix}_accept", state, index, task_id, fields,
                    {"archived": result(f"{prefix}_false")}, "task_unarchived",
                ),
                "else_body": code_decision_body(f"{prefix}_same", "unchanged_code", task_id, "task_not_archived"),
            }),
        ]

    handlers.append(task_handler("handle_unarchive_task", "i64", direct_id, unarchive_action))

    def label_action(add):
        def build(prefix, state, payload, task_id, index, fields):
            operations = [
                expression(f"{prefix}_label", "project_field", {"value": payload, "field": local("label_value_field")}),
                expression(f"{prefix}_label_len", "text_len", {"value": result(f"{prefix}_label")}),
                expression(f"{prefix}_valid", "lt_i64", {"lhs": result(f"{prefix}_zero"), "rhs": result(f"{prefix}_label_len")}),
                call(f"{prefix}_present", "text_sequence_contains", [fields["labels"], result(f"{prefix}_label")]),
            ]
            if add:
                operations.append(expression(f"{prefix}_updated", "sequence_append", {
                    "sequence": local("text_sequence"), "value": fields["labels"], "element": result(f"{prefix}_label"),
                }))
                eligible = result(f"{prefix}_present")
                same_body = code_decision_body(f"{prefix}_same", "unchanged_code", task_id, "label_already_present")
                change_body = accept_rebuilt_body(
                    f"{prefix}_accept", state, index, task_id, fields,
                    {"labels": result(f"{prefix}_updated")}, "label_added",
                )
            else:
                operations.append(call(f"{prefix}_updated", "remove_text_value", [fields["labels"], result(f"{prefix}_label")]))
                eligible = result(f"{prefix}_present")
                same_body = accept_rebuilt_body(
                    f"{prefix}_accept", state, index, task_id, fields,
                    {"labels": result(f"{prefix}_updated")}, "label_removed",
                )
                change_body = code_decision_body(f"{prefix}_absent", "unchanged_code", task_id, "label_absent")
            operations.append(expression(f"{prefix}_choice", "if", {
                "condition": result(f"{prefix}_valid"),
                "result": nominal("mutation_decision"),
                "then_body": yielding([
                    expression(f"{prefix}_presence_choice", "if", {
                        "condition": eligible,
                        "result": nominal("mutation_decision"),
                        "then_body": same_body,
                        "else_body": change_body,
                    }),
                ], result(f"{prefix}_presence_choice")),
                "else_body": code_decision_body(f"{prefix}_invalid", "decline_code", task_id, "label_empty"),
            }))
            return operations
        return build

    handlers.append(task_handler("handle_add_label", nominal("label_input"), product_id("label_task_field"), label_action(True)))
    handlers.append(task_handler("handle_remove_label", nominal("label_input"), product_id("label_task_field"), label_action(False)))

    def dependency_action(add):
        def build(prefix, state, payload, task_id, index, fields):
            operations = [
                expression(f"{prefix}_on", "project_field", {"value": payload, "field": local("dependency_on_field")}),
                expression(f"{prefix}_self", "equal_i64", {"lhs": task_id, "rhs": result(f"{prefix}_on")}),
                call(f"{prefix}_present", "id_sequence_contains", [fields["dependencies"], result(f"{prefix}_on")]),
            ]
            if add:
                operations.extend([
                    call(f"{prefix}_on_index", "find_task_index", [result(f"{prefix}_tasks"), result(f"{prefix}_on")]),
                    expression(f"{prefix}_on_missing", "lt_i64", {"lhs": result(f"{prefix}_on_index"), "rhs": result(f"{prefix}_zero")}),
                    call(f"{prefix}_cycle", "dependency_reaches", [
                        result(f"{prefix}_tasks"), result(f"{prefix}_on"), task_id,
                    ]),
                    expression(f"{prefix}_not_self", "not_bool", {"value": result(f"{prefix}_self")}),
                    expression(f"{prefix}_not_missing", "not_bool", {"value": result(f"{prefix}_on_missing")}),
                    expression(f"{prefix}_not_cycle", "not_bool", {"value": result(f"{prefix}_cycle")}),
                    expression(f"{prefix}_valid_a", "and_bool", {"lhs": result(f"{prefix}_not_self"), "rhs": result(f"{prefix}_not_missing")}),
                    expression(f"{prefix}_valid", "and_bool", {"lhs": result(f"{prefix}_valid_a"), "rhs": result(f"{prefix}_not_cycle")}),
                    expression(f"{prefix}_updated", "sequence_append", {
                        "sequence": local("id_sequence"), "value": fields["dependencies"], "element": result(f"{prefix}_on"),
                    }),
                    expression(f"{prefix}_choice", "if", {
                        "condition": result(f"{prefix}_present"),
                        "result": nominal("mutation_decision"),
                        "then_body": code_decision_body(f"{prefix}_same", "unchanged_code", task_id, "dependency_already_present"),
                        "else_body": yielding([
                            expression(f"{prefix}_valid_choice", "if", {
                                "condition": result(f"{prefix}_valid"),
                                "result": nominal("mutation_decision"),
                                "then_body": accept_rebuilt_body(
                                    f"{prefix}_accept", state, index, task_id, fields,
                                    {"dependencies": result(f"{prefix}_updated")}, "dependency_added",
                                ),
                                "else_body": yielding([
                                    expression(f"{prefix}_invalid_reason", "if", {
                                        "condition": result(f"{prefix}_self"),
                                        "result": nominal("mutation_decision"),
                                        "then_body": code_decision_body(
                                            f"{prefix}_invalid_self", "decline_code", task_id,
                                            "dependency_self",
                                        ),
                                        "else_body": yielding([
                                            expression(f"{prefix}_invalid_nonself", "if", {
                                                "condition": result(f"{prefix}_on_missing"),
                                                "result": nominal("mutation_decision"),
                                                "then_body": code_decision_body(
                                                    f"{prefix}_invalid_missing", "decline_code",
                                                    task_id, "dependency_not_found",
                                                ),
                                                "else_body": code_decision_body(
                                                    f"{prefix}_invalid_cycle", "decline_code",
                                                    task_id, "dependency_cycle",
                                                ),
                                            }),
                                        ], result(f"{prefix}_invalid_nonself")),
                                    }),
                                ], result(f"{prefix}_invalid_reason")),
                            }),
                        ], result(f"{prefix}_valid_choice")),
                    }),
                ])
            else:
                operations.extend([
                    call(f"{prefix}_updated", "remove_id_value", [fields["dependencies"], result(f"{prefix}_on")]),
                    expression(f"{prefix}_choice", "if", {
                        "condition": result(f"{prefix}_present"),
                        "result": nominal("mutation_decision"),
                        "then_body": accept_rebuilt_body(
                            f"{prefix}_accept", state, index, task_id, fields,
                            {"dependencies": result(f"{prefix}_updated")}, "dependency_removed",
                        ),
                        "else_body": code_decision_body(f"{prefix}_absent", "unchanged_code", task_id, "dependency_absent"),
                    }),
                ])
            return operations
        return build

    handlers.append(task_handler(
        "handle_add_dependency", nominal("dependency_input"), product_id("dependency_task_field"), dependency_action(True),
    ))
    handlers.append(task_handler(
        "handle_remove_dependency", nominal("dependency_input"), product_id("dependency_task_field"), dependency_action(False),
    ))
    return handlers


def edit_and_note_handlers():
    handlers = []

    def edit_action(prefix, state, payload, task_id, index, fields):
        return [
            expression(f"{prefix}_set_title", "project_field", {
                "value": payload, "field": local("edit_set_title_field"),
            }),
            expression(f"{prefix}_title", "project_field", {
                "value": payload, "field": local("edit_title_field"),
            }),
            expression(f"{prefix}_set_description", "project_field", {
                "value": payload, "field": local("edit_set_description_field"),
            }),
            expression(f"{prefix}_description", "project_field", {
                "value": payload, "field": local("edit_description_field"),
            }),
            expression(f"{prefix}_set_priority", "project_field", {
                "value": payload, "field": local("edit_set_priority_field"),
            }),
            expression(f"{prefix}_priority", "project_field", {
                "value": payload, "field": local("edit_priority_field"),
            }),
            expression(f"{prefix}_title_len", "text_len", {"value": result(f"{prefix}_title")}),
            expression(f"{prefix}_title_nonempty", "lt_i64", {
                "lhs": result(f"{prefix}_zero"), "rhs": result(f"{prefix}_title_len"),
            }),
            expression(f"{prefix}_not_set_title", "not_bool", {"value": result(f"{prefix}_set_title")}),
            expression(f"{prefix}_title_valid", "or_bool", {
                "lhs": result(f"{prefix}_not_set_title"), "rhs": result(f"{prefix}_title_nonempty"),
            }),
            expression(f"{prefix}_any_a", "or_bool", {
                "lhs": result(f"{prefix}_set_title"), "rhs": result(f"{prefix}_set_description"),
            }),
            expression(f"{prefix}_any", "or_bool", {
                "lhs": result(f"{prefix}_any_a"), "rhs": result(f"{prefix}_set_priority"),
            }),
            expression(f"{prefix}_title_equal", "text_equal", {
                "lhs": fields["title"], "rhs": result(f"{prefix}_title"),
            }),
            expression(f"{prefix}_title_different", "not_bool", {"value": result(f"{prefix}_title_equal")}),
            expression(f"{prefix}_title_changed", "and_bool", {
                "lhs": result(f"{prefix}_set_title"), "rhs": result(f"{prefix}_title_different"),
            }),
            expression(f"{prefix}_description_equal", "text_equal", {
                "lhs": fields["description"], "rhs": result(f"{prefix}_description"),
            }),
            expression(f"{prefix}_description_different", "not_bool", {
                "value": result(f"{prefix}_description_equal"),
            }),
            expression(f"{prefix}_description_changed", "and_bool", {
                "lhs": result(f"{prefix}_set_description"), "rhs": result(f"{prefix}_description_different"),
            }),
            expression(f"{prefix}_priority_equal", "equal_i64", {
                "lhs": fields["priority"], "rhs": result(f"{prefix}_priority"),
            }),
            expression(f"{prefix}_priority_different", "not_bool", {
                "value": result(f"{prefix}_priority_equal"),
            }),
            expression(f"{prefix}_priority_changed", "and_bool", {
                "lhs": result(f"{prefix}_set_priority"), "rhs": result(f"{prefix}_priority_different"),
            }),
            expression(f"{prefix}_changed_a", "or_bool", {
                "lhs": result(f"{prefix}_title_changed"), "rhs": result(f"{prefix}_description_changed"),
            }),
            expression(f"{prefix}_changed", "or_bool", {
                "lhs": result(f"{prefix}_changed_a"), "rhs": result(f"{prefix}_priority_changed"),
            }),
            expression(f"{prefix}_selected_title", "if", {
                "condition": result(f"{prefix}_set_title"),
                "result": "text",
                "then_body": yielding([], result(f"{prefix}_title")),
                "else_body": yielding([], fields["title"]),
            }),
            expression(f"{prefix}_selected_description", "if", {
                "condition": result(f"{prefix}_set_description"),
                "result": "text",
                "then_body": yielding([], result(f"{prefix}_description")),
                "else_body": yielding([], fields["description"]),
            }),
            expression(f"{prefix}_selected_priority", "if", {
                "condition": result(f"{prefix}_set_priority"),
                "result": "i64",
                "then_body": yielding([], result(f"{prefix}_priority")),
                "else_body": yielding([], fields["priority"]),
            }),
            expression(f"{prefix}_choice", "if", {
                "condition": result(f"{prefix}_title_valid"),
                "result": nominal("mutation_decision"),
                "then_body": yielding([
                    expression(f"{prefix}_patch_choice", "if", {
                        "condition": result(f"{prefix}_any"),
                        "result": nominal("mutation_decision"),
                        "then_body": yielding([
                            expression(f"{prefix}_changed_choice", "if", {
                                "condition": result(f"{prefix}_changed"),
                                "result": nominal("mutation_decision"),
                                "then_body": accept_rebuilt_body(
                                    f"{prefix}_accept", state, index, task_id, fields,
                                    {
                                        "title": result(f"{prefix}_selected_title"),
                                        "description": result(f"{prefix}_selected_description"),
                                        "priority": result(f"{prefix}_selected_priority"),
                                    },
                                    "task_edited",
                                ),
                                "else_body": code_decision_body(
                                    f"{prefix}_same", "unchanged_code", task_id, "task_unchanged",
                                ),
                            }),
                        ], result(f"{prefix}_changed_choice")),
                        "else_body": code_decision_body(
                            f"{prefix}_empty", "decline_code", task_id, "edit_patch_empty",
                        ),
                    }),
                ], result(f"{prefix}_patch_choice")),
                "else_body": code_decision_body(
                    f"{prefix}_invalid", "decline_code", task_id, "title_empty",
                ),
            }),
        ]

    handlers.append(task_handler(
        "handle_edit_task", nominal("edit_task_input"), product_id("edit_task_field"), edit_action,
    ))

    def note_action(prefix, state, payload, task_id, index, fields):
        return [
            expression(f"{prefix}_actor", "project_field", {
                "value": payload, "field": local("note_input_actor_field"),
            }),
            expression(f"{prefix}_body", "project_field", {
                "value": payload, "field": local("note_input_body_field"),
            }),
            expression(f"{prefix}_actor_len", "text_len", {"value": result(f"{prefix}_actor")}),
            expression(f"{prefix}_body_len", "text_len", {"value": result(f"{prefix}_body")}),
            expression(f"{prefix}_actor_valid", "lt_i64", {
                "lhs": result(f"{prefix}_zero"), "rhs": result(f"{prefix}_actor_len"),
            }),
            expression(f"{prefix}_body_valid", "lt_i64", {
                "lhs": result(f"{prefix}_zero"), "rhs": result(f"{prefix}_body_len"),
            }),
            expression(f"{prefix}_valid", "and_bool", {
                "lhs": result(f"{prefix}_actor_valid"), "rhs": result(f"{prefix}_body_valid"),
            }),
            expression(f"{prefix}_choice", "if", {
                "condition": result(f"{prefix}_valid"),
                "result": nominal("mutation_decision"),
                "then_body": yielding([
                    expression(f"{prefix}_next_note", "project_field", {
                        "value": state, "field": local("project_next_note_field"),
                    }),
                    expression(f"{prefix}_note", "construct_product", {
                        "product": local("note"),
                        "fields": [
                            {"field": local("note_id_field"), "value": result(f"{prefix}_next_note")},
                            {"field": local("note_actor_field"), "value": result(f"{prefix}_actor")},
                            {"field": local("note_body_field"), "value": result(f"{prefix}_body")},
                        ],
                    }),
                    expression(f"{prefix}_notes", "sequence_append", {
                        "sequence": local("note_sequence"),
                        "value": fields["notes"],
                        "element": result(f"{prefix}_note"),
                    }),
                    task_rebuild_call(f"{prefix}_task", fields, {"notes": result(f"{prefix}_notes")}),
                    expression(f"{prefix}_code", "const_text", "note_added"),
                    call(f"{prefix}_updated", "replace_project_task", [
                        state, index, result(f"{prefix}_task_rebuilt"),
                        result(f"{prefix}_actor"), result(f"{prefix}_code"),
                    ]),
                    *project_fields(result(f"{prefix}_updated"), f"{prefix}_project"),
                    expression(f"{prefix}_one", "const_i64", 1),
                    expression(f"{prefix}_next", "add_i64", {
                        "lhs": result(f"{prefix}_project_next_note"), "rhs": result(f"{prefix}_one"),
                    }),
                    construct_project(
                        f"{prefix}_project_value",
                        prefix,
                        result(f"{prefix}_project_name"),
                        result(f"{prefix}_project_next_task"),
                        result(f"{prefix}_next"),
                        result(f"{prefix}_project_tasks"),
                        result(f"{prefix}_project_activity"),
                        result(f"{prefix}_project_pending"),
                    ),
                    call(f"{prefix}_response", "make_response_accepted", [task_id, result(f"{prefix}_code")]),
                    call(f"{prefix}_completed", "make_completed", [
                        result(f"{prefix}_project_value"), result(f"{prefix}_response"),
                    ]),
                ], result(f"{prefix}_completed")),
                "else_body": code_decision_body(
                    f"{prefix}_invalid", "decline_code", task_id, "note_requires_actor_and_body",
                ),
            }),
        ]

    handlers.append(task_handler(
        "handle_add_note", nominal("note_input"), product_id("note_input_task_field"), note_action,
    ))

    def attachment_action(prefix, state, payload, task_id, index, fields):
        return [
            expression(f"{prefix}_name", "project_field", {
                "value": payload, "field": local("attachment_input_name_field"),
            }),
            expression(f"{prefix}_actor", "project_field", {
                "value": payload, "field": local("attachment_input_actor_field"),
            }),
            expression(f"{prefix}_content", "project_field", {
                "value": payload, "field": local("attachment_input_content_field"),
            }),
            expression(f"{prefix}_name_len", "text_len", {"value": result(f"{prefix}_name")}),
            expression(f"{prefix}_actor_len", "text_len", {"value": result(f"{prefix}_actor")}),
            expression(f"{prefix}_content_len", "bytes_len", {"value": result(f"{prefix}_content")}),
            # Application bytes have a platform-wide 64 KiB canonical bound. Keep
            # the product check explicit so the attachment contract does not rely
            # on an adapter's larger immutable-blob ceiling.
            expression(f"{prefix}_content_max_plus_one", "const_i64", 65_537),
            expression(f"{prefix}_name_valid", "lt_i64", {
                "lhs": result(f"{prefix}_zero"), "rhs": result(f"{prefix}_name_len"),
            }),
            expression(f"{prefix}_actor_valid", "lt_i64", {
                "lhs": result(f"{prefix}_zero"), "rhs": result(f"{prefix}_actor_len"),
            }),
            expression(f"{prefix}_content_nonempty", "lt_i64", {
                "lhs": result(f"{prefix}_zero"), "rhs": result(f"{prefix}_content_len"),
            }),
            expression(f"{prefix}_content_bounded", "lt_i64", {
                "lhs": result(f"{prefix}_content_len"), "rhs": result(f"{prefix}_content_max_plus_one"),
            }),
            expression(f"{prefix}_pending", "project_field", {
                "value": state, "field": local("project_pending_field"),
            }),
            call(f"{prefix}_pending_none", "pending_is_none", [result(f"{prefix}_pending")]),
            expression(f"{prefix}_not_archived", "not_bool", {"value": fields["archived"]}),
            expression(f"{prefix}_valid_a", "and_bool", {
                "lhs": result(f"{prefix}_name_valid"), "rhs": result(f"{prefix}_actor_valid"),
            }),
            expression(f"{prefix}_valid_b", "and_bool", {
                "lhs": result(f"{prefix}_content_nonempty"), "rhs": result(f"{prefix}_content_bounded"),
            }),
            expression(f"{prefix}_valid_c", "and_bool", {
                "lhs": result(f"{prefix}_valid_a"), "rhs": result(f"{prefix}_valid_b"),
            }),
            expression(f"{prefix}_valid_d", "and_bool", {
                "lhs": result(f"{prefix}_valid_c"), "rhs": result(f"{prefix}_pending_none"),
            }),
            expression(f"{prefix}_valid", "and_bool", {
                "lhs": result(f"{prefix}_valid_d"), "rhs": result(f"{prefix}_not_archived"),
            }),
            expression(f"{prefix}_choice", "if", {
                "condition": result(f"{prefix}_valid"),
                "result": nominal("mutation_decision"),
                "then_body": yielding([
                    *project_fields(state, f"{prefix}_project"),
                    expression(f"{prefix}_empty_digest", "const_bytes", ""),
                    expression(f"{prefix}_pending_value", "construct_product", {
                        "product": local("pending_attachment"),
                        "fields": [
                            {"field": local("pending_task_field"), "value": task_id},
                            {"field": local("pending_name_field"), "value": result(f"{prefix}_name")},
                            {"field": local("pending_actor_field"), "value": result(f"{prefix}_actor")},
                            {"field": local("pending_content_field"), "value": result(f"{prefix}_content")},
                            {"field": local("pending_digest_field"), "value": result(f"{prefix}_empty_digest")},
                        ],
                    }),
                    expression(f"{prefix}_pending_some", "construct_variant", {
                        "variant": local("pending_some_variant"), "payload": result(f"{prefix}_pending_value"),
                    }),
                    construct_project(
                        f"{prefix}_project_value",
                        prefix,
                        result(f"{prefix}_project_name"),
                        result(f"{prefix}_project_next_task"),
                        result(f"{prefix}_project_next_note"),
                        result(f"{prefix}_project_tasks"),
                        result(f"{prefix}_project_activity"),
                        result(f"{prefix}_pending_some"),
                    ),
                    expression(f"{prefix}_blob_request", "construct_variant", {
                        "variant": local("blob_request_put_variant"), "payload": result(f"{prefix}_content"),
                    }),
                    expression(f"{prefix}_host_command", "construct_variant", {
                        "variant": local("host_command_blob_variant"), "payload": result(f"{prefix}_blob_request"),
                    }),
                    expression(f"{prefix}_code", "const_text", "attachment_pending"),
                    call(f"{prefix}_response", "make_response_accepted", [task_id, result(f"{prefix}_code")]),
                    call(f"{prefix}_suspended", "make_suspended", [
                        result(f"{prefix}_project_value"), result(f"{prefix}_response"),
                        result(f"{prefix}_host_command"),
                    ]),
                ], result(f"{prefix}_suspended")),
                "else_body": code_decision_body(
                    f"{prefix}_invalid", "decline_code", task_id, "attachment_invalid_or_pending",
                ),
            }),
        ]

    handlers.append(task_handler(
        "handle_request_attachment", nominal("attachment_input"),
        product_id("attachment_input_task_field"), attachment_action,
    ))
    return handlers


def attachment_functions():
    functions = [function(
        "pending_is_none",
        [("pending_none_value", "pending", nominal("pending_attachment_option"))],
        "bool",
        [expression("pending_none_match", "match_sum", {
            "scrutinee": parameter("pending_none_value"),
            "result": "bool",
            "arms": [
                arm("pending_none_variant", yielding([
                    expression("pending_none_true", "const_bool", True),
                ], result("pending_none_true"))),
                arm("pending_some_variant", yielding([
                    expression("pending_none_false", "const_bool", False),
                ], result("pending_none_false")), "pending_none_payload"),
            ],
        })],
        result("pending_none_match"),
    )]
    functions.append(function(
        "clear_pending_attachment",
        [
            ("clear_attachment_state", "state", nominal("project")),
            ("clear_attachment_pending", "pending", nominal("pending_attachment")),
            ("clear_attachment_code", "code", "text"),
        ],
        nominal("mutation_decision"),
        [
            *project_fields(parameter("clear_attachment_state"), "clear_attachment_project"),
            expression("clear_attachment_task", "project_field", {
                "value": parameter("clear_attachment_pending"), "field": local("pending_task_field"),
            }),
            expression("clear_attachment_none", "construct_variant", {"variant": local("pending_none_variant")}),
            construct_project(
                "clear_attachment_project_value",
                "clear_attachment",
                result("clear_attachment_project_name"),
                result("clear_attachment_project_next_task"),
                result("clear_attachment_project_next_note"),
                result("clear_attachment_project_tasks"),
                result("clear_attachment_project_activity"),
                result("clear_attachment_none"),
            ),
            call("clear_attachment_response", "make_response_conflict", [
                result("clear_attachment_task"), parameter("clear_attachment_code"),
            ]),
            call("clear_attachment_completed", "make_completed", [
                result("clear_attachment_project_value"), result("clear_attachment_response"),
            ]),
        ],
        result("clear_attachment_completed"),
    ))
    functions.append(function(
        "inspect_pending_attachment",
        [
            ("inspect_attachment_state", "state", nominal("project")),
            ("inspect_attachment_pending", "pending", nominal("pending_attachment")),
            ("inspect_attachment_digest", "digest", "bytes"),
        ],
        nominal("mutation_decision"),
        [
            *project_fields(parameter("inspect_attachment_state"), "inspect_attachment_project"),
            expression("inspect_attachment_task", "project_field", {
                "value": parameter("inspect_attachment_pending"), "field": local("pending_task_field"),
            }),
            expression("inspect_attachment_name", "project_field", {
                "value": parameter("inspect_attachment_pending"), "field": local("pending_name_field"),
            }),
            expression("inspect_attachment_actor", "project_field", {
                "value": parameter("inspect_attachment_pending"), "field": local("pending_actor_field"),
            }),
            expression("inspect_attachment_content", "project_field", {
                "value": parameter("inspect_attachment_pending"), "field": local("pending_content_field"),
            }),
            expression("inspect_attachment_pending_value", "construct_product", {
                "product": local("pending_attachment"),
                "fields": [
                    {"field": local("pending_task_field"), "value": result("inspect_attachment_task")},
                    {"field": local("pending_name_field"), "value": result("inspect_attachment_name")},
                    {"field": local("pending_actor_field"), "value": result("inspect_attachment_actor")},
                    {"field": local("pending_content_field"), "value": result("inspect_attachment_content")},
                    {"field": local("pending_digest_field"), "value": parameter("inspect_attachment_digest")},
                ],
            }),
            expression("inspect_attachment_some", "construct_variant", {
                "variant": local("pending_some_variant"), "payload": result("inspect_attachment_pending_value"),
            }),
            construct_project(
                "inspect_attachment_project_value",
                "inspect_attachment",
                result("inspect_attachment_project_name"),
                result("inspect_attachment_project_next_task"),
                result("inspect_attachment_project_next_note"),
                result("inspect_attachment_project_tasks"),
                result("inspect_attachment_project_activity"),
                result("inspect_attachment_some"),
            ),
            expression("inspect_attachment_request", "construct_variant", {
                "variant": local("blob_request_inspect_variant"), "payload": parameter("inspect_attachment_digest"),
            }),
            expression("inspect_attachment_command", "construct_variant", {
                "variant": local("host_command_blob_variant"), "payload": result("inspect_attachment_request"),
            }),
            expression("inspect_attachment_code", "const_text", "attachment_visibility_unknown"),
            call("inspect_attachment_response", "make_response_accepted", [
                result("inspect_attachment_task"), result("inspect_attachment_code"),
            ]),
            call("inspect_attachment_suspended", "make_suspended", [
                result("inspect_attachment_project_value"), result("inspect_attachment_response"),
                result("inspect_attachment_command"),
            ]),
        ],
        result("inspect_attachment_suspended"),
    ))
    functions.append(function(
        "finalize_attachment",
        [
            ("finalize_attachment_state", "state", nominal("project")),
            ("finalize_attachment_pending", "pending", nominal("pending_attachment")),
            ("finalize_attachment_digest", "digest", "bytes"),
        ],
        nominal("mutation_decision"),
        [
            expression("finalize_attachment_task_id", "project_field", {
                "value": parameter("finalize_attachment_pending"), "field": local("pending_task_field"),
            }),
            expression("finalize_attachment_digest_len", "bytes_len", {
                "value": parameter("finalize_attachment_digest"),
            }),
            expression("finalize_attachment_digest_expected", "const_i64", 32),
            expression("finalize_attachment_digest_valid", "equal_i64", {
                "lhs": result("finalize_attachment_digest_len"),
                "rhs": result("finalize_attachment_digest_expected"),
            }),
            expression("finalize_attachment_choice", "if", {
                "condition": result("finalize_attachment_digest_valid"),
                "result": nominal("mutation_decision"),
                "then_body": yielding([
                    expression("finalize_attachment_tasks", "project_field", {
                        "value": parameter("finalize_attachment_state"), "field": local("project_tasks_field"),
                    }),
                    call("finalize_attachment_index", "find_task_index", [
                        result("finalize_attachment_tasks"), result("finalize_attachment_task_id"),
                    ]),
                    expression("finalize_attachment_zero", "const_i64", 0),
                    expression("finalize_attachment_missing", "lt_i64", {
                        "lhs": result("finalize_attachment_index"), "rhs": result("finalize_attachment_zero"),
                    }),
                    expression("finalize_attachment_task_choice", "if", {
                        "condition": result("finalize_attachment_missing"),
                        "result": nominal("mutation_decision"),
                        "then_body": code_decision_body(
                            "finalize_attachment_missing_task", "decline_code",
                            result("finalize_attachment_task_id"), "attachment_task_missing",
                        ),
                        "else_body": yielding([
                            expression("finalize_attachment_task", "sequence_get", {
                                "sequence": local("task_sequence"), "value": result("finalize_attachment_tasks"),
                                "index": result("finalize_attachment_index"),
                            }),
                            *task_fields(result("finalize_attachment_task"), "finalize_attachment_field"),
                            expression("finalize_attachment_name", "project_field", {
                                "value": parameter("finalize_attachment_pending"), "field": local("pending_name_field"),
                            }),
                            expression("finalize_attachment_actor", "project_field", {
                                "value": parameter("finalize_attachment_pending"), "field": local("pending_actor_field"),
                            }),
                            expression("finalize_attachment_content", "project_field", {
                                "value": parameter("finalize_attachment_pending"), "field": local("pending_content_field"),
                            }),
                            expression("finalize_attachment_length", "bytes_len", {
                                "value": result("finalize_attachment_content"),
                            }),
                            expression("finalize_attachment_value", "construct_product", {
                                "product": local("attachment"),
                                "fields": [
                                    {"field": local("attachment_digest_field"), "value": parameter("finalize_attachment_digest")},
                                    {"field": local("attachment_name_field"), "value": result("finalize_attachment_name")},
                                    {"field": local("attachment_length_field"), "value": result("finalize_attachment_length")},
                                    {"field": local("attachment_actor_field"), "value": result("finalize_attachment_actor")},
                                ],
                            }),
                            expression("finalize_attachment_values", "sequence_append", {
                                "sequence": local("attachment_sequence"),
                                "value": result("finalize_attachment_field_attachments"),
                                "element": result("finalize_attachment_value"),
                            }),
                            task_rebuild_call(
                                "finalize_attachment_task_value",
                                {name: result(f"finalize_attachment_field_{name}") for name in TASK_FIELD_ORDER},
                                {"attachments": result("finalize_attachment_values")},
                            ),
                            expression("finalize_attachment_code", "const_text", "attachment_added"),
                            call("finalize_attachment_updated", "replace_project_task", [
                                parameter("finalize_attachment_state"), result("finalize_attachment_index"),
                                result("finalize_attachment_task_value_rebuilt"),
                                result("finalize_attachment_actor"), result("finalize_attachment_code"),
                            ]),
                            *project_fields(result("finalize_attachment_updated"), "finalize_attachment_project"),
                            expression("finalize_attachment_none", "construct_variant", {
                                "variant": local("pending_none_variant"),
                            }),
                            construct_project(
                                "finalize_attachment_project_value",
                                "finalize_attachment",
                                result("finalize_attachment_project_name"),
                                result("finalize_attachment_project_next_task"),
                                result("finalize_attachment_project_next_note"),
                                result("finalize_attachment_project_tasks"),
                                result("finalize_attachment_project_activity"),
                                result("finalize_attachment_none"),
                            ),
                            call("finalize_attachment_response", "make_response_accepted", [
                                result("finalize_attachment_task_id"), result("finalize_attachment_code"),
                            ]),
                            call("finalize_attachment_completed", "make_completed", [
                                result("finalize_attachment_project_value"), result("finalize_attachment_response"),
                            ]),
                        ], result("finalize_attachment_completed")),
                    }),
                ], result("finalize_attachment_task_choice")),
                "else_body": code_decision_body(
                    "finalize_attachment_invalid_digest", "decline_code",
                    result("finalize_attachment_task_id"), "attachment_digest_invalid",
                ),
            }),
        ],
        result("finalize_attachment_choice"),
    ))
    return functions


def query_error_body(prefix, code):
    return yielding([
        expression(f"{prefix}_zero", "const_i64", 0),
        expression(f"{prefix}_code", "const_text", code),
        call(f"{prefix}_detail", "make_response_detail", [result(f"{prefix}_zero"), result(f"{prefix}_code")]),
        expression(f"{prefix}_result", "construct_variant", {
            "variant": local("query_result_error_variant"),
            "payload": result(f"{prefix}_detail"),
        }),
    ], result(f"{prefix}_result"))


def page_query_body(prefix, include_archived):
    payload = block_argument(f"query_{'export_page' if include_archived else 'list_tasks'}_payload")
    if include_archived:
        selected_operations = []
        selected = result(f"{prefix}_tasks")
        total_source = selected
        after_field = "page_after_field"
        limit_field = "page_limit_field"
    else:
        selected_operations = [
            call(f"{prefix}_filtered", "filter_tasks", [result(f"{prefix}_tasks"), payload]),
            expression(f"{prefix}_order", "project_field", {
                "value": payload, "field": local("list_order_field"),
            }),
            expression(f"{prefix}_selection_limit", "add_i64", {
                "lhs": result(f"{prefix}_after"), "rhs": result(f"{prefix}_limit"),
            }),
            call(f"{prefix}_selected", "order_tasks", [
                result(f"{prefix}_order"), result(f"{prefix}_filtered"),
                result(f"{prefix}_selection_limit"),
            ]),
        ]
        selected = result(f"{prefix}_selected")
        total_source = result(f"{prefix}_filtered")
        after_field = "list_after_field"
        limit_field = "list_limit_field"
    return yielding([
        expression(f"{prefix}_tasks", "project_field", {
            "value": parameter("query_state"), "field": local("project_tasks_field"),
        }),
        expression(f"{prefix}_after", "project_field", {
            "value": payload, "field": local(after_field),
        }),
        expression(f"{prefix}_limit", "project_field", {
            "value": payload, "field": local(limit_field),
        }),
        *selected_operations,
        expression(f"{prefix}_total", "sequence_len", {
            "sequence": local("task_sequence"), "value": total_source,
        }),
        call(f"{prefix}_valid", "page_request_valid", [
            result(f"{prefix}_after"), result(f"{prefix}_limit"), result(f"{prefix}_total"),
        ]),
        expression(f"{prefix}_choice", "if", {
            "condition": result(f"{prefix}_valid"),
            "result": nominal("query_result"),
            "then_body": yielding([
                call(f"{prefix}_page", "make_task_page", [
                    result(f"{prefix}_tasks"), selected,
                    result(f"{prefix}_after"), result(f"{prefix}_limit"),
                    result(f"{prefix}_total"),
                ]),
                expression(f"{prefix}_result", "construct_variant", {
                    "variant": local("query_result_tasks_variant"),
                    "payload": result(f"{prefix}_page"),
                }),
            ], result(f"{prefix}_result")),
            "else_body": query_error_body(f"{prefix}_invalid", "query_bounds_invalid"),
        }),
    ], result(f"{prefix}_choice"))


def next_query_body():
    return yielding([
        expression("query_next_tasks", "project_field", {
            "value": parameter("query_state"), "field": local("project_tasks_field"),
        }),
        call("query_next_actionable", "actionable_tasks", [result("query_next_tasks")]),
        expression("query_next_total", "sequence_len", {
            "sequence": local("task_sequence"), "value": result("query_next_actionable"),
        }),
        call("query_next_sorted", "sort_tasks", [
            result("query_next_actionable"), block_argument("query_next_tasks_payload"),
        ]),
        expression("query_next_zero", "const_i64", 0),
        call("query_next_valid", "page_request_valid", [
            result("query_next_zero"), block_argument("query_next_tasks_payload"), result("query_next_total"),
        ]),
        expression("query_next_choice", "if", {
            "condition": result("query_next_valid"),
            "result": nominal("query_result"),
            "then_body": yielding([
                call("query_next_page", "make_task_page", [
                    result("query_next_tasks"), result("query_next_sorted"), result("query_next_zero"),
                    block_argument("query_next_tasks_payload"), result("query_next_total"),
                ]),
                expression("query_next_result", "construct_variant", {
                    "variant": local("query_result_tasks_variant"), "payload": result("query_next_page"),
                }),
            ], result("query_next_result")),
            "else_body": query_error_body("query_next_invalid", "query_bounds_invalid"),
        }),
    ], result("query_next_choice"))


def summary_query_body():
    return yielding([
        expression("query_summary_tasks", "project_field", {
            "value": parameter("query_state"), "field": local("project_tasks_field"),
        }),
        call("query_summary_value", "summarize_tasks", [result("query_summary_tasks")]),
        expression("query_summary_result", "construct_variant", {
            "variant": local("query_result_summary_variant"), "payload": result("query_summary_value"),
        }),
    ], result("query_summary_result"))


def context_query_body():
    payload = block_argument("query_agent_context_payload")
    return yielding([
        expression("query_context_tasks", "project_field", {
            "value": parameter("query_state"), "field": local("project_tasks_field"),
        }),
        expression("query_context_max_tasks", "project_field", {
            "value": payload, "field": local("context_tasks_field"),
        }),
        expression("query_context_max_notes", "project_field", {
            "value": payload, "field": local("context_notes_field"),
        }),
        expression("query_context_max_dependencies", "project_field", {
            "value": payload, "field": local("context_dependencies_field"),
        }),
        expression("query_context_max_text", "project_field", {
            "value": payload, "field": local("context_text_bytes_field"),
        }),
        call("query_context_candidates", "context_candidates", [
            result("query_context_tasks"), result("query_context_max_tasks"),
        ]),
        call("query_context_full_candidates", "context_all_candidates", [
            result("query_context_tasks"),
        ]),
        expression("query_context_total", "sequence_len", {
            "sequence": local("task_sequence"),
            "value": result("query_context_full_candidates"),
        }),
        expression("query_context_zero", "const_i64", 0),
        expression("query_context_notes_positive", "lt_i64", {
            "lhs": result("query_context_zero"), "rhs": result("query_context_max_notes"),
        }),
        expression("query_context_dependencies_positive", "lt_i64", {
            "lhs": result("query_context_zero"), "rhs": result("query_context_max_dependencies"),
        }),
        expression("query_context_text_positive", "lt_i64", {
            "lhs": result("query_context_zero"), "rhs": result("query_context_max_text"),
        }),
        call("query_context_tasks_valid", "page_request_valid", [
            result("query_context_zero"), result("query_context_max_tasks"), result("query_context_total"),
        ]),
        expression("query_context_valid_a", "and_bool", {
            "lhs": result("query_context_tasks_valid"), "rhs": result("query_context_notes_positive"),
        }),
        expression("query_context_valid_b", "and_bool", {
            "lhs": result("query_context_valid_a"), "rhs": result("query_context_dependencies_positive"),
        }),
        expression("query_context_valid", "and_bool", {
            "lhs": result("query_context_valid_b"), "rhs": result("query_context_text_positive"),
        }),
        expression("query_context_choice", "if", {
            "condition": result("query_context_valid"),
            "result": nominal("query_result"),
            "then_body": yielding([
                call("query_context_value", "make_context_result", [
                    result("query_context_tasks"), result("query_context_full_candidates"),
                    result("query_context_candidates"),
                    result("query_context_max_tasks"), result("query_context_max_notes"),
                    result("query_context_max_dependencies"), result("query_context_max_text"),
                ]),
                expression("query_context_result", "construct_variant", {
                    "variant": local("query_result_context_variant"), "payload": result("query_context_value"),
                }),
            ], result("query_context_result")),
            "else_body": query_error_body("query_context_invalid", "context_bounds_invalid"),
        }),
    ], result("query_context_choice"))


def activity_query_body():
    payload = block_argument("query_recent_activity_payload")
    return yielding([
        expression("query_activity_values", "project_field", {
            "value": parameter("query_state"), "field": local("project_activity_field"),
        }),
        expression("query_activity_after", "project_field", {
            "value": payload, "field": local("activity_request_after_field"),
        }),
        expression("query_activity_limit", "project_field", {
            "value": payload, "field": local("activity_request_limit_field"),
        }),
        expression("query_activity_filter", "project_field", {
            "value": payload, "field": local("activity_request_filter_field"),
        }),
        call("query_activity_filtered", "filter_activities", [
            result("query_activity_values"), result("query_activity_filter"),
        ]),
        expression("query_activity_total", "sequence_len", {
            "sequence": local("activity_sequence"), "value": result("query_activity_filtered"),
        }),
        call("query_activity_valid", "page_request_valid", [
            result("query_activity_after"), result("query_activity_limit"), result("query_activity_total"),
        ]),
        expression("query_activity_choice", "if", {
            "condition": result("query_activity_valid"),
            "result": nominal("query_result"),
            "then_body": yielding([
                call("query_activity_page", "make_activity_page", [
                    result("query_activity_filtered"), result("query_activity_after"),
                    result("query_activity_limit"),
                ]),
                expression("query_activity_result", "construct_variant", {
                    "variant": local("query_result_activity_variant"),
                    "payload": result("query_activity_page"),
                }),
            ], result("query_activity_result")),
            "else_body": query_error_body("query_activity_invalid", "query_bounds_invalid"),
        }),
    ], result("query_activity_choice"))


def get_query_body():
    return yielding([
        expression("query_get_tasks", "project_field", {"value": parameter("query_state"), "field": local("project_tasks_field")}),
        call("query_get_index", "find_task_index", [result("query_get_tasks"), block_argument("query_get_task_payload")]),
        expression("query_get_zero", "const_i64", 0),
        expression("query_get_missing", "lt_i64", {"lhs": result("query_get_index"), "rhs": result("query_get_zero")}),
        expression("query_get_choice", "if", {
            "condition": result("query_get_missing"),
            "result": nominal("query_result"),
            "then_body": yielding([
                expression("query_get_code", "const_text", "task_not_found"),
                call("query_get_detail", "make_response_detail", [block_argument("query_get_task_payload"), result("query_get_code")]),
                expression("query_get_not_found", "construct_variant", {
                    "variant": local("query_result_not_found_variant"),
                    "payload": result("query_get_detail"),
                }),
            ], result("query_get_not_found")),
            "else_body": yielding([
                expression("query_get_task", "sequence_get", {
                    "sequence": local("task_sequence"),
                    "value": result("query_get_tasks"),
                    "index": result("query_get_index"),
                }),
                call("query_get_view", "make_task_view", [result("query_get_tasks"), result("query_get_task")]),
                expression("query_get_found", "construct_variant", {
                    "variant": local("query_result_task_view_variant"),
                    "payload": result("query_get_view"),
                }),
            ], result("query_get_found")),
        }),
    ], result("query_get_choice"))


def query_function(query_variants):
    query_arms = []
    for variant, variant_name, payload in query_variants:
        payload_symbol = f"query_{variant_name}_payload" if payload is not None else None
        if variant_name == "get_task":
            body = get_query_body()
        elif variant_name == "list_tasks":
            body = page_query_body("query_list", False)
        elif variant_name == "next_tasks":
            body = next_query_body()
        elif variant_name == "project_summary":
            body = summary_query_body()
        elif variant_name == "agent_context":
            body = context_query_body()
        elif variant_name == "export_page":
            body = page_query_body("query_export", True)
        elif variant_name == "recent_activity":
            body = activity_query_body()
        else:
            body = query_error_body(f"query_{variant_name}", "query_not_implemented")
        query_arms.append(arm(variant, body, payload_symbol))
    return function(
        "query_entry",
        [("query_state", "state", nominal("project")), ("query_value", "query", nominal("query"))],
        nominal("query_result"),
        [expression("query_match", "match_sum", {
            "scrutinee": parameter("query_value"),
            "result": nominal("query_result"),
            "arms": query_arms,
        })],
        result("query_match"),
    )


def resume_function(blob_outcome_variants):
    complete_variants = {"stored", "already_present", "inspect_present"}
    clear_codes = {
        "put_failed": "attachment_put_failed",
        "inspect_absent": "attachment_absent_retry_allowed",
    }
    inner_arms = []
    for variant, variant_name, payload in blob_outcome_variants:
        payload_symbol = f"resume_{variant_name}_payload" if payload is not None else None
        if variant_name in complete_variants:
            call_name = f"resume_{variant_name}_complete"
            body = yielding([
                call(call_name, "finalize_attachment", [
                    parameter("resume_state"), block_argument("resume_pending_payload"),
                    block_argument(payload_symbol),
                ]),
            ], result(call_name))
        elif variant_name == "put_unknown":
            body = yielding([
                call("resume_put_unknown_inspect", "inspect_pending_attachment", [
                    parameter("resume_state"), block_argument("resume_pending_payload"),
                    block_argument(payload_symbol),
                ]),
            ], result("resume_put_unknown_inspect"))
        elif variant_name in clear_codes:
            body = yielding([
                expression(f"resume_{variant_name}_code", "const_text", clear_codes[variant_name]),
                call(f"resume_{variant_name}_clear", "clear_pending_attachment", [
                    parameter("resume_state"), block_argument("resume_pending_payload"),
                    result(f"resume_{variant_name}_code"),
                ]),
            ], result(f"resume_{variant_name}_clear"))
        else:
            body = yielding([
                expression("resume_indeterminate_task", "project_field", {
                    "value": block_argument("resume_pending_payload"), "field": local("pending_task_field"),
                }),
                expression("resume_indeterminate_code", "const_text", "attachment_visibility_indeterminate"),
                call("resume_indeterminate_result", "unchanged_code", [
                    result("resume_indeterminate_task"), result("resume_indeterminate_code"),
                ]),
            ], result("resume_indeterminate_result"))
        inner_arms.append(arm(variant, body, payload_symbol))
    pending_body = yielding([
        expression("resume_blob_match", "match_sum", {
            "scrutinee": block_argument("resume_blob_payload"),
            "result": nominal("mutation_decision"),
            "arms": inner_arms,
        }),
    ], result("resume_blob_match"))
    no_pending_body = yielding([
        expression("resume_no_pending_task", "const_i64", 0),
        expression("resume_no_pending_code", "const_text", "attachment_outcome_without_pending_request"),
        call("resume_no_pending_result", "decline_code", [
            result("resume_no_pending_task"), result("resume_no_pending_code"),
        ]),
    ], result("resume_no_pending_result"))
    inner = yielding([
        expression("resume_pending", "project_field", {
            "value": parameter("resume_state"), "field": local("project_pending_field"),
        }),
        expression("resume_pending_match", "match_sum", {
            "scrutinee": result("resume_pending"),
            "result": nominal("mutation_decision"),
            "arms": [
                arm("pending_none_variant", no_pending_body),
                arm("pending_some_variant", pending_body, "resume_pending_payload"),
            ],
        }),
    ], result("resume_pending_match"))
    return function(
        "transition_resume",
        [("resume_state", "state", nominal("project")), ("resume_outcome", "outcome", nominal("host_outcome"))],
        nominal("mutation_decision"),
        [expression("resume_match", "match_sum", {
            "scrutinee": parameter("resume_outcome"),
            "result": nominal("mutation_decision"),
            "arms": [arm("host_outcome_blob_variant", inner, "resume_blob_payload")],
        })],
        result("resume_match"),
    )


def semantic_operations():
    event_variants = [
        ("event_rename_project_variant", "rename_project", "text"),
        ("event_create_task_variant", "create_task", nominal("create_task_input")),
        ("event_edit_task_variant", "edit_task", nominal("edit_task_input")),
        ("event_start_task_variant", "start_task", "i64"),
        ("event_stop_task_variant", "stop_task", "i64"),
        ("event_complete_task_variant", "complete_task", "i64"),
        ("event_cancel_task_variant", "cancel_task", "i64"),
        ("event_reopen_task_variant", "reopen_task", "i64"),
        ("event_hold_task_variant", "hold_task", nominal("hold_input")),
        ("event_release_task_variant", "release_task", "i64"),
        ("event_set_priority_variant", "set_priority", nominal("priority_input")),
        ("event_add_label_variant", "add_label", nominal("label_input")),
        ("event_remove_label_variant", "remove_label", nominal("label_input")),
        ("event_add_dependency_variant", "add_dependency", nominal("dependency_input")),
        ("event_remove_dependency_variant", "remove_dependency", nominal("dependency_input")),
        ("event_add_note_variant", "add_note", nominal("note_input")),
        ("event_request_attachment_variant", "request_attachment", nominal("attachment_input")),
        ("event_archive_task_variant", "archive_task", "i64"),
        ("event_unarchive_task_variant", "unarchive_task", "i64"),
    ]
    query_variants = [
        ("query_get_task_variant", "get_task", "i64"),
        ("query_list_tasks_variant", "list_tasks", nominal("list_request")),
        ("query_next_tasks_variant", "next_tasks", "i64"),
        ("query_project_summary_variant", "project_summary", None),
        ("query_agent_context_variant", "agent_context", nominal("context_request")),
        ("query_export_page_variant", "export_page", nominal("page_request")),
        ("query_recent_activity_variant", "recent_activity", nominal("activity_request")),
    ]
    blob_outcome_variants = [
        ("blob_outcome_stored_variant", "stored", "bytes"),
        ("blob_outcome_already_present_variant", "already_present", "bytes"),
        ("blob_outcome_put_failed_variant", "put_failed", "bytes"),
        ("blob_outcome_put_unknown_variant", "put_unknown", "bytes"),
        ("blob_outcome_inspect_present_variant", "inspect_present", "bytes"),
        ("blob_outcome_inspect_absent_variant", "inspect_absent", "bytes"),
        ("blob_outcome_inspect_indeterminate_variant", "inspect_indeterminate", "bytes"),
    ]
    operations = [
        {"kind": "create_package", "data": {"symbol": "lkjwork_package", "name": "lkjwork"}},
        {"kind": "create_module", "data": {"symbol": "main_module", "package": local("lkjwork_package"), "name": "main"}},
        sequence_type("text_sequence", "text"),
        sequence_type("id_sequence", "i64"),
        product_type("dependency_search_state", [
            ("dependency_search_work_field", "work", nominal("id_sequence")),
            ("dependency_search_found_field", "found", "bool"),
        ]),
        sum_type("task_phase", [
            ("phase_planned_variant", "planned", None),
            ("phase_active_variant", "active", None),
            ("phase_done_variant", "done", None),
            ("phase_cancelled_variant", "cancelled", None),
        ]),
        sum_type("task_hold", [
            ("hold_none_variant", "none", None),
            ("hold_manual_variant", "manual", "text"),
        ]),
        product_type("note", [
            ("note_id_field", "id", "i64"),
            ("note_actor_field", "actor", "text"),
            ("note_body_field", "body", "text"),
        ]),
        sequence_type("note_sequence", nominal("note")),
        product_type("attachment", [
            ("attachment_digest_field", "digest", "bytes"),
            ("attachment_name_field", "name", "text"),
            ("attachment_length_field", "length", "i64"),
            ("attachment_actor_field", "actor", "text"),
        ]),
        sequence_type("attachment_sequence", nominal("attachment")),
        product_type("task", [
            ("task_id_field", "id", "i64"),
            ("task_title_field", "title", "text"),
            ("task_description_field", "description", "text"),
            ("task_phase_field", "phase", nominal("task_phase")),
            ("task_hold_field", "hold", nominal("task_hold")),
            ("task_priority_field", "priority", "i64"),
            ("task_labels_field", "labels", nominal("text_sequence")),
            ("task_dependencies_field", "dependencies", nominal("id_sequence")),
            ("task_notes_field", "notes", nominal("note_sequence")),
            ("task_attachments_field", "attachments", nominal("attachment_sequence")),
            ("task_archived_field", "archived", "bool"),
        ]),
        sequence_type("task_sequence", nominal("task")),
        product_type("task_view", [
            ("task_view_task_field", "task", nominal("task")),
            ("task_view_ready_field", "ready", "bool"),
            ("task_view_blockers_field", "blockers", nominal("id_sequence")),
        ]),
        sequence_type("task_view_sequence", nominal("task_view")),
        product_type("task_sort_state", [
            ("task_sort_previous_field", "previous_index", "i64"),
            ("task_sort_output_field", "output", nominal("task_sequence")),
        ]),
        product_type("page_build_state", [
            ("page_build_views_field", "views", nominal("task_view_sequence")),
            ("page_build_omitted_field", "omitted", "i64"),
        ]),
        product_type("context_build_state", [
            ("context_build_views_field", "views", nominal("task_view_sequence")),
            ("context_build_notes_used_field", "notes_used", "i64"),
            ("context_build_dependencies_used_field", "dependencies_used", "i64"),
            ("context_build_text_used_field", "text_used", "i64"),
            ("context_build_text_truncated_field", "text_truncated", "bool"),
        ]),
        product_type("activity_page_state", [
            ("activity_page_state_items_field", "items", nominal("activity_sequence")),
            ("activity_page_state_omitted_field", "omitted", "i64"),
        ]),
        product_type("activity", [
            ("activity_task_field", "task", "i64"),
            ("activity_actor_field", "actor", "text"),
            ("activity_code_field", "code", "text"),
        ]),
        sequence_type("activity_sequence", nominal("activity")),
        product_type("pending_attachment", [
            ("pending_task_field", "task", "i64"),
            ("pending_name_field", "name", "text"),
            ("pending_actor_field", "actor", "text"),
            ("pending_content_field", "content", "bytes"),
            ("pending_digest_field", "digest", "bytes"),
        ]),
        sum_type("pending_attachment_option", [
            ("pending_none_variant", "none", None),
            ("pending_some_variant", "some", nominal("pending_attachment")),
        ]),
        product_type("project", [
            ("project_name_field", "name", "text"),
            ("project_next_task_field", "next_task_id", "i64"),
            ("project_next_note_field", "next_note_id", "i64"),
            ("project_tasks_field", "tasks", nominal("task_sequence")),
            ("project_activity_field", "activity", nominal("activity_sequence")),
            ("project_pending_field", "pending_attachment", nominal("pending_attachment_option")),
        ]),
        product_type("create_task_input", [
            ("create_title_field", "title", "text"),
            ("create_description_field", "description", "text"),
            ("create_priority_field", "priority", "i64"),
            ("create_labels_field", "labels", nominal("text_sequence")),
            ("create_dependencies_field", "dependencies", nominal("id_sequence")),
            ("create_actor_field", "actor", "text"),
        ]),
        product_type("edit_task_input", [
            ("edit_task_field", "task", "i64"),
            ("edit_set_title_field", "set_title", "bool"),
            ("edit_title_field", "title", "text"),
            ("edit_set_description_field", "set_description", "bool"),
            ("edit_description_field", "description", "text"),
            ("edit_set_priority_field", "set_priority", "bool"),
            ("edit_priority_field", "priority", "i64"),
        ]),
        product_type("hold_input", [("hold_task_field", "task", "i64"), ("hold_reason_field", "reason", "text")]),
        product_type("priority_input", [("priority_task_field", "task", "i64"), ("priority_value_field", "priority", "i64")]),
        product_type("label_input", [("label_task_field", "task", "i64"), ("label_value_field", "label", "text")]),
        product_type("dependency_input", [("dependency_task_field", "task", "i64"), ("dependency_on_field", "prerequisite", "i64")]),
        product_type("note_input", [
            ("note_input_task_field", "task", "i64"),
            ("note_input_actor_field", "actor", "text"),
            ("note_input_body_field", "body", "text"),
        ]),
        product_type("attachment_input", [
            ("attachment_input_task_field", "task", "i64"),
            ("attachment_input_name_field", "name", "text"),
            ("attachment_input_actor_field", "actor", "text"),
            ("attachment_input_content_field", "content", "bytes"),
        ]),
        sum_type("mutation_event", event_variants),
        product_type("response_detail", [
            ("response_detail_task_field", "task", "i64"),
            ("response_detail_code_field", "code", "text"),
        ]),
        sum_type("mutation_response", [
            ("response_accepted_variant", "accepted", nominal("response_detail")),
            ("response_conflict_variant", "conflict", nominal("response_detail")),
            ("response_no_change_variant", "no_change", nominal("response_detail")),
        ]),
        product_type("page_request", [("page_after_field", "after", "i64"), ("page_limit_field", "limit", "i64")]),
        sum_type("phase_filter", [
            ("phase_filter_any_variant", "any", None),
            ("phase_filter_planned_variant", "planned", None),
            ("phase_filter_active_variant", "active", None),
            ("phase_filter_done_variant", "done", None),
            ("phase_filter_cancelled_variant", "cancelled", None),
        ]),
        sum_type("readiness_filter", [
            ("readiness_filter_any_variant", "any", None),
            ("readiness_filter_ready_variant", "ready", None),
            ("readiness_filter_blocked_variant", "blocked", None),
        ]),
        sum_type("label_filter", [
            ("label_filter_any_variant", "any", None),
            ("label_filter_exact_variant", "exact", "text"),
        ]),
        sum_type("archive_filter", [
            ("archive_filter_default_variant", "default", None),
            ("archive_filter_archived_variant", "archived", None),
            ("archive_filter_all_variant", "all", None),
        ]),
        sum_type("task_order", [
            ("task_order_id_variant", "id", None),
            ("task_order_priority_variant", "priority", None),
        ]),
        product_type("list_request", [
            ("list_after_field", "after", "i64"),
            ("list_limit_field", "limit", "i64"),
            ("list_phase_field", "phase", nominal("phase_filter")),
            ("list_readiness_field", "readiness", nominal("readiness_filter")),
            ("list_label_field", "label", nominal("label_filter")),
            ("list_archive_field", "archive", nominal("archive_filter")),
            ("list_order_field", "order", nominal("task_order")),
        ]),
        product_type("context_request", [
            ("context_tasks_field", "maximum_tasks", "i64"),
            ("context_notes_field", "maximum_notes", "i64"),
            ("context_dependencies_field", "maximum_dependencies", "i64"),
            ("context_text_bytes_field", "maximum_text_bytes", "i64"),
        ]),
        sum_type("activity_filter", [
            ("activity_filter_all_variant", "all", None),
            ("activity_filter_task_variant", "task", "i64"),
        ]),
        product_type("activity_request", [
            ("activity_request_after_field", "after", "i64"),
            ("activity_request_limit_field", "limit", "i64"),
            ("activity_request_filter_field", "filter", nominal("activity_filter")),
        ]),
        sum_type("query", query_variants),
        product_type("task_page", [
            ("task_page_tasks_field", "tasks", nominal("task_view_sequence")),
            ("task_page_total_field", "total", "i64"),
            ("task_page_omitted_field", "omitted", "i64"),
            ("task_page_next_field", "next_after", "i64"),
        ]),
        product_type("context_result", [
            ("context_page_field", "page", nominal("task_page")),
            ("context_notes_omitted_field", "notes_omitted", "i64"),
            ("context_dependencies_omitted_field", "dependencies_omitted", "i64"),
            ("context_text_truncated_field", "text_truncated", "bool"),
        ]),
        product_type("activity_page", [
            ("activity_page_items_field", "items", nominal("activity_sequence")),
            ("activity_page_total_field", "total", "i64"),
            ("activity_page_omitted_field", "omitted", "i64"),
            ("activity_page_next_field", "next_after", "i64"),
        ]),
        product_type("project_summary", [
            ("summary_planned_field", "planned", "i64"),
            ("summary_active_field", "active", "i64"),
            ("summary_done_field", "done", "i64"),
            ("summary_cancelled_field", "cancelled", "i64"),
            ("summary_actionable_field", "actionable", "i64"),
            ("summary_archived_field", "archived", "i64"),
        ]),
        sum_type("query_result", [
            ("query_result_task_view_variant", "task_view", nominal("task_view")),
            ("query_result_tasks_variant", "tasks", nominal("task_page")),
            ("query_result_summary_variant", "summary", nominal("project_summary")),
            ("query_result_context_variant", "context", nominal("context_result")),
            ("query_result_activity_variant", "activity", nominal("activity_page")),
            ("query_result_not_found_variant", "not_found", nominal("response_detail")),
            ("query_result_error_variant", "error", nominal("response_detail")),
        ]),
        sum_type("blob_request", [
            ("blob_request_put_variant", "put", "bytes"),
            ("blob_request_inspect_variant", "inspect", "bytes"),
        ]),
        sum_type("blob_outcome", blob_outcome_variants),
        sum_type("host_command", [("host_command_blob_variant", "blob", nominal("blob_request"))]),
        sum_type("host_outcome", [("host_outcome_blob_variant", "blob", nominal("blob_outcome"))]),
        product_type("declined_payload", [("declined_response_field", "response", nominal("mutation_response"))]),
        product_type("unchanged_payload", [("unchanged_response_field", "response", nominal("mutation_response"))]),
        product_type("completed_payload", [
            ("completed_state_field", "state", nominal("project")),
            ("completed_response_field", "response", nominal("mutation_response")),
        ]),
        product_type("suspended_payload", [
            ("suspended_state_field", "state", nominal("project")),
            ("suspended_response_field", "response", nominal("mutation_response")),
            ("suspended_command_field", "command", nominal("host_command")),
        ]),
        sum_type("mutation_decision", [
            ("decision_declined_variant", "declined", nominal("declined_payload")),
            ("decision_unchanged_variant", "unchanged", nominal("unchanged_payload")),
            ("decision_completed_variant", "completed", nominal("completed_payload")),
            ("decision_suspended_variant", "suspended", nominal("suspended_payload")),
        ]),
        *response_functions(),
        *decision_functions(),
        *task_support_functions(),
        *state_predicate_functions(),
        id_contains_function(),
        text_contains_function(),
        *collection_validation_functions(),
        dependency_reachability_function(),
        *readiness_functions(),
        *query_support_functions(),
        *context_budget_functions(),
        *activity_query_functions(),
        *collection_transform_functions(),
        *attachment_functions(),
        unsupported_function(),
        find_task_function(),
        *lifecycle_handlers(),
        *relation_and_annotation_handlers(),
        *edit_and_note_handlers(),
        transition_function(event_variants),
        resume_function(blob_outcome_variants),
        query_function(query_variants),
        function(
            "identity_text",
            [("identity_text_value", "value", "text")],
            "text",
            [],
            parameter("identity_text_value"),
        ),
        {"kind": "set_entry_function", "data": {"package": local("lkjwork_package"), "function": local("transition_event")}},
    ]
    return operations


def run_json(command, value=None):
    encoded = b"" if value is None else json.dumps(value, sort_keys=True, separators=(",", ":")).encode()
    completed = subprocess.run(command, input=encoded, stdout=subprocess.PIPE, stderr=subprocess.PIPE, check=False)
    if completed.returncode != 0:
        raise RuntimeError(
            f"command failed ({completed.returncode}): {' '.join(map(str, command))}\n"
            f"stdout={completed.stdout.decode(errors='replace')}\n"
            f"stderr={completed.stderr.decode(errors='replace')}"
        )
    return json.loads(completed.stdout)


def rpc(process, request_id, request):
    value = {"version": MACHINE_VERSION, "request_id": request_id, "request": request}
    process.stdin.write(json.dumps(value, sort_keys=True, separators=(",", ":")).encode() + b"\n")
    process.stdin.flush()
    line = process.stdout.readline()
    if not line:
        raise RuntimeError(f"authoring session ended: {process.stderr.read().decode(errors='replace')}")
    decoded = json.loads(line)
    if decoded.get("request_id") != request_id or "response" not in decoded:
        raise RuntimeError(f"authoring request failed: {decoded}")
    return decoded["response"]


def expect(response, kind):
    if response.get("kind") != kind:
        raise RuntimeError(f"expected {kind}, received {response}")
    return response["data"]


def app_target(release, item):
    return {"release": release, "item": item}


def application_value(kind, data=None):
    result_value = {"kind": kind}
    if data is not None:
        result_value["data"] = data
    return result_value


def app_product(release, types, name, fields):
    ty = types[name]
    return application_value("product", {
        "ty": app_target(release, ty["target"]),
        "fields": [
            {"field": app_target(release, ty["fields"][field]), "value": value}
            for field, value in fields
        ],
    })


def app_sum(release, types, name, variant, payload=None):
    ty = types[name]
    data = {
        "ty": app_target(release, ty["target"]),
        "variant": app_target(release, ty["variants"][variant]),
    }
    if payload is not None:
        data["payload"] = payload
    return application_value("sum", data)


def app_sequence(release, types, name, elements):
    return application_value("sequence", {
        "ty": app_target(release, types[name]["target"]),
        "elements": elements,
    })


def text(value):
    return application_value("text", value)


def integer(value):
    return application_value("i64", value)


def boolean(value):
    return application_value("bool", value)


def byte_string(value):
    return application_value("bytes", base64.urlsafe_b64encode(value).rstrip(b"=").decode())


def initial_project(release, types, name):
    return app_product(release, types, "project", [
        ("name", text(name)),
        ("next_task_id", integer(1)),
        ("next_note_id", integer(1)),
        ("tasks", app_sequence(release, types, "task_sequence", [])),
        ("activity", app_sequence(release, types, "activity_sequence", [])),
        ("pending_attachment", app_sum(release, types, "pending_attachment_option", "none")),
    ])


def response(release, types, variant, task, code):
    detail = app_product(release, types, "response_detail", [("task", integer(task)), ("code", text(code))])
    return app_sum(release, types, "mutation_response", variant, detail)


def declined(release, types, response_value):
    payload = app_product(release, types, "declined_payload", [("response", response_value)])
    return app_sum(release, types, "mutation_decision", "declined", payload)


def unchanged(release, types, response_value):
    payload = app_product(release, types, "unchanged_payload", [("response", response_value)])
    return app_sum(release, types, "mutation_decision", "unchanged", payload)


def completed(release, types, state, response_value):
    payload = app_product(release, types, "completed_payload", [("state", state), ("response", response_value)])
    return app_sum(release, types, "mutation_decision", "completed", payload)


def suspended(release, types, state, response_value, command_value):
    payload = app_product(release, types, "suspended_payload", [
        ("state", state),
        ("response", response_value),
        ("command", command_value),
    ])
    return app_sum(release, types, "mutation_decision", "suspended", payload)


def export_types(receipt):
    result_types = {}
    for exported in receipt["inspection"]["exports"]:
        signature = exported["signature"]["data"]
        item = {"target": exported["target"]}
        if "fields" in signature:
            item["fields"] = {value["name"]: value["target"] for value in signature["fields"]}
        if "variants" in signature:
            item["variants"] = {value["name"]: value["target"] for value in signature["variants"]}
        result_types[exported["name"]] = item
    return result_types


def reproduce(cli, output, bindings_output):
    with tempfile.TemporaryDirectory(prefix="lkjwork-build-") as temporary:
        root = pathlib.Path(temporary).resolve()
        state = root / "state"
        state.mkdir(mode=0o700)
        author = subprocess.Popen(
            [str(cli), "--state", str(state), "session"],
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
        )
        workspace = expect(rpc(author, 1, {"kind": "create_workspace"}), "workspace_created")["workspace"]
        returned = [
            "lkjwork_package", "project", "mutation_event", "mutation_response", "query", "query_result",
            "host_command", "host_outcome", "declined_payload", "unchanged_payload", "completed_payload",
            "suspended_payload", "mutation_decision", "transition_event", "transition_resume", "query_entry",
            "identity_text",
            "task", "task_sequence", "task_view", "task_view_sequence", "task_phase", "task_hold",
            "text_sequence", "id_sequence", "note",
            "note_sequence", "attachment", "attachment_sequence", "activity", "activity_sequence",
            "pending_attachment", "pending_attachment_option", "create_task_input", "edit_task_input",
            "hold_input", "priority_input", "label_input", "dependency_input", "note_input",
            "attachment_input", "response_detail", "page_request", "list_request", "phase_filter",
            "readiness_filter", "label_filter", "archive_filter", "task_order", "context_request", "task_page",
            "context_result", "activity_filter", "activity_request", "activity_page",
            "project_summary", "blob_request", "blob_outcome",
        ]
        transaction = expect(rpc(author, 2, {
            "kind": "apply_transaction",
            "data": {
                "transaction": {
                    "workspace": workspace,
                    "base_revision": 0,
                    "mode": "commit",
                    "operations": semantic_operations(),
                },
                "response": {"return_symbols": returned},
            },
        }), "transaction_receipt")
        author.stdin.close()
        if author.wait(timeout=30) != 0:
            raise RuntimeError(author.stderr.read().decode(errors="replace"))
        identifiers = dict(transaction["returned_bindings"])
        exports = [
            {"name": name, "target": identifiers[name]}
            for name in returned
            if name != "lkjwork_package"
        ]
        release_path = root / "lkjwork.lkjr"
        release_receipt = run_json([
            str(cli), "release", "build", "--state", str(state), "--output", str(release_path)
        ], {
            "version": RELEASE_VERSION,
            "workspace": workspace,
            "revision": transaction["revision"],
            "root": identifiers["lkjwork_package"],
            "coordinate": "applications/lkjwork",
            "user_version": "1.0.0",
            "exports": exports,
            "dependencies": [],
            "imports": [],
            "tests": [{
                "name": "identity_text",
                "target": identifiers["identity_text"],
                "arguments": [text("lkjwork")],
                "expected": {"kind": "value", "data": text("lkjwork")},
                "policy": {"fuel": 1000, "maximum_frames": 32},
            }],
        })
        release = release_receipt["inspection"]["release"]
        types = export_types(release_receipt)
        state_value = initial_project(release, types, "test")
        create_input = app_product(release, types, "create_task_input", [
            ("title", text("first task")),
            ("description", text("")),
            ("priority", integer(0)),
            ("labels", app_sequence(release, types, "text_sequence", [])),
            ("dependencies", app_sequence(release, types, "id_sequence", [])),
            ("actor", text("test")),
        ])
        create_event = app_sum(release, types, "mutation_event", "create_task", create_input)
        task_value = app_product(release, types, "task", [
            ("id", integer(1)),
            ("title", text("first task")),
            ("description", text("")),
            ("phase", app_sum(release, types, "task_phase", "planned")),
            ("hold", app_sum(release, types, "task_hold", "none")),
            ("priority", integer(0)),
            ("labels", app_sequence(release, types, "text_sequence", [])),
            ("dependencies", app_sequence(release, types, "id_sequence", [])),
            ("notes", app_sequence(release, types, "note_sequence", [])),
            ("attachments", app_sequence(release, types, "attachment_sequence", [])),
            ("archived", boolean(False)),
        ])
        activity_value = app_product(release, types, "activity", [
            ("task", integer(1)), ("actor", text("test")), ("code", text("task_created")),
        ])
        created_state = app_product(release, types, "project", [
            ("name", text("test")),
            ("next_task_id", integer(2)),
            ("next_note_id", integer(1)),
            ("tasks", app_sequence(release, types, "task_sequence", [task_value])),
            ("activity", app_sequence(release, types, "activity_sequence", [activity_value])),
            ("pending_attachment", app_sum(release, types, "pending_attachment_option", "none")),
        ])
        rename_same_event = app_sum(release, types, "mutation_event", "rename_project", text("test"))
        attachment_content = b"proof"
        attachment_input = app_product(release, types, "attachment_input", [
            ("task", integer(1)),
            ("name", text("evidence.txt")),
            ("actor", text("test")),
            ("content", byte_string(attachment_content)),
        ])
        attachment_event = app_sum(
            release, types, "mutation_event", "request_attachment", attachment_input
        )
        pending_attachment = app_product(release, types, "pending_attachment", [
            ("task", integer(1)),
            ("name", text("evidence.txt")),
            ("actor", text("test")),
            ("content", byte_string(attachment_content)),
            ("digest", byte_string(b"")),
        ])
        attachment_pending_state = app_product(release, types, "project", [
            ("name", text("test")),
            ("next_task_id", integer(2)),
            ("next_note_id", integer(1)),
            ("tasks", app_sequence(release, types, "task_sequence", [task_value])),
            ("activity", app_sequence(release, types, "activity_sequence", [activity_value])),
            ("pending_attachment", app_sum(
                release, types, "pending_attachment_option", "some", pending_attachment
            )),
        ])
        attachment_command = app_sum(
            release,
            types,
            "host_command",
            "blob",
            app_sum(release, types, "blob_request", "put", byte_string(attachment_content)),
        )
        list_query = app_sum(
            release,
            types,
            "query",
            "list_tasks",
            app_product(release, types, "list_request", [
                ("after", integer(0)),
                ("limit", integer(20)),
                ("phase", app_sum(release, types, "phase_filter", "any")),
                ("readiness", app_sum(release, types, "readiness_filter", "any")),
                ("label", app_sum(release, types, "label_filter", "any")),
                ("archive", app_sum(release, types, "archive_filter", "default")),
                ("order", app_sum(release, types, "task_order", "id")),
            ]),
        )
        empty_page = app_product(release, types, "task_page", [
            ("tasks", app_sequence(release, types, "task_view_sequence", [])),
            ("total", integer(0)),
            ("omitted", integer(0)),
            ("next_after", integer(0)),
        ])
        blob_outcome = app_sum(
            release,
            types,
            "host_outcome",
            "blob",
            app_sum(release, types, "blob_outcome", "put_failed", byte_string(b"failure")),
        )
        target = lambda name: app_target(release, types[name]["target"])
        application_request = {
            "version": APPLICATION_VERSION,
            "root_release": release,
            "entry": target("transition_event"),
            "profile": {"kind": "stateful", "data": {
                "resume": target("transition_resume"),
                "query_entry": target("query_entry"),
                "state": target("project"),
                "event": target("mutation_event"),
                "response": target("mutation_response"),
                "query": target("query"),
                "query_result": target("query_result"),
                "command": target("host_command"),
                "outcome": target("host_outcome"),
                "decision": target("mutation_decision"),
                "declined_variant": app_target(release, types["mutation_decision"]["variants"]["declined"]),
                "declined_payload": target("declined_payload"),
                "declined_response_field": app_target(release, types["declined_payload"]["fields"]["response"]),
                "unchanged_variant": app_target(release, types["mutation_decision"]["variants"]["unchanged"]),
                "unchanged_payload": target("unchanged_payload"),
                "unchanged_response_field": app_target(release, types["unchanged_payload"]["fields"]["response"]),
                "completed_variant": app_target(release, types["mutation_decision"]["variants"]["completed"]),
                "completed_payload": target("completed_payload"),
                "completed_state_field": app_target(release, types["completed_payload"]["fields"]["state"]),
                "completed_response_field": app_target(release, types["completed_payload"]["fields"]["response"]),
                "suspended_variant": app_target(release, types["mutation_decision"]["variants"]["suspended"]),
                "suspended_payload": target("suspended_payload"),
                "suspended_state_field": app_target(release, types["suspended_payload"]["fields"]["state"]),
                "suspended_response_field": app_target(release, types["suspended_payload"]["fields"]["response"]),
                "suspended_command_field": app_target(release, types["suspended_payload"]["fields"]["command"]),
                "imports": [{
                    "slot": "attachments",
                    "interface": "immutable_blob",
                    "request": target("blob_request"),
                    "outcome": target("blob_outcome"),
                    "command_variant": app_target(release, types["host_command"]["variants"]["blob"]),
                    "outcome_variant": app_target(release, types["host_outcome"]["variants"]["blob"]),
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
            "policy": {"fuel": 100000000, "maximum_frames": 1024},
            "tests": [
                {
                    "name": "create_first_task",
                    "target": target("transition_event"),
                    "arguments": [state_value, create_event],
                    "expected": {"kind": "value", "data": completed(
                        release, types, created_state, response(release, types, "accepted", 1, "task_created")
                    )},
                    "policy": {"fuel": 1000000, "maximum_frames": 256},
                },
                {
                    "name": "list_empty_project",
                    "target": target("query_entry"),
                    "arguments": [state_value, list_query],
                    "expected": {"kind": "value", "data": app_sum(release, types, "query_result", "tasks", empty_page)},
                    "policy": {"fuel": 1000000, "maximum_frames": 256},
                },
                {
                    "name": "host_outcome_without_pending_request_declines",
                    "target": target("transition_resume"),
                    "arguments": [state_value, blob_outcome],
                    "expected": {"kind": "value", "data": declined(
                        release, types, response(
                            release, types, "conflict", 0, "attachment_outcome_without_pending_request"
                        )
                    )},
                    "policy": {"fuel": 1000000, "maximum_frames": 256},
                },
                {
                    "name": "rename_same_project_is_unchanged",
                    "target": target("transition_event"),
                    "arguments": [state_value, rename_same_event],
                    "expected": {"kind": "value", "data": unchanged(
                        release,
                        types,
                        response(release, types, "no_change", 0, "project_name_unchanged"),
                    )},
                    "policy": {"fuel": 1000000, "maximum_frames": 256},
                },
                {
                    "name": "request_attachment_suspends",
                    "target": target("transition_event"),
                    "arguments": [created_state, attachment_event],
                    "expected": {"kind": "value", "data": suspended(
                        release,
                        types,
                        attachment_pending_state,
                        response(release, types, "accepted", 1, "attachment_pending"),
                        attachment_command,
                    )},
                    "policy": {"fuel": 1000000, "maximum_frames": 256},
                },
            ],
        }
        generated_application = root / "lkjwork.lkja"
        application_receipt = run_json([
            str(cli), "app", "build", "--release", str(release_path), "--output", str(generated_application)
        ], application_request)
        bindings = {
            "contract_version": 1,
            "application_digest": application_receipt["inspection"]["digest"],
            "release": release,
            "types": types,
        }
        output.parent.mkdir(parents=True, exist_ok=True)
        bindings_output.parent.mkdir(parents=True, exist_ok=True)
        output.write_bytes(generated_application.read_bytes())
        bindings_output.write_text(json.dumps(bindings, sort_keys=True, separators=(",", ":")) + "\n")
        return application_receipt["inspection"]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("lkjscript", type=pathlib.Path)
    parser.add_argument("--output", type=pathlib.Path, default=pathlib.Path(__file__).with_name("lkjwork.lkja"))
    parser.add_argument("--bindings", type=pathlib.Path, default=pathlib.Path(__file__).with_name("bindings.json"))
    arguments = parser.parse_args()
    inspection = reproduce(arguments.lkjscript.resolve(), arguments.output.resolve(), arguments.bindings.resolve())
    print(json.dumps({
        "application_digest": inspection["digest"],
        "artifact_bytes": inspection["artifact_bytes"],
        "semantic_items": inspection["flattened_semantic_items"],
    }, sort_keys=True, separators=(",", ":")))


if __name__ == "__main__":
    main()
