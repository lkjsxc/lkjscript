//! Deterministic compact records for the copied-binary stateful HTTP acceptance application.
//!
//! This module constructs only public compact records from identities obtained through public
//! discovery. It does not open a repository or construct semantic owners through Rust APIs.

use crate::error::DevError;
use lkjscript::platform::control::render_record;
use std::collections::BTreeMap;

pub(crate) const MIGRATION_ID: i64 = 1;
pub(crate) const MAXIMUM_REQUEST_JSON_BYTES: i64 = 65_536;
pub(crate) const MIGRATION_STATEMENT: &str = "CREATE TABLE IF NOT EXISTS bbs_posts (id TEXT PRIMARY KEY, author TEXT NOT NULL CHECK (char_length(author) BETWEEN 1 AND 64), body TEXT NOT NULL CHECK (char_length(body) BETWEEN 1 AND 4096), created_at BIGINT NOT NULL, updated_at BIGINT NOT NULL)";

const LIST_STATEMENT: &str = "SELECT COALESCE(json_agg(json_build_object('id', id, 'author', author, 'body', body, 'created', created_at, 'updated', updated_at) ORDER BY created_at, id), '[]'::json)::text FROM bbs_posts";
const CREATE_STATEMENT: &str = "INSERT INTO bbs_posts (id, author, body, created_at, updated_at) VALUES ($1, $2, $3, $4, $5) RETURNING json_build_object('id', id, 'author', author, 'body', body, 'created', created_at, 'updated', updated_at)::text";
const UPDATE_STATEMENT: &str = "UPDATE bbs_posts SET author = $2, body = $3, updated_at = $4 WHERE id = $1 RETURNING json_build_object('id', id, 'author', author, 'body', body, 'created', created_at, 'updated', updated_at)::text";
const DELETE_STATEMENT: &str = "DELETE FROM bbs_posts WHERE id = $1 RETURNING id";

#[derive(Clone, Debug)]
pub(crate) struct StandardReferences {
    pub(crate) declarations: BTreeMap<String, String>,
    pub(crate) interfaces: BTreeMap<String, String>,
    pub(crate) operations: BTreeMap<String, String>,
    pub(crate) cases: BTreeMap<String, String>,
}

impl StandardReferences {
    fn declaration(&self, name: &str) -> Result<&str, DevError> {
        self.declarations
            .get(name)
            .map(String::as_str)
            .ok_or_else(|| {
                DevError::corrupt(format!("standard discovery omitted declaration '{name}'"))
            })
    }

    fn interface(&self, name: &str) -> Result<&str, DevError> {
        self.interfaces
            .get(name)
            .map(String::as_str)
            .ok_or_else(|| {
                DevError::corrupt(format!("standard discovery omitted interface '{name}'"))
            })
    }

    fn operation(&self, interface: &str, name: &str) -> Result<&str, DevError> {
        let key = format!("{interface}.{name}");
        self.operations
            .get(&key)
            .map(String::as_str)
            .ok_or_else(|| {
                DevError::corrupt(format!("standard discovery omitted operation '{key}'"))
            })
    }

    fn case(&self, variant: &str, name: &str) -> Result<&str, DevError> {
        let key = format!("{variant}.{name}");
        self.cases
            .get(&key)
            .map(String::as_str)
            .ok_or_else(|| DevError::corrupt(format!("standard discovery omitted case '{key}'")))
    }
}

#[derive(Clone, Debug)]
pub(crate) struct ProjectReferences {
    pub(crate) base_revision: String,
    pub(crate) package: String,
    pub(crate) component: String,
    pub(crate) handler: String,
    pub(crate) request_parameter: String,
    pub(crate) streams_requirement: String,
}

#[derive(Clone, Debug)]
pub(crate) struct ProgramRequest {
    pub(crate) bytes: Vec<u8>,
    pub(crate) records: usize,
    pub(crate) migration_checksum: String,
}

#[derive(Debug)]
struct Builder<'a> {
    standard: &'a StandardReferences,
    project: &'a ProjectReferences,
    records: Vec<String>,
    next_expression: u64,
    next_binding: u64,
}

impl<'a> Builder<'a> {
    fn new(standard: &'a StandardReferences, project: &'a ProjectReferences) -> Self {
        Self {
            standard,
            project,
            records: Vec::new(),
            next_expression: 0,
            next_binding: 0,
        }
    }

    fn record(&mut self, operation: &str, fields: Vec<(&str, String)>) -> Result<(), DevError> {
        let borrowed = fields
            .iter()
            .map(|(name, value)| (*name, value.as_str()))
            .collect::<Vec<_>>();
        self.records.push(
            render_record(operation, &borrowed)
                .map_err(|error| DevError::corrupt(format!("render {operation}: {error}")))?,
        );
        Ok(())
    }

    fn expression_label(&mut self) -> String {
        self.next_expression = self.next_expression.saturating_add(1);
        format!("$e{:05}", self.next_expression)
    }

    fn binding_label(&mut self) -> String {
        self.next_binding = self.next_binding.saturating_add(1);
        format!("$b{:05}", self.next_binding)
    }

    fn type_named(&mut self, symbol: &str, declaration: &str) -> Result<(), DevError> {
        self.record(
            "type.named",
            vec![
                ("as", symbol.to_owned()),
                ("declaration", declaration.to_owned()),
            ],
        )
    }

    fn type_list(&mut self, symbol: &str, item: &str) -> Result<(), DevError> {
        self.record(
            "type.list",
            vec![("as", symbol.to_owned()), ("item", item.to_owned())],
        )
    }

    fn type_stream(&mut self, symbol: &str, item: &str) -> Result<(), DevError> {
        self.record(
            "type.stream",
            vec![("as", symbol.to_owned()), ("item", item.to_owned())],
        )
    }

    fn type_structural(&mut self, symbol: &str, fields: &[(&str, &str)]) -> Result<(), DevError> {
        self.record("type.structural-record", vec![("as", symbol.to_owned())])?;
        for (index, (name, ty)) in fields.iter().enumerate() {
            self.record(
                "type.field",
                vec![
                    ("parent", symbol.to_owned()),
                    ("index", index.to_string()),
                    ("name", (*name).to_owned()),
                    ("type", (*ty).to_owned()),
                ],
            )?;
        }
        Ok(())
    }

    fn expression(
        &mut self,
        form: &str,
        mut fields: Vec<(&str, String)>,
    ) -> Result<String, DevError> {
        let label = self.expression_label();
        fields.insert(0, ("as", label.clone()));
        self.record(&format!("expression.{form}"), fields)?;
        Ok(label)
    }

    fn boolean(&mut self, value: bool) -> Result<String, DevError> {
        self.expression("bool", vec![("value", value.to_string())])
    }

    fn i64(&mut self, value: i64) -> Result<String, DevError> {
        self.expression("i64", vec![("value", value.to_string())])
    }

    fn text(&mut self, value: &str) -> Result<String, DevError> {
        self.expression("text", vec![("value", value.to_owned())])
    }

    fn static_text(&mut self, value: &str) -> Result<String, DevError> {
        self.expression("static-text", vec![("value", value.to_owned())])
    }

    fn local(&mut self, value: &str) -> Result<String, DevError> {
        self.expression("local", vec![("value", value.to_owned())])
    }

    fn field_name(&mut self, value: String, name: &str) -> Result<String, DevError> {
        self.expression("field", vec![("value", value), ("name", name.to_owned())])
    }

    fn field_nominal(&mut self, value: String, field: &str) -> Result<String, DevError> {
        self.expression("field", vec![("value", value), ("field", field.to_owned())])
    }

    fn call(
        &mut self,
        function: &str,
        type_arguments: &[&str],
        arguments: Vec<String>,
    ) -> Result<String, DevError> {
        let label = self.expression("call", vec![("function", function.to_owned())])?;
        for (index, ty) in type_arguments.iter().enumerate() {
            self.record(
                "type.argument",
                vec![
                    ("parent", label.clone()),
                    ("index", index.to_string()),
                    ("type", (*ty).to_owned()),
                ],
            )?;
        }
        self.expression_arguments(&label, arguments)?;
        Ok(label)
    }

    fn capability_call(
        &mut self,
        requirement: &str,
        operation: &str,
        arguments: Vec<String>,
    ) -> Result<String, DevError> {
        let label = self.expression(
            "capability-call",
            vec![
                ("requirement", requirement.to_owned()),
                ("operation", operation.to_owned()),
            ],
        )?;
        self.expression_arguments(&label, arguments)?;
        Ok(label)
    }

    fn expression_arguments(
        &mut self,
        parent: &str,
        arguments: Vec<String>,
    ) -> Result<(), DevError> {
        for (index, expression) in arguments.into_iter().enumerate() {
            self.record(
                "expression.argument",
                vec![
                    ("parent", parent.to_owned()),
                    ("index", index.to_string()),
                    ("expression", expression),
                ],
            )?;
        }
        Ok(())
    }

    fn if_expression(
        &mut self,
        condition: String,
        when_true: String,
        when_false: String,
    ) -> Result<String, DevError> {
        self.expression(
            "if",
            vec![
                ("condition", condition),
                ("when-true", when_true),
                ("when-false", when_false),
            ],
        )
    }

    fn sequence(&mut self, items: Vec<String>) -> Result<String, DevError> {
        let label = self.expression("sequence", vec![])?;
        self.expression_arguments(&label, items)?;
        Ok(label)
    }

    fn let_one(
        &mut self,
        name: &str,
        value: String,
        ty: Option<&str>,
        body: impl FnOnce(&mut Self, &str) -> Result<String, DevError>,
    ) -> Result<String, DevError> {
        let binding = self.binding_label();
        let body = body(self, &binding)?;
        let label = self.expression("let", vec![("body", body)])?;
        let mut fields = vec![
            ("parent", label.clone()),
            ("index", "0".to_owned()),
            ("as", binding),
            ("name", name.to_owned()),
            ("value", value),
        ];
        if let Some(ty) = ty {
            fields.push(("type", ty.to_owned()));
        }
        self.record("expression.binding", fields)?;
        Ok(label)
    }

    fn structural_record(&mut self, fields: Vec<(&str, String)>) -> Result<String, DevError> {
        self.record_expression(None, fields, false)
    }

    fn nominal_record(
        &mut self,
        declaration: &str,
        fields: Vec<(&str, String)>,
    ) -> Result<String, DevError> {
        self.record_expression(Some(declaration), fields, true)
    }

    fn record_expression(
        &mut self,
        declaration: Option<&str>,
        fields: Vec<(&str, String)>,
        nominal: bool,
    ) -> Result<String, DevError> {
        let mut root = Vec::new();
        if let Some(declaration) = declaration {
            root.push(("type", declaration.to_owned()));
        }
        let label = self.expression("record", root)?;
        for (index, (selector, value)) in fields.into_iter().enumerate() {
            let selector_name = if nominal { "field" } else { "name" };
            self.record(
                "expression.record-field",
                vec![
                    ("parent", label.clone()),
                    ("index", index.to_string()),
                    (selector_name, selector.to_owned()),
                    ("value", value),
                ],
            )?;
        }
        Ok(label)
    }

    fn list(&mut self, item_type: &str, items: Vec<String>) -> Result<String, DevError> {
        let label = self.expression("list", vec![("item", item_type.to_owned())])?;
        self.expression_arguments(&label, items)?;
        Ok(label)
    }

    fn variant(&mut self, case: &str, payload: Option<String>) -> Result<String, DevError> {
        let mut fields = vec![("case", case.to_owned())];
        if let Some(payload) = payload {
            fields.push(("payload", payload));
        }
        self.expression("variant", fields)
    }

    fn match_expression(&mut self, value: String, arms: Vec<MatchArm>) -> Result<String, DevError> {
        let label = self.expression("match", vec![("value", value)])?;
        for (index, arm) in arms.into_iter().enumerate() {
            let mut fields = vec![
                ("parent", label.clone()),
                ("index", index.to_string()),
                ("case", arm.case),
                ("body", arm.body),
            ];
            if let Some((binding, name, ty)) = arm.binding {
                fields.push(("as", binding));
                fields.push(("name", name));
                fields.push(("type", ty));
            }
            self.record("expression.match-arm", fields)?;
        }
        Ok(label)
    }

    fn transaction(&mut self, requirement: &str, body: String) -> Result<String, DevError> {
        let binding = self.binding_label();
        self.expression(
            "transaction",
            vec![
                ("requirement", requirement.to_owned()),
                ("binding", binding),
                ("name", "transaction".to_owned()),
                ("body", body),
            ],
        )
    }

    fn create_function(
        &mut self,
        symbol: &str,
        name: &str,
        result: &str,
        requirements: &[&str],
        body: String,
        parameters: &[(&str, &str, &str)],
    ) -> Result<(), DevError> {
        let effect = if requirements.is_empty() {
            "pure"
        } else {
            "task"
        };
        self.record(
            "create.function",
            vec![
                ("as", symbol.to_owned()),
                ("module", "$bbs".to_owned()),
                ("name", name.to_owned()),
                ("visibility", "private".to_owned()),
                ("result", result.to_owned()),
                ("effect", effect.to_owned()),
                ("body", body),
            ],
        )?;
        for (index, requirement) in requirements.iter().enumerate() {
            self.record(
                "effect.requirement",
                vec![
                    ("parent", symbol.to_owned()),
                    ("index", index.to_string()),
                    ("requirement", (*requirement).to_owned()),
                ],
            )?;
        }
        for (parameter, name, ty) in parameters {
            self.record(
                "add.parameter",
                vec![
                    ("as", (*parameter).to_owned()),
                    ("function", symbol.to_owned()),
                    ("name", (*name).to_owned()),
                    ("type", (*ty).to_owned()),
                ],
            )?;
        }
        Ok(())
    }

    fn external_call(&mut self, name: &str, arguments: Vec<String>) -> Result<String, DevError> {
        let function = self.standard.declaration(name)?.to_owned();
        self.call(&function, &[], arguments)
    }

    fn generic_external_call(
        &mut self,
        name: &str,
        ty: &str,
        arguments: Vec<String>,
    ) -> Result<String, DevError> {
        let function = self.standard.declaration(name)?.to_owned();
        self.call(&function, &[ty], arguments)
    }

    fn finish(self, migration_checksum: String) -> ProgramRequest {
        let records = self.records.len();
        ProgramRequest {
            bytes: self.records.concat().into_bytes(),
            records,
            migration_checksum,
        }
    }
}

#[derive(Clone, Debug)]
struct MatchArm {
    case: String,
    binding: Option<(String, String, String)>,
    body: String,
}

impl MatchArm {
    fn plain(case: &str, body: String) -> Self {
        Self {
            case: case.to_owned(),
            binding: None,
            body,
        }
    }

    fn payload(case: &str, binding: String, name: &str, ty: &str, body: String) -> Self {
        Self {
            case: case.to_owned(),
            binding: Some((binding, name.to_owned(), ty.to_owned())),
            body,
        }
    }
}

pub(crate) fn build_program_request(
    standard: &StandardReferences,
    project: &ProjectReferences,
) -> Result<ProgramRequest, DevError> {
    let mut builder = Builder::new(standard, project);
    builder.record(
        "request",
        vec![
            ("base", project.base_revision.clone()),
            ("idempotency", "stateful-http-bbs-1".to_owned()),
            (
                "intent",
                "author persistent request-dependent BBS through public compact changes".to_owned(),
            ),
        ],
    )?;
    add_types_and_domain(&mut builder)?;
    add_capability_requirements(&mut builder)?;
    add_response_functions(&mut builder)?;
    add_validation_and_route_functions(&mut builder)?;
    add_sql_conversion_functions(&mut builder)?;
    add_persistence_functions(&mut builder)?;
    add_handler(&mut builder)?;
    add_graph_tests(&mut builder)?;
    let checksum = blake3::hash(MIGRATION_STATEMENT.as_bytes())
        .to_hex()
        .to_string();
    Ok(builder.finish(checksum))
}

fn add_types_and_domain(builder: &mut Builder<'_>) -> Result<(), DevError> {
    builder.record(
        "create.module",
        vec![("as", "$bbs".to_owned()), ("name", "bbs".to_owned())],
    )?;
    builder.record(
        "create.record",
        vec![
            ("as", "$post".to_owned()),
            ("module", "$bbs".to_owned()),
            ("name", "Post".to_owned()),
            ("visibility", "private".to_owned()),
        ],
    )?;
    for (symbol, name, ty) in [
        ("$post_id", "id", "text"),
        ("$post_author", "author", "text"),
        ("$post_body", "body", "text"),
        ("$post_created", "created", "i64"),
        ("$post_updated", "updated", "i64"),
    ] {
        builder.record(
            "add.field",
            vec![
                ("as", symbol.to_owned()),
                ("record", "$post".to_owned()),
                ("name", name.to_owned()),
                ("type", ty.to_owned()),
            ],
        )?;
    }
    builder.record(
        "create.record",
        vec![
            ("as", "$write_post".to_owned()),
            ("module", "$bbs".to_owned()),
            ("name", "WritePost".to_owned()),
            ("visibility", "private".to_owned()),
        ],
    )?;
    for (symbol, name) in [("$write_author", "author"), ("$write_body", "body")] {
        builder.record(
            "add.field",
            vec![
                ("as", symbol.to_owned()),
                ("record", "$write_post".to_owned()),
                ("name", name.to_owned()),
                ("type", "text".to_owned()),
            ],
        )?;
    }
    for (symbol, name, cases) in [
        (
            "$maybe_text",
            "MaybeText",
            vec![
                ("$maybe_text_missing", "Missing", None),
                ("$maybe_text_found", "Found", Some("text")),
            ],
        ),
        (
            "$post_result",
            "PostResult",
            vec![
                ("$post_result_missing", "Missing", None),
                ("$post_result_found", "Found", Some("@post")),
                ("$post_result_corrupt", "Corrupt", None),
            ],
        ),
        (
            "$posts_result",
            "PostsResult",
            vec![
                ("$posts_result_found", "Found", Some("@posts")),
                ("$posts_result_corrupt", "Corrupt", None),
            ],
        ),
        (
            "$route",
            "Route",
            vec![
                ("$route_home", "Home", None),
                ("$route_list", "List", None),
                ("$route_create", "Create", None),
                ("$route_update", "Update", None),
                ("$route_delete", "Delete", None),
                ("$route_method_not_allowed", "MethodNotAllowed", None),
                ("$route_missing", "NotFound", None),
            ],
        ),
    ] {
        builder.record(
            "create.variant",
            vec![
                ("as", symbol.to_owned()),
                ("module", "$bbs".to_owned()),
                ("name", name.to_owned()),
                ("visibility", "private".to_owned()),
            ],
        )?;
        for (case_symbol, case_name, payload) in cases {
            let mut fields = vec![
                ("as", case_symbol.to_owned()),
                ("variant", symbol.to_owned()),
                ("name", case_name.to_owned()),
            ];
            if let Some(payload) = payload {
                fields.push(("payload", payload.to_owned()));
            }
            builder.record("add.case", fields)?;
        }
    }

    builder.type_named("@post", "$post")?;
    builder.type_named("@write_post", "$write_post")?;
    builder.type_named("@maybe_text", "$maybe_text")?;
    builder.type_named("@post_result", "$post_result")?;
    builder.type_named("@posts_result", "$posts_result")?;
    builder.type_named("@route", "$route")?;
    builder.type_list("@posts", "@post")?;
    builder.type_named("@sql_value", builder.standard.declaration("SqlValue")?)?;
    builder.type_named("@sql_type", builder.standard.declaration("SqlType")?)?;
    builder.type_list("@sql_values", "@sql_value")?;
    builder.type_list("@sql_row", "@sql_value")?;
    builder.type_list("@sql_rows", "@sql_row")?;
    builder.type_list("@sql_types", "@sql_type")?;
    builder.type_structural("@header", &[("name", "text"), ("value", "bytes")])?;
    builder.type_list("@headers", "@header")?;
    builder.type_stream("@body_stream", "bytes")?;
    builder.type_structural(
        "@response",
        &[
            ("body", "bytes"),
            ("headers", "@headers"),
            ("status", "i64"),
        ],
    )?;
    Ok(())
}

fn add_capability_requirements(builder: &mut Builder<'_>) -> Result<(), DevError> {
    let definitions = [
        (
            "$database",
            "database",
            "Database",
            vec!["execute", "migration", "transaction", "query"],
            vec![
                ("maximum_calls", 128_u64, "calls"),
                ("maximum_rows", 128, "items"),
            ],
        ),
        (
            "$identifiers",
            "identifiers",
            "Identifier",
            vec!["uuid-v4"],
            vec![("maximum_calls", 8_u64, "calls")],
        ),
        (
            "$clock",
            "clock",
            "WallClock",
            vec!["utc-milliseconds"],
            vec![("maximum_calls", 8_u64, "calls")],
        ),
    ];
    for (symbol, name, interface, operations, limits) in definitions {
        builder.record(
            "add.requirement",
            vec![
                ("as", symbol.to_owned()),
                ("component", builder.project.component.clone()),
                ("name", name.to_owned()),
                (
                    "interface",
                    builder.standard.interface(interface)?.to_owned(),
                ),
            ],
        )?;
        for (index, operation) in operations.iter().enumerate() {
            builder.record(
                "requirement.operation",
                vec![
                    ("parent", symbol.to_owned()),
                    ("index", index.to_string()),
                    (
                        "operation",
                        builder.standard.operation(interface, operation)?.to_owned(),
                    ),
                ],
            )?;
        }
        for (index, (limit, maximum, unit)) in limits.iter().enumerate() {
            builder.record(
                "requirement.limit",
                vec![
                    ("parent", symbol.to_owned()),
                    ("index", index.to_string()),
                    ("name", (*limit).to_owned()),
                    ("maximum", maximum.to_string()),
                    ("unit", (*unit).to_owned()),
                ],
            )?;
        }
    }
    Ok(())
}

fn add_response_functions(builder: &mut Builder<'_>) -> Result<(), DevError> {
    let header_name = builder.text("content-type")?;
    let content_type = builder.local("$response_content_type")?;
    let header_value = builder.external_call("bytes-from-text", vec![content_type])?;
    let header = builder.structural_record(vec![("name", header_name), ("value", header_value)])?;
    let headers = builder.list("@header", vec![header])?;
    let body_value = builder.local("$response_body")?;
    let status_value = builder.local("$response_status")?;
    let response = builder.structural_record(vec![
        ("body", body_value),
        ("headers", headers),
        ("status", status_value),
    ])?;
    builder.create_function(
        "$make_response",
        "make-response",
        "@response",
        &[],
        response,
        &[
            ("$response_status", "status", "i64"),
            ("$response_content_type", "content-type", "text"),
            ("$response_body", "body", "bytes"),
        ],
    )?;

    let text_value = builder.local("$text_response_text")?;
    let bytes = builder.external_call("bytes-from-text", vec![text_value])?;
    let status = builder.local("$text_response_status")?;
    let content_type = builder.local("$text_response_content_type")?;
    let response = builder.call("$make_response", &[], vec![status, content_type, bytes])?;
    builder.create_function(
        "$text_response",
        "text-response",
        "@response",
        &[],
        response,
        &[
            ("$text_response_status", "status", "i64"),
            ("$text_response_content_type", "content-type", "text"),
            ("$text_response_text", "text", "text"),
        ],
    )?;

    let empty_text = builder.text("")?;
    let empty_body = builder.external_call("bytes-from-text", vec![empty_text])?;
    let empty_headers = builder.list("@header", vec![])?;
    let empty_status = builder.local("$empty_response_status")?;
    let empty_response = builder.structural_record(vec![
        ("body", empty_body),
        ("headers", empty_headers),
        ("status", empty_status),
    ])?;
    builder.create_function(
        "$empty_response",
        "empty-response",
        "@response",
        &[],
        empty_response,
        &[("$empty_response_status", "status", "i64")],
    )?;
    Ok(())
}

fn add_validation_and_route_functions(builder: &mut Builder<'_>) -> Result<(), DevError> {
    let author_record = builder.local("$valid_write_value")?;
    let author = builder.field_nominal(author_record, "$write_author")?;
    let author_for_empty = author;
    let author_empty = builder.external_call("text-empty", vec![author_for_empty])?;
    let author_nonempty = builder.external_call("bool-not", vec![author_empty])?;
    let author_record = builder.local("$valid_write_value")?;
    let author = builder.field_nominal(author_record, "$write_author")?;
    let author_length = builder.external_call("text-length", vec![author])?;
    let author_maximum = builder.i64(64)?;
    let author_bounded =
        builder.external_call("less-equal", vec![author_length, author_maximum])?;
    let author_valid = builder.external_call("bool-and", vec![author_nonempty, author_bounded])?;

    let body_record = builder.local("$valid_write_value")?;
    let body = builder.field_nominal(body_record, "$write_body")?;
    let body_empty = builder.external_call("text-empty", vec![body])?;
    let body_nonempty = builder.external_call("bool-not", vec![body_empty])?;
    let body_record = builder.local("$valid_write_value")?;
    let body = builder.field_nominal(body_record, "$write_body")?;
    let body_length = builder.external_call("text-length", vec![body])?;
    let body_maximum = builder.i64(4096)?;
    let body_bounded = builder.external_call("less-equal", vec![body_length, body_maximum])?;
    let body_valid = builder.external_call("bool-and", vec![body_nonempty, body_bounded])?;
    let valid = builder.external_call("bool-and", vec![author_valid, body_valid])?;
    builder.create_function(
        "$valid_write",
        "valid-write-post",
        "bool",
        &[],
        valid,
        &[("$valid_write_value", "value", "@write_post")],
    )?;

    let identity = builder.local("$valid_id_value")?;
    let length = builder.external_call("text-length", vec![identity])?;
    let expected = builder.i64(36)?;
    let valid = builder.external_call("i64-equal", vec![length, expected])?;
    builder.create_function(
        "$valid_id",
        "valid-post-id",
        "bool",
        &[],
        valid,
        &[("$valid_id_value", "value", "text")],
    )?;

    let method_get = route_method_equal(builder, "GET")?;
    let root_code = builder.i64(1)?;
    let method_not_allowed = builder.i64(6)?;
    let root = builder.if_expression(method_get, root_code, method_not_allowed)?;

    let method_delete = route_method_equal(builder, "DELETE")?;
    let delete_code = builder.i64(5)?;
    let method_not_allowed = builder.i64(6)?;
    let api = builder.if_expression(method_delete, delete_code, method_not_allowed)?;
    let method_put = route_method_equal(builder, "PUT")?;
    let put_code = builder.i64(4)?;
    let api = builder.if_expression(method_put, put_code, api)?;
    let method_post = route_method_equal(builder, "POST")?;
    let post_code = builder.i64(3)?;
    let api = builder.if_expression(method_post, post_code, api)?;
    let method_get = route_method_equal(builder, "GET")?;
    let get_code = builder.i64(2)?;
    let api = builder.if_expression(method_get, get_code, api)?;

    let path = builder.local("$route_path")?;
    let api_path = builder.text("/api/posts")?;
    let path_is_api = builder.external_call("text-equal", vec![path, api_path])?;
    let unknown = builder.i64(7)?;
    let non_root = builder.if_expression(path_is_api, api, unknown)?;
    let path = builder.local("$route_path")?;
    let root_path = builder.text("/")?;
    let path_is_root = builder.external_call("text-equal", vec![path, root_path])?;
    let route = builder.if_expression(path_is_root, root, non_root)?;
    builder.create_function(
        "$route_code",
        "route-code",
        "i64",
        &[],
        route,
        &[
            ("$route_method", "method", "text"),
            ("$route_path", "path", "text"),
        ],
    )?;

    let method_get = select_route_method_equal(builder, "GET")?;
    let home = builder.variant("$route_home", None)?;
    let method_not_allowed = builder.variant("$route_method_not_allowed", None)?;
    let root = builder.if_expression(method_get, home, method_not_allowed)?;

    let method_delete = select_route_method_equal(builder, "DELETE")?;
    let delete = builder.variant("$route_delete", None)?;
    let method_not_allowed = builder.variant("$route_method_not_allowed", None)?;
    let api = builder.if_expression(method_delete, delete, method_not_allowed)?;
    let method_put = select_route_method_equal(builder, "PUT")?;
    let update = builder.variant("$route_update", None)?;
    let api = builder.if_expression(method_put, update, api)?;
    let method_post = select_route_method_equal(builder, "POST")?;
    let create = builder.variant("$route_create", None)?;
    let api = builder.if_expression(method_post, create, api)?;
    let method_get = select_route_method_equal(builder, "GET")?;
    let list = builder.variant("$route_list", None)?;
    let api = builder.if_expression(method_get, list, api)?;

    let path = builder.local("$select_route_path")?;
    let api_path = builder.text("/api/posts")?;
    let path_is_api = builder.external_call("text-equal", vec![path, api_path])?;
    let missing = builder.variant("$route_missing", None)?;
    let non_root = builder.if_expression(path_is_api, api, missing)?;
    let path = builder.local("$select_route_path")?;
    let root_path = builder.text("/")?;
    let path_is_root = builder.external_call("text-equal", vec![path, root_path])?;
    let route = builder.if_expression(path_is_root, root, non_root)?;
    builder.create_function(
        "$select_route",
        "select-route",
        "@route",
        &[],
        route,
        &[
            ("$select_route_method", "method", "text"),
            ("$select_route_path", "path", "text"),
        ],
    )?;

    let index = builder.local("$header_index")?;
    let length = builder.local("$header_length")?;
    let within = builder.external_call("less", vec![index, length])?;
    let headers = builder.local("$header_values")?;
    let index = builder.local("$header_index")?;
    let selected = builder.generic_external_call("list-get", "@header", vec![headers, index])?;
    let selected_body =
        builder.let_one("header", selected, Some("@header"), |builder, binding| {
            let header = builder.local(binding)?;
            let name = builder.field_name(header, "name")?;
            let expected_name = builder.text("content-type")?;
            let name_matches = builder.external_call("text-equal", vec![name, expected_name])?;
            let header = builder.local(binding)?;
            let value = builder.field_name(header, "value")?;
            let expected_text = builder.text("application/json")?;
            let expected = builder.external_call("bytes-from-text", vec![expected_text])?;
            let value_matches = builder.external_call("bytes-equal", vec![value, expected])?;
            let matches = builder.external_call("bool-and", vec![name_matches, value_matches])?;
            let yes = builder.boolean(true)?;
            let index = builder.local("$header_index")?;
            let one = builder.i64(1)?;
            let next = builder.external_call("add", vec![index, one])?;
            let headers = builder.local("$header_values")?;
            let length = builder.local("$header_length")?;
            let recurse =
                builder.call("$has_json_content_type", &[], vec![headers, next, length])?;
            builder.if_expression(matches, yes, recurse)
        })?;
    let no = builder.boolean(false)?;
    let header_result = builder.if_expression(within, selected_body, no)?;
    builder.create_function(
        "$has_json_content_type",
        "has-json-content-type",
        "bool",
        &[],
        header_result,
        &[
            ("$header_values", "headers", "@headers"),
            ("$header_index", "index", "i64"),
            ("$header_length", "length", "i64"),
        ],
    )?;
    Ok(())
}

fn route_method_equal(builder: &mut Builder<'_>, expected: &str) -> Result<String, DevError> {
    let method = builder.local("$route_method")?;
    let expected = builder.text(expected)?;
    builder.external_call("text-equal", vec![method, expected])
}

fn select_route_method_equal(
    builder: &mut Builder<'_>,
    expected: &str,
) -> Result<String, DevError> {
    let method = builder.local("$select_route_method")?;
    let expected = builder.text(expected)?;
    builder.external_call("text-equal", vec![method, expected])
}

fn add_sql_conversion_functions(builder: &mut Builder<'_>) -> Result<(), DevError> {
    let value = builder.local("$sql_text_value")?;
    let mut cases = [
        "NullText",
        "I64",
        "Bytes",
        "Text",
        "NullBool",
        "NullBytes",
        "NullI64",
        "Bool",
    ]
    .into_iter()
    .map(|name| Ok((name, builder.standard.case("SqlValue", name)?.to_owned())))
    .collect::<Result<Vec<_>, DevError>>()?;
    cases.sort_by(|left, right| left.1.cmp(&right.1));
    let mut arms = Vec::with_capacity(cases.len());
    for (name, case) in cases {
        let payload = matches!(name, "I64" | "Bytes" | "Text" | "Bool");
        if payload {
            let binding = builder.binding_label();
            let body = if name == "Text" {
                builder.local(&binding)?
            } else {
                builder.local("$sql_text_fallback")?
            };
            let ty = match name {
                "I64" => "i64",
                "Bytes" => "bytes",
                "Text" => "text",
                "Bool" => "bool",
                _ => return Err(DevError::corrupt("unexpected SQL value payload case")),
            };
            arms.push(MatchArm::payload(&case, binding, "payload", ty, body));
        } else {
            let body = builder.local("$sql_text_fallback")?;
            arms.push(MatchArm::plain(&case, body));
        }
    }
    let matched = builder.match_expression(value, arms)?;
    builder.create_function(
        "$sql_text_or",
        "sql-text-or",
        "text",
        &[],
        matched,
        &[
            ("$sql_text_value", "value", "@sql_value"),
            ("$sql_text_fallback", "fallback", "text"),
        ],
    )?;

    let empty = builder.text("")?;
    let zero = builder.i64(0)?;
    let empty_author = builder.text("")?;
    let empty_body = builder.text("")?;
    let zero_updated = builder.i64(0)?;
    let fallback_post = builder.nominal_record(
        "$post",
        vec![
            ("$post_id", empty),
            ("$post_author", empty_author),
            ("$post_body", empty_body),
            ("$post_created", zero),
            ("$post_updated", zero_updated),
        ],
    )?;
    let json = builder.local("$decode_post_json")?;
    let bytes = builder.external_call("bytes-from-text", vec![json])?;
    let decoded =
        builder.generic_external_call("json-decode-or", "@post", vec![bytes, fallback_post])?;
    let decoded_post = builder.let_one("decoded", decoded, None, |builder, binding| {
        let valid_source = builder.local(binding)?;
        let valid = builder.field_name(valid_source, "valid")?;
        let value_source = builder.local(binding)?;
        let value = builder.field_name(value_source, "value")?;
        let found = builder.variant("$post_result_found", Some(value))?;
        let corrupt = builder.variant("$post_result_corrupt", None)?;
        builder.if_expression(valid, found, corrupt)
    })?;
    builder.create_function(
        "$decode_post",
        "decode-post-row",
        "@post_result",
        &[],
        decoded_post,
        &[("$decode_post_json", "json", "text")],
    )?;

    let fallback_posts = builder.list("@post", vec![])?;
    let json = builder.local("$decode_posts_json")?;
    let bytes = builder.external_call("bytes-from-text", vec![json])?;
    let decoded =
        builder.generic_external_call("json-decode-or", "@posts", vec![bytes, fallback_posts])?;
    let decoded_posts = builder.let_one("decoded", decoded, None, |builder, binding| {
        let valid_source = builder.local(binding)?;
        let valid = builder.field_name(valid_source, "valid")?;
        let value_source = builder.local(binding)?;
        let value = builder.field_name(value_source, "value")?;
        let found = builder.variant("$posts_result_found", Some(value))?;
        let corrupt = builder.variant("$posts_result_corrupt", None)?;
        builder.if_expression(valid, found, corrupt)
    })?;
    builder.create_function(
        "$decode_posts",
        "decode-post-list-row",
        "@posts_result",
        &[],
        decoded_posts,
        &[("$decode_posts_json", "json", "text")],
    )?;
    Ok(())
}

fn add_persistence_functions(builder: &mut Builder<'_>) -> Result<(), DevError> {
    let text_case = builder.standard.case("SqlType", "Text")?.to_owned();
    let text_column = builder.variant(&text_case, None)?;
    let columns = builder.list("@sql_type", vec![text_column])?;
    let statement = builder.local("$query_one_statement")?;
    let parameters = builder.local("$query_one_parameters")?;
    let maximum_rows = builder.i64(1)?;
    let rows = builder.capability_call(
        "$database",
        builder.standard.operation("Database", "query")?,
        vec![statement, parameters, columns, maximum_rows],
    )?;
    let query_body =
        builder.let_one("rows", rows, Some("@sql_rows"), |builder, rows_binding| {
            let rows = builder.local(rows_binding)?;
            let length = builder.external_call("sql-rows-length", vec![rows])?;
            let zero = builder.i64(0)?;
            let has_row = builder.external_call("less", vec![zero, length])?;
            let rows = builder.local(rows_binding)?;
            let zero = builder.i64(0)?;
            let row = builder.external_call("sql-rows-get", vec![rows, zero])?;
            let zero = builder.i64(0)?;
            let value = builder.external_call("sql-row-get", vec![row, zero])?;
            let fallback = builder.text("")?;
            let text = builder.call("$sql_text_or", &[], vec![value, fallback])?;
            let found = builder.variant("$maybe_text_found", Some(text))?;
            let missing = builder.variant("$maybe_text_missing", None)?;
            builder.if_expression(has_row, found, missing)
        })?;
    builder.create_function(
        "$query_one_text",
        "query-one-text",
        "@maybe_text",
        &["$database"],
        query_body,
        &[
            ("$query_one_statement", "statement", "static-text"),
            ("$query_one_parameters", "parameters", "@sql_values"),
        ],
    )?;

    let migration_id = builder.i64(MIGRATION_ID)?;
    let migration_checksum = blake3::hash(MIGRATION_STATEMENT.as_bytes())
        .to_hex()
        .to_string();
    let checksum = builder.static_text(&migration_checksum)?;
    let statement = builder.static_text(MIGRATION_STATEMENT)?;
    let migrate = builder.capability_call(
        "$database",
        builder.standard.operation("Database", "migration")?,
        vec![migration_id, checksum, statement],
    )?;
    builder.create_function("$migrate", "migrate", "bool", &["$database"], migrate, &[])?;

    let statement = builder.static_text(LIST_STATEMENT)?;
    let parameters = builder.list("@sql_value", vec![])?;
    let selected = builder.call("$query_one_text", &[], vec![statement, parameters])?;
    let missing = builder.variant("$posts_result_corrupt", None)?;
    let found_binding = builder.binding_label();
    let found_value = builder.local(&found_binding)?;
    let found = builder.call("$decode_posts", &[], vec![found_value])?;
    let list_posts = builder.match_expression(
        selected,
        vec![
            MatchArm::plain("$maybe_text_missing", missing),
            MatchArm::payload("$maybe_text_found", found_binding, "json", "text", found),
        ],
    )?;
    builder.create_function(
        "$list_posts",
        "list-posts",
        "@posts_result",
        &["$database"],
        list_posts,
        &[],
    )?;

    let identifier = builder.capability_call(
        "$identifiers",
        builder.standard.operation("Identifier", "uuid-v4")?,
        vec![],
    )?;
    let create = builder.let_one("id", identifier, Some("text"), |builder, id_binding| {
        let now = builder.capability_call(
            "$clock",
            builder
                .standard
                .operation("WallClock", "utc-milliseconds")?,
            vec![],
        )?;
        builder.let_one("now", now, Some("i64"), |builder, now_binding| {
            let id_value = builder.local(id_binding)?;
            let id = sql_text_value(builder, id_value)?;
            let input = builder.local("$create_post_input")?;
            let author = builder.field_nominal(input, "$write_author")?;
            let author = sql_text_value(builder, author)?;
            let input = builder.local("$create_post_input")?;
            let body = builder.field_nominal(input, "$write_body")?;
            let body = sql_text_value(builder, body)?;
            let created_value = builder.local(now_binding)?;
            let created = sql_i64_value(builder, created_value)?;
            let updated_value = builder.local(now_binding)?;
            let updated = sql_i64_value(builder, updated_value)?;
            let parameters =
                builder.list("@sql_value", vec![id, author, body, created, updated])?;
            let statement = builder.static_text(CREATE_STATEMENT)?;
            let selected = builder.call("$query_one_text", &[], vec![statement, parameters])?;
            let missing = builder.variant("$post_result_corrupt", None)?;
            let found_binding = builder.binding_label();
            let found_value = builder.local(&found_binding)?;
            let found = builder.call("$decode_post", &[], vec![found_value])?;
            let result = builder.match_expression(
                selected,
                vec![
                    MatchArm::plain("$maybe_text_missing", missing),
                    MatchArm::payload("$maybe_text_found", found_binding, "json", "text", found),
                ],
            )?;
            builder.transaction("$database", result)
        })
    })?;
    builder.create_function(
        "$create_post",
        "create-post",
        "@post_result",
        &["$database", "$identifiers", "$clock"],
        create,
        &[("$create_post_input", "input", "@write_post")],
    )?;

    let now = builder.capability_call(
        "$clock",
        builder
            .standard
            .operation("WallClock", "utc-milliseconds")?,
        vec![],
    )?;
    let update = builder.let_one("now", now, Some("i64"), |builder, now_binding| {
        let id_value = builder.local("$update_post_id")?;
        let id = sql_text_value(builder, id_value)?;
        let input = builder.local("$update_post_input")?;
        let author = builder.field_nominal(input, "$write_author")?;
        let author = sql_text_value(builder, author)?;
        let input = builder.local("$update_post_input")?;
        let body = builder.field_nominal(input, "$write_body")?;
        let body = sql_text_value(builder, body)?;
        let updated_value = builder.local(now_binding)?;
        let updated = sql_i64_value(builder, updated_value)?;
        let parameters = builder.list("@sql_value", vec![id, author, body, updated])?;
        let statement = builder.static_text(UPDATE_STATEMENT)?;
        let selected = builder.call("$query_one_text", &[], vec![statement, parameters])?;
        let missing = builder.variant("$post_result_missing", None)?;
        let found_binding = builder.binding_label();
        let found_value = builder.local(&found_binding)?;
        let found = builder.call("$decode_post", &[], vec![found_value])?;
        let result = builder.match_expression(
            selected,
            vec![
                MatchArm::plain("$maybe_text_missing", missing),
                MatchArm::payload("$maybe_text_found", found_binding, "json", "text", found),
            ],
        )?;
        builder.transaction("$database", result)
    })?;
    builder.create_function(
        "$update_post",
        "update-post",
        "@post_result",
        &["$database", "$clock"],
        update,
        &[
            ("$update_post_id", "id", "text"),
            ("$update_post_input", "input", "@write_post"),
        ],
    )?;

    let id_value = builder.local("$delete_post_id")?;
    let id = sql_text_value(builder, id_value)?;
    let parameters = builder.list("@sql_value", vec![id])?;
    let statement = builder.static_text(DELETE_STATEMENT)?;
    let selected = builder.call("$query_one_text", &[], vec![statement, parameters])?;
    let missing = builder.boolean(false)?;
    let found_binding = builder.binding_label();
    let found = builder.boolean(true)?;
    let result = builder.match_expression(
        selected,
        vec![
            MatchArm::plain("$maybe_text_missing", missing),
            MatchArm::payload(
                "$maybe_text_found",
                found_binding,
                "deleted-id",
                "text",
                found,
            ),
        ],
    )?;
    let delete = builder.transaction("$database", result)?;
    builder.create_function(
        "$delete_post",
        "delete-post",
        "bool",
        &["$database"],
        delete,
        &[("$delete_post_id", "id", "text")],
    )?;
    Ok(())
}

fn sql_text_value(builder: &mut Builder<'_>, value: String) -> Result<String, DevError> {
    let case = builder.standard.case("SqlValue", "Text")?.to_owned();
    builder.variant(&case, Some(value))
}

fn sql_i64_value(builder: &mut Builder<'_>, value: String) -> Result<String, DevError> {
    let case = builder.standard.case("SqlValue", "I64")?.to_owned();
    builder.variant(&case, Some(value))
}

fn add_handler(builder: &mut Builder<'_>) -> Result<(), DevError> {
    add_route_handlers(builder)?;

    let request = builder.local(&builder.project.request_parameter.clone())?;
    let method = builder.field_name(request, "method")?;
    let request = builder.local(&builder.project.request_parameter.clone())?;
    let path = builder.field_name(request, "path")?;
    let route = builder.call("$select_route", &[], vec![method, path])?;

    let homepage = text_response(
        builder,
        200,
        "text/html; charset=utf-8",
        "<!doctype html><html><body><h1>lkjscript BBS</h1></body></html>",
    )?;
    let list = builder.call("$handle_list", &[], vec![])?;
    let request = builder.local(&builder.project.request_parameter.clone())?;
    let headers = builder.field_name(request, "headers")?;
    let request = builder.local(&builder.project.request_parameter.clone())?;
    let body_stream = builder.field_name(request, "body")?;
    let create = builder.call("$handle_create", &[], vec![headers, body_stream])?;
    let id = request_query_identity(builder)?;
    let request = builder.local(&builder.project.request_parameter.clone())?;
    let headers = builder.field_name(request, "headers")?;
    let request = builder.local(&builder.project.request_parameter.clone())?;
    let body_stream = builder.field_name(request, "body")?;
    let update = builder.call("$handle_update", &[], vec![id, headers, body_stream])?;
    let id = request_query_identity(builder)?;
    let delete = builder.call("$handle_delete", &[], vec![id])?;
    let method_not_allowed = error_response(builder, 405, "method_not_allowed")?;
    let not_found = error_response(builder, 404, "not_found")?;
    let routed = builder.match_expression(
        route,
        vec![
            MatchArm::plain("$route_home", homepage),
            MatchArm::plain("$route_list", list),
            MatchArm::plain("$route_create", create),
            MatchArm::plain("$route_update", update),
            MatchArm::plain("$route_delete", delete),
            MatchArm::plain("$route_method_not_allowed", method_not_allowed),
            MatchArm::plain("$route_missing", not_found),
        ],
    )?;
    let migration = builder.call("$migrate", &[], vec![])?;
    let body = builder.sequence(vec![migration, routed])?;

    let streams = format!(
        "{}/{}",
        builder.project.package, builder.project.streams_requirement
    );
    builder.record(
        "set.function-contract",
        vec![
            ("as", "%handler-contract".to_owned()),
            ("function", builder.project.handler.clone()),
            ("result", "@response".to_owned()),
            ("effect", "task".to_owned()),
        ],
    )?;
    for (index, requirement) in [streams.as_str(), "$database", "$identifiers", "$clock"]
        .into_iter()
        .enumerate()
    {
        builder.record(
            "effect.requirement",
            vec![
                ("parent", "%handler-contract".to_owned()),
                ("index", index.to_string()),
                ("requirement", requirement.to_owned()),
            ],
        )?;
    }
    builder.record(
        "replace.body",
        vec![
            ("function", builder.project.handler.clone()),
            ("body", body),
        ],
    )?;
    Ok(())
}

fn request_query_identity(builder: &mut Builder<'_>) -> Result<String, DevError> {
    let request = builder.local(&builder.project.request_parameter.clone())?;
    let query = builder.field_name(request, "query_parameters")?;
    let key = builder.text("id")?;
    let fallback = builder.list("text", vec![])?;
    let values = builder.external_call("query-get-or", vec![query, key, fallback])?;
    builder.let_one(
        "query-id-values",
        values,
        None,
        |builder, values_binding| {
            let values = builder.local(values_binding)?;
            let length = builder.generic_external_call("list-length", "text", vec![values])?;
            let one = builder.i64(1)?;
            let exactly_one = builder.external_call("i64-equal", vec![length, one])?;
            let values = builder.local(values_binding)?;
            let zero = builder.i64(0)?;
            let selected = builder.generic_external_call("list-get", "text", vec![values, zero])?;
            let missing = builder.text("")?;
            builder.if_expression(exactly_one, selected, missing)
        },
    )
}

fn add_route_handlers(builder: &mut Builder<'_>) -> Result<(), DevError> {
    let listed = builder.call("$list_posts", &[], vec![])?;
    let list_corrupt = error_response(builder, 500, "persistence_shape")?;
    let list_binding = builder.binding_label();
    let list_value = builder.local(&list_binding)?;
    let list_json = builder.generic_external_call("json-encode", "@posts", vec![list_value])?;
    let list_ok = json_response(builder, 200, list_json)?;
    let list_response = builder.match_expression(
        listed,
        vec![
            MatchArm::payload(
                "$posts_result_found",
                list_binding,
                "posts",
                "@posts",
                list_ok,
            ),
            MatchArm::plain("$posts_result_corrupt", list_corrupt),
        ],
    )?;
    builder.create_function(
        "$handle_list",
        "handle-list-posts",
        "@response",
        &["$database"],
        list_response,
        &[],
    )?;

    let create_admitted = content_type_admitted(builder, "$create_headers")?;
    let create_body = decode_write_body(builder, "$create_body", |builder, write| {
        let created = builder.call("$create_post", &[], vec![write])?;
        post_result_response(builder, created, 201, 500)
    })?;
    let create_bad_type = error_response(builder, 400, "content_type")?;
    let create_response = builder.if_expression(create_admitted, create_body, create_bad_type)?;
    let streams = format!(
        "{}/{}",
        builder.project.package, builder.project.streams_requirement
    );
    builder.create_function(
        "$handle_create",
        "handle-create-post",
        "@response",
        &[streams.as_str(), "$database", "$identifiers", "$clock"],
        create_response,
        &[
            ("$create_headers", "headers", "@headers"),
            ("$create_body", "body", "@body_stream"),
        ],
    )?;

    let id = builder.local("$update_id")?;
    let valid_id = builder.call("$valid_id", &[], vec![id])?;
    let content_type = content_type_admitted(builder, "$update_headers")?;
    let admitted = builder.external_call("bool-and", vec![valid_id, content_type])?;
    let update_body = decode_write_body(builder, "$update_body", |builder, write| {
        let id = builder.local("$update_id")?;
        let updated = builder.call("$update_post", &[], vec![id, write])?;
        post_result_response(builder, updated, 200, 404)
    })?;
    let update_bad = error_response(builder, 400, "invalid_request")?;
    let update_response = builder.if_expression(admitted, update_body, update_bad)?;
    builder.create_function(
        "$handle_update",
        "handle-update-post",
        "@response",
        &[streams.as_str(), "$database", "$clock"],
        update_response,
        &[
            ("$update_id", "id", "text"),
            ("$update_headers", "headers", "@headers"),
            ("$update_body", "body", "@body_stream"),
        ],
    )?;

    let id = builder.local("$delete_id")?;
    let valid_id = builder.call("$valid_id", &[], vec![id])?;
    let id = builder.local("$delete_id")?;
    let deleted = builder.call("$delete_post", &[], vec![id])?;
    let no_content = empty_response(builder, 204)?;
    let missing = error_response(builder, 404, "not_found")?;
    let delete_result = builder.if_expression(deleted, no_content, missing)?;
    let invalid = error_response(builder, 400, "invalid_id")?;
    let delete_response = builder.if_expression(valid_id, delete_result, invalid)?;
    builder.create_function(
        "$handle_delete",
        "handle-delete-post",
        "@response",
        &["$database"],
        delete_response,
        &[("$delete_id", "id", "text")],
    )?;
    Ok(())
}

fn content_type_admitted(builder: &mut Builder<'_>, parameter: &str) -> Result<String, DevError> {
    let headers = builder.local(parameter)?;
    let headers_for_length = builder.local(parameter)?;
    let length =
        builder.generic_external_call("list-length", "@header", vec![headers_for_length])?;
    let zero = builder.i64(0)?;
    builder.call("$has_json_content_type", &[], vec![headers, zero, length])
}

fn decode_write_body(
    builder: &mut Builder<'_>,
    body_parameter: &str,
    valid: impl FnOnce(&mut Builder<'_>, String) -> Result<String, DevError>,
) -> Result<String, DevError> {
    let stream = builder.local(body_parameter)?;
    let maximum = builder.i64(MAXIMUM_REQUEST_JSON_BYTES)?;
    let bytes = builder.capability_call(
        &format!(
            "{}/{}",
            builder.project.package, builder.project.streams_requirement
        ),
        builder.standard.operation("ByteStream", "read-all")?,
        vec![stream, maximum],
    )?;
    let empty_author = builder.text("")?;
    let empty_body = builder.text("")?;
    let fallback = builder.nominal_record(
        "$write_post",
        vec![("$write_author", empty_author), ("$write_body", empty_body)],
    )?;
    let decoded =
        builder.generic_external_call("json-decode-or", "@write_post", vec![bytes, fallback])?;
    builder.let_one("decoded", decoded, None, |builder, binding| {
        let source = builder.local(binding)?;
        let decoder_valid = builder.field_name(source, "valid")?;
        let source = builder.local(binding)?;
        let value_for_validation = builder.field_name(source, "value")?;
        let domain_valid = builder.call("$valid_write", &[], vec![value_for_validation])?;
        let accepted = builder.external_call("bool-and", vec![decoder_valid, domain_valid])?;
        let source = builder.local(binding)?;
        let value = builder.field_name(source, "value")?;
        let success = valid(builder, value)?;
        let bad = error_response(builder, 400, "invalid_json")?;
        builder.if_expression(accepted, success, bad)
    })
}

fn post_result_response(
    builder: &mut Builder<'_>,
    result: String,
    found_status: i64,
    missing_status: i64,
) -> Result<String, DevError> {
    let found_binding = builder.binding_label();
    let post = builder.local(&found_binding)?;
    let encoded = builder.generic_external_call("json-encode", "@post", vec![post])?;
    let found = json_response(builder, found_status, encoded)?;
    let missing = error_response(builder, missing_status, "not_found")?;
    let corrupt = error_response(builder, 500, "persistence_shape")?;
    builder.match_expression(
        result,
        vec![
            MatchArm::plain("$post_result_missing", missing),
            MatchArm::payload("$post_result_found", found_binding, "post", "@post", found),
            MatchArm::plain("$post_result_corrupt", corrupt),
        ],
    )
}

fn text_response(
    builder: &mut Builder<'_>,
    status: i64,
    content_type: &str,
    body: &str,
) -> Result<String, DevError> {
    let status = builder.i64(status)?;
    let content_type = builder.text(content_type)?;
    let body = builder.text(body)?;
    builder.call("$text_response", &[], vec![status, content_type, body])
}

fn json_response(builder: &mut Builder<'_>, status: i64, body: String) -> Result<String, DevError> {
    let status = builder.i64(status)?;
    let content_type = builder.text("application/json")?;
    builder.call("$make_response", &[], vec![status, content_type, body])
}

fn error_response(builder: &mut Builder<'_>, status: i64, code: &str) -> Result<String, DevError> {
    text_response(
        builder,
        status,
        "application/json",
        &format!("{{\"error\":\"{code}\"}}"),
    )
}

fn empty_response(builder: &mut Builder<'_>, status: i64) -> Result<String, DevError> {
    let status = builder.i64(status)?;
    builder.call("$empty_response", &[], vec![status])
}

fn add_graph_tests(builder: &mut Builder<'_>) -> Result<(), DevError> {
    let empty_json = builder.text("[]")?;
    let empty_json = builder.external_call("bytes-from-text", vec![empty_json])?;
    let empty_posts = builder.list("@post", vec![])?;
    let decoded =
        builder.generic_external_call("json-decode-or", "@posts", vec![empty_json, empty_posts])?;
    let actual = builder.field_name(decoded, "valid")?;
    let expected = builder.boolean(true)?;
    add_test(
        builder,
        "$test_empty_post_list_json",
        "empty-post-list-json",
        actual,
        expected,
    )?;

    let method = builder.text("GET")?;
    let path = builder.text("/")?;
    let selected = builder.call("$select_route", &[], vec![method, path])?;
    let actual = route_variant_code(builder, selected)?;
    let expected = builder.i64(1)?;
    add_test(
        builder,
        "$test_select_route_root",
        "select-route-root",
        actual,
        expected,
    )?;

    let method = builder.text("GET")?;
    let path = builder.text("/")?;
    let actual = builder.call("$route_code", &[], vec![method, path])?;
    let expected = builder.i64(1)?;
    add_test(builder, "$test_route_root", "route-root", actual, expected)?;

    let valid_write = write_post_value(builder, "agent", "first post")?;
    let actual = builder.call("$valid_write", &[], vec![valid_write])?;
    let expected = builder.boolean(true)?;
    add_test(
        builder,
        "$test_valid_write",
        "test-valid-write-post",
        actual,
        expected,
    )?;

    let invalid_write = write_post_value(builder, "", "body")?;
    let actual = builder.call("$valid_write", &[], vec![invalid_write])?;
    let expected = builder.boolean(false)?;
    add_test(
        builder,
        "$test_invalid_write",
        "invalid-write-post",
        actual,
        expected,
    )?;

    let method = builder.text("GET")?;
    let path = builder.text("/api/posts")?;
    let actual = builder.call("$route_code", &[], vec![method, path])?;
    let expected = builder.i64(2)?;
    add_test(
        builder,
        "$test_route_list",
        "route-list-posts",
        actual,
        expected,
    )?;

    let method = builder.text("PATCH")?;
    let path = builder.text("/api/posts")?;
    let actual = builder.call("$route_code", &[], vec![method, path])?;
    let expected = builder.i64(6)?;
    add_test(
        builder,
        "$test_route_method",
        "route-method-not-allowed",
        actual,
        expected,
    )?;

    let duplicate = builder.text("{\"author\":\"one\",\"author\":\"two\",\"body\":\"body\"}")?;
    let duplicate = builder.external_call("bytes-from-text", vec![duplicate])?;
    let fallback = write_post_value(builder, "", "")?;
    let decoded = builder.generic_external_call(
        "json-decode-or",
        "@write_post",
        vec![duplicate, fallback],
    )?;
    let actual = builder.field_name(decoded, "valid")?;
    let expected = builder.boolean(false)?;
    add_test(
        builder,
        "$test_duplicate_json",
        "duplicate-json-rejects",
        actual,
        expected,
    )?;

    let trailing = builder.text("{\"author\":\"one\",\"body\":\"body\"} trailing")?;
    let trailing = builder.external_call("bytes-from-text", vec![trailing])?;
    let fallback = write_post_value(builder, "", "")?;
    let decoded =
        builder.generic_external_call("json-decode-or", "@write_post", vec![trailing, fallback])?;
    let actual = builder.field_name(decoded, "valid")?;
    let expected = builder.boolean(false)?;
    add_test(
        builder,
        "$test_trailing_json",
        "trailing-json-rejects",
        actual,
        expected,
    )?;

    let value = builder.text("row-value")?;
    let sql_value = sql_text_value(builder, value)?;
    let fallback = builder.text("fallback")?;
    let actual = builder.call("$sql_text_or", &[], vec![sql_value, fallback])?;
    let expected = builder.text("row-value")?;
    add_test(
        builder,
        "$test_sql_text",
        "sql-text-row-conversion",
        actual,
        expected,
    )?;

    let response = text_response(builder, 201, "text/plain", "created")?;
    let actual = builder.field_name(response, "status")?;
    let expected = builder.i64(201)?;
    add_test(
        builder,
        "$test_response_status",
        "response-status-is-graph-owned",
        actual,
        expected,
    )?;
    Ok(())
}

fn route_variant_code(builder: &mut Builder<'_>, route: String) -> Result<String, DevError> {
    let home = builder.i64(1)?;
    let list = builder.i64(2)?;
    let create = builder.i64(3)?;
    let update = builder.i64(4)?;
    let delete = builder.i64(5)?;
    let method = builder.i64(6)?;
    let missing = builder.i64(7)?;
    builder.match_expression(
        route,
        vec![
            MatchArm::plain("$route_home", home),
            MatchArm::plain("$route_list", list),
            MatchArm::plain("$route_create", create),
            MatchArm::plain("$route_update", update),
            MatchArm::plain("$route_delete", delete),
            MatchArm::plain("$route_method_not_allowed", method),
            MatchArm::plain("$route_missing", missing),
        ],
    )
}

fn write_post_value(
    builder: &mut Builder<'_>,
    author: &str,
    body: &str,
) -> Result<String, DevError> {
    let author = builder.text(author)?;
    let body = builder.text(body)?;
    builder.nominal_record(
        "$write_post",
        vec![("$write_author", author), ("$write_body", body)],
    )
}

fn add_test(
    builder: &mut Builder<'_>,
    symbol: &str,
    name: &str,
    actual: String,
    expected: String,
) -> Result<(), DevError> {
    builder.record(
        "create.test",
        vec![
            ("as", symbol.to_owned()),
            ("module", "$bbs".to_owned()),
            ("name", name.to_owned()),
            ("visibility", "private".to_owned()),
            ("actual", actual),
            ("expected", expected),
        ],
    )
}
