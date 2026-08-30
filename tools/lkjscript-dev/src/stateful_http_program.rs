//! Deterministic compact records for the copied-binary stateful HTTP acceptance application.
//!
//! This module constructs only public compact records from identities obtained through public
//! discovery. It does not open a repository or construct semantic owners through Rust APIs.

use crate::error::DevError;
use lkjscript::platform::control::render_record;
use std::collections::BTreeMap;

pub(crate) const MAXIMUM_REQUEST_JSON_BYTES: i64 = 65_536;
const POSTS_SPACE: &str = "posts";
const POST_INDEX_SPACE: &str = "post-index";
const BBS_SCHEMA_IDENTITY: &str = "bbs-post-v1";
const BBS_SCHEMA_DIGEST: &str = "c7f973a247bcdcc9e942a3d71bf15732145f3929c16f6a462e84a5437e149693";

#[derive(Clone, Debug)]
pub(crate) struct StandardReferences {
    pub(crate) declarations: BTreeMap<String, String>,
    pub(crate) interfaces: BTreeMap<String, String>,
    pub(crate) operations: BTreeMap<String, String>,
    pub(crate) cases: BTreeMap<String, String>,
    pub(crate) fields: BTreeMap<String, String>,
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

    fn field(&self, record: &str, name: &str) -> Result<&str, DevError> {
        let key = format!("{record}.{name}");
        self.fields
            .get(&key)
            .map(String::as_str)
            .ok_or_else(|| DevError::corrupt(format!("standard discovery omitted field '{key}'")))
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

    fn standard_field(
        &mut self,
        value: String,
        record: &str,
        name: &str,
    ) -> Result<String, DevError> {
        let field = self.standard.field(record, name)?.to_owned();
        self.field_nominal(value, &field)
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

    fn function_value(
        &mut self,
        function: &str,
        type_arguments: &[&str],
    ) -> Result<String, DevError> {
        let label = self.expression("function-value", vec![("function", function.to_owned())])?;
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

    fn finish(self) -> ProgramRequest {
        let records = self.records.len();
        ProgramRequest {
            bytes: self.records.concat().into_bytes(),
            records,
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
    add_data_helpers(&mut builder)?;
    add_persistence_functions(&mut builder)?;
    add_handler(&mut builder)?;
    add_graph_tests(&mut builder)?;
    Ok(builder.finish())
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
                ("$posts_result_found", "Found", Some("bytes")),
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
    builder.type_named(
        "@data_key_part",
        builder.standard.declaration("DataKeyPart")?,
    )?;
    builder.type_named(
        "@data_expectation",
        builder.standard.declaration("DataExpectation")?,
    )?;
    builder.type_named(
        "@data_schema_expectation",
        builder.standard.declaration("DataSchemaExpectation")?,
    )?;
    builder.type_named(
        "@data_direction",
        builder.standard.declaration("DataScanDirection")?,
    )?;
    builder.type_named("@data_schema", builder.standard.declaration("DataSchema")?)?;
    builder.type_named("@data_entry", builder.standard.declaration("DataEntry")?)?;
    builder.type_named(
        "@data_scan_item",
        builder.standard.declaration("DataScanItem")?,
    )?;
    builder.type_named(
        "@data_scan_page",
        builder.standard.declaration("DataScanPage")?,
    )?;
    builder.type_list("@data_key", "@data_key_part")?;
    builder.type_list("@data_schemas", "@data_schema")?;
    builder.type_list("@data_entries", "@data_entry")?;
    builder.type_list("@data_scan_items", "@data_scan_item")?;
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
            "$data",
            "data",
            "DataStore",
            vec![
                "schema-read",
                "schema-set",
                "get",
                "scan",
                "put",
                "delete",
                "transaction",
            ],
            vec![
                ("maximum_calls", 256_u64, "calls"),
                ("maximum_input_bytes", 4_194_304, "bytes"),
                ("maximum_output_bytes", 4_194_304, "bytes"),
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

    let previous = builder.local("$header_match_state")?;
    let header = builder.local("$header_match_header")?;
    let name = builder.field_name(header, "name")?;
    let expected_name = builder.text("content-type")?;
    let name_matches = builder.external_call("text-equal", vec![name, expected_name])?;
    let header = builder.local("$header_match_header")?;
    let value = builder.field_name(header, "value")?;
    let expected_text = builder.text("application/json")?;
    let expected = builder.external_call("bytes-from-text", vec![expected_text])?;
    let value_matches = builder.external_call("bytes-equal", vec![value, expected])?;
    let matches = builder.external_call("bool-and", vec![name_matches, value_matches])?;
    let accumulated = builder.external_call("bool-or", vec![previous, matches])?;
    builder.create_function(
        "$header_matches_step",
        "header-matches-json-content-type",
        "bool",
        &[],
        accumulated,
        &[
            ("$header_match_state", "matched", "bool"),
            ("$header_match_header", "header", "@header"),
        ],
    )?;

    let headers = builder.local("$header_values")?;
    let initial = builder.boolean(false)?;
    let step = builder.function_value("$header_matches_step", &[])?;
    let fold = builder.standard.declaration("list-fold-left")?.to_owned();
    let header_result = builder.call(&fold, &["@header", "bool"], vec![headers, initial, step])?;
    builder.create_function(
        "$has_json_content_type",
        "has-json-content-type",
        "bool",
        &[],
        header_result,
        &[("$header_values", "headers", "@headers")],
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

fn add_data_helpers(builder: &mut Builder<'_>) -> Result<(), DevError> {
    let id = builder.local("$post_key_id")?;
    let id = data_key_text(builder, id)?;
    let key = builder.list("@data_key_part", vec![id])?;
    builder.create_function(
        "$post_key",
        "post-key",
        "@data_key",
        &[],
        key,
        &[("$post_key_id", "id", "text")],
    )?;

    let created = builder.local("$index_key_created")?;
    let created = data_key_i64(builder, created)?;
    let id = builder.local("$index_key_id")?;
    let id = data_key_text(builder, id)?;
    let key = builder.list("@data_key_part", vec![created, id])?;
    builder.create_function(
        "$index_key",
        "post-index-key",
        "@data_key",
        &[],
        key,
        &[
            ("$index_key_created", "created", "i64"),
            ("$index_key_id", "id", "text"),
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
    let bytes = builder.local("$decode_post_bytes")?;
    let decoded =
        builder.generic_external_call("data-decode-or", "@post", vec![bytes, fallback_post])?;
    let decoded_post = builder.let_one(
        "decoded-post",
        decoded,
        Some("@post"),
        |builder, binding| {
            let source = builder.local(binding)?;
            let id = builder.field_nominal(source, "$post_id")?;
            let expected = builder.local("$decode_post_expected_id")?;
            let valid = builder.external_call("text-equal", vec![id, expected])?;
            let value = builder.local(binding)?;
            let found = builder.variant("$post_result_found", Some(value))?;
            let corrupt = builder.variant("$post_result_corrupt", None)?;
            builder.if_expression(valid, found, corrupt)
        },
    )?;
    builder.create_function(
        "$decode_post",
        "decode-post",
        "@post_result",
        &[],
        decoded_post,
        &[
            ("$decode_post_bytes", "value", "bytes"),
            ("$decode_post_expected_id", "expected-id", "text"),
        ],
    )?;

    let revision = builder.local("$exact_revision")?;
    let exact = data_expectation_exact(builder, revision)?;
    builder.create_function(
        "$exact_expectation",
        "exact-entry-expectation",
        "@data_expectation",
        &[],
        exact,
        &[("$exact_revision", "revision", "bytes")],
    )?;
    Ok(())
}

fn add_schema_migration(builder: &mut Builder<'_>) -> Result<(), DevError> {
    let posts_space = builder.static_text(POSTS_SPACE)?;
    let posts = builder.capability_call(
        "$data",
        builder.standard.operation("DataStore", "schema-read")?,
        vec![posts_space],
    )?;
    let index_space = builder.static_text(POST_INDEX_SPACE)?;
    let index = builder.capability_call(
        "$data",
        builder.standard.operation("DataStore", "schema-read")?,
        vec![index_space],
    )?;
    let body = builder.let_one(
        "post-schemas",
        posts,
        Some("@data_schemas"),
        |builder, posts_binding| {
            builder.let_one(
                "index-schemas",
                index,
                Some("@data_schemas"),
                |builder, index_binding| {
                    let posts = builder.local(posts_binding)?;
                    let posts_length = builder.generic_external_call(
                        "list-length",
                        "@data_schema",
                        vec![posts],
                    )?;
                    let zero = builder.i64(0)?;
                    let posts_missing =
                        builder.external_call("i64-equal", vec![posts_length, zero])?;
                    let index = builder.local(index_binding)?;
                    let index_length = builder.generic_external_call(
                        "list-length",
                        "@data_schema",
                        vec![index],
                    )?;
                    let zero = builder.i64(0)?;
                    let index_missing =
                        builder.external_call("i64-equal", vec![index_length, zero])?;
                    let both_missing =
                        builder.external_call("bool-and", vec![posts_missing, index_missing])?;

                    let posts_schema = data_schema(builder)?;
                    let posts_expected = data_schema_expectation_missing(builder)?;
                    let posts_space = builder.static_text(POSTS_SPACE)?;
                    let posts_set = builder.capability_call(
                        "$data",
                        builder.standard.operation("DataStore", "schema-set")?,
                        vec![posts_space, posts_expected, posts_schema],
                    )?;
                    let index_schema = data_schema(builder)?;
                    let index_expected = data_schema_expectation_missing(builder)?;
                    let index_space = builder.static_text(POST_INDEX_SPACE)?;
                    let index_set = builder.capability_call(
                        "$data",
                        builder.standard.operation("DataStore", "schema-set")?,
                        vec![index_space, index_expected, index_schema],
                    )?;
                    let initialized =
                        builder.external_call("bool-and", vec![posts_set, index_set])?;

                    let posts = builder.local(posts_binding)?;
                    let posts_length = builder.generic_external_call(
                        "list-length",
                        "@data_schema",
                        vec![posts],
                    )?;
                    let one = builder.i64(1)?;
                    let posts_one = builder.external_call("i64-equal", vec![posts_length, one])?;
                    let index = builder.local(index_binding)?;
                    let index_length = builder.generic_external_call(
                        "list-length",
                        "@data_schema",
                        vec![index],
                    )?;
                    let one = builder.i64(1)?;
                    let index_one = builder.external_call("i64-equal", vec![index_length, one])?;
                    let both_one = builder.external_call("bool-and", vec![posts_one, index_one])?;

                    let posts = builder.local(posts_binding)?;
                    let zero = builder.i64(0)?;
                    let posts_schema = builder.generic_external_call(
                        "list-get",
                        "@data_schema",
                        vec![posts, zero],
                    )?;
                    let posts_identity =
                        builder.standard_field(posts_schema, "DataSchema", "identity")?;
                    let identity = builder.text(BBS_SCHEMA_IDENTITY)?;
                    let posts_identity_ok =
                        builder.external_call("text-equal", vec![posts_identity, identity])?;
                    let posts = builder.local(posts_binding)?;
                    let zero = builder.i64(0)?;
                    let posts_schema = builder.generic_external_call(
                        "list-get",
                        "@data_schema",
                        vec![posts, zero],
                    )?;
                    let posts_digest =
                        builder.standard_field(posts_schema, "DataSchema", "digest")?;
                    let digest = builder.text(BBS_SCHEMA_DIGEST)?;
                    let digest = builder.external_call("bytes-from-text", vec![digest])?;
                    let posts_digest_ok =
                        builder.external_call("bytes-equal", vec![posts_digest, digest])?;
                    let posts_valid = builder
                        .external_call("bool-and", vec![posts_identity_ok, posts_digest_ok])?;

                    let index = builder.local(index_binding)?;
                    let zero = builder.i64(0)?;
                    let index_schema = builder.generic_external_call(
                        "list-get",
                        "@data_schema",
                        vec![index, zero],
                    )?;
                    let index_identity =
                        builder.standard_field(index_schema, "DataSchema", "identity")?;
                    let identity = builder.text(BBS_SCHEMA_IDENTITY)?;
                    let index_identity_ok =
                        builder.external_call("text-equal", vec![index_identity, identity])?;
                    let index = builder.local(index_binding)?;
                    let zero = builder.i64(0)?;
                    let index_schema = builder.generic_external_call(
                        "list-get",
                        "@data_schema",
                        vec![index, zero],
                    )?;
                    let index_digest =
                        builder.standard_field(index_schema, "DataSchema", "digest")?;
                    let digest = builder.text(BBS_SCHEMA_DIGEST)?;
                    let digest = builder.external_call("bytes-from-text", vec![digest])?;
                    let index_digest_ok =
                        builder.external_call("bytes-equal", vec![index_digest, digest])?;
                    let index_valid = builder
                        .external_call("bool-and", vec![index_identity_ok, index_digest_ok])?;
                    let schemas_valid =
                        builder.external_call("bool-and", vec![posts_valid, index_valid])?;
                    let false_value = builder.boolean(false)?;
                    let existing = builder.if_expression(both_one, schemas_valid, false_value)?;
                    builder.if_expression(both_missing, initialized, existing)
                },
            )
        },
    )?;
    let body = builder.transaction("$data", body)?;
    builder.create_function("$migrate", "migrate", "bool", &["$data"], body, &[])
}

fn add_index_projection(builder: &mut Builder<'_>) -> Result<(), DevError> {
    let index = builder.local("$collect_index")?;
    let items = builder.local("$collect_items")?;
    let length = builder.generic_external_call("list-length", "@data_scan_item", vec![items])?;
    let more = builder.external_call("less", vec![index, length])?;

    let items = builder.local("$collect_items")?;
    let index = builder.local("$collect_index")?;
    let item = builder.generic_external_call("list-get", "@data_scan_item", vec![items, index])?;
    let key = builder.standard_field(item, "DataScanItem", "key")?;
    let one = builder.i64(1)?;
    let id_part = builder.generic_external_call("list-get", "@data_key_part", vec![key, one])?;

    let bool_binding = builder.binding_label();
    let bool_corrupt = builder.variant("$posts_result_corrupt", None)?;
    let i64_binding = builder.binding_label();
    let i64_corrupt = builder.variant("$posts_result_corrupt", None)?;
    let text_binding = builder.binding_label();
    let text_body = index_text_projection(builder, &text_binding)?;
    let bytes_binding = builder.binding_label();
    let bytes_corrupt = builder.variant("$posts_result_corrupt", None)?;
    let projected = builder.match_expression(
        id_part,
        vec![
            MatchArm::payload(
                builder.standard.case("DataKeyPart", "Bool")?,
                bool_binding,
                "foreign-bool-key",
                "bool",
                bool_corrupt,
            ),
            MatchArm::payload(
                builder.standard.case("DataKeyPart", "I64")?,
                i64_binding,
                "foreign-i64-key",
                "i64",
                i64_corrupt,
            ),
            MatchArm::payload(
                builder.standard.case("DataKeyPart", "Text")?,
                text_binding,
                "post-id",
                "text",
                text_body,
            ),
            MatchArm::payload(
                builder.standard.case("DataKeyPart", "Bytes")?,
                bytes_binding,
                "foreign-bytes-key",
                "bytes",
                bytes_corrupt,
            ),
        ],
    )?;

    let output = builder.local("$collect_output")?;
    let closing = builder.text("]")?;
    let closing = builder.external_call("bytes-from-text", vec![closing])?;
    let completed = builder.external_call("bytes-concat", vec![output, closing])?;
    let completed = builder.variant("$posts_result_found", Some(completed))?;
    let body = builder.if_expression(more, projected, completed)?;
    builder.create_function(
        "$collect_posts_json",
        "collect-post-index-json",
        "@posts_result",
        &["$data"],
        body,
        &[
            ("$collect_index", "index", "i64"),
            ("$collect_items", "items", "@data_scan_items"),
            ("$collect_output", "output", "bytes"),
        ],
    )
}

fn index_text_projection(builder: &mut Builder<'_>, id_binding: &str) -> Result<String, DevError> {
    let id = builder.local(id_binding)?;
    let key = builder.call("$post_key", &[], vec![id])?;
    let space = builder.static_text(POSTS_SPACE)?;
    let entries = builder.capability_call(
        "$data",
        builder.standard.operation("DataStore", "get")?,
        vec![space, key],
    )?;
    builder.let_one(
        "indexed-post-entry",
        entries,
        Some("@data_entries"),
        |builder, entries_binding| {
            let entries = builder.local(entries_binding)?;
            let length =
                builder.generic_external_call("list-length", "@data_entry", vec![entries])?;
            let one = builder.i64(1)?;
            let present = builder.external_call("i64-equal", vec![length, one])?;

            let entries = builder.local(entries_binding)?;
            let zero = builder.i64(0)?;
            let entry =
                builder.generic_external_call("list-get", "@data_entry", vec![entries, zero])?;
            let value = builder.standard_field(entry, "DataEntry", "value")?;
            let id = builder.local(id_binding)?;
            let decoded = builder.call("$decode_post", &[], vec![value, id])?;

            let post_binding = builder.binding_label();
            let post = builder.local(&post_binding)?;
            let encoded = builder.generic_external_call("json-encode", "@post", vec![post])?;
            let index = builder.local("$collect_index")?;
            let zero = builder.i64(0)?;
            let first = builder.external_call("i64-equal", vec![index, zero])?;
            let empty = empty_bytes(builder)?;
            let comma = builder.text(",")?;
            let comma = builder.external_call("bytes-from-text", vec![comma])?;
            let delimiter = builder.if_expression(first, empty, comma)?;
            let output = builder.local("$collect_output")?;
            let output = builder.external_call("bytes-concat", vec![output, delimiter])?;
            let output = builder.external_call("bytes-concat", vec![output, encoded])?;
            let index = builder.local("$collect_index")?;
            let one = builder.i64(1)?;
            let next = builder.external_call("add", vec![index, one])?;
            let items = builder.local("$collect_items")?;
            let recurse = builder.call("$collect_posts_json", &[], vec![next, items, output])?;
            let missing = builder.variant("$posts_result_corrupt", None)?;
            let corrupt = builder.variant("$posts_result_corrupt", None)?;
            let decoded_result = builder.match_expression(
                decoded,
                vec![
                    MatchArm::plain("$post_result_missing", missing),
                    MatchArm::payload("$post_result_found", post_binding, "post", "@post", recurse),
                    MatchArm::plain("$post_result_corrupt", corrupt),
                ],
            )?;
            let missing = builder.variant("$posts_result_corrupt", None)?;
            builder.if_expression(present, decoded_result, missing)
        },
    )
}

fn add_persistence_functions(builder: &mut Builder<'_>) -> Result<(), DevError> {
    add_schema_migration(builder)?;
    add_index_projection(builder)?;

    let prefix = builder.list("@data_key_part", vec![])?;
    let direction = data_direction(builder, "Forward")?;
    let maximum_items = builder.i64(128)?;
    let maximum_bytes = builder.i64(1_048_576)?;
    let maximum_work = builder.i64(4_096)?;
    let continuation = empty_bytes(builder)?;
    let space = builder.static_text(POST_INDEX_SPACE)?;
    let page = builder.capability_call(
        "$data",
        builder.standard.operation("DataStore", "scan")?,
        vec![
            space,
            prefix,
            direction,
            maximum_items,
            maximum_bytes,
            maximum_work,
            continuation,
        ],
    )?;
    let list_posts = builder.let_one(
        "post-index-page",
        page,
        Some("@data_scan_page"),
        |builder, page_binding| {
            let source = builder.local(page_binding)?;
            let continuation = builder.standard_field(source, "DataScanPage", "continuation")?;
            let continuation_length = builder.external_call("bytes-length", vec![continuation])?;
            let zero = builder.i64(0)?;
            let complete = builder.external_call("i64-equal", vec![continuation_length, zero])?;
            let source = builder.local(page_binding)?;
            let items = builder.standard_field(source, "DataScanPage", "items")?;
            let index = builder.i64(0)?;
            let opening = builder.text("[")?;
            let opening = builder.external_call("bytes-from-text", vec![opening])?;
            let projected =
                builder.call("$collect_posts_json", &[], vec![index, items, opening])?;
            let corrupt = builder.variant("$posts_result_corrupt", None)?;
            builder.if_expression(complete, projected, corrupt)
        },
    )?;
    let list_posts = builder.transaction("$data", list_posts)?;
    builder.create_function(
        "$list_posts",
        "list-posts",
        "@posts_result",
        &["$data"],
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
            let id = builder.local(id_binding)?;
            let input = builder.local("$create_post_input")?;
            let author = builder.field_nominal(input, "$write_author")?;
            let input = builder.local("$create_post_input")?;
            let body = builder.field_nominal(input, "$write_body")?;
            let created = builder.local(now_binding)?;
            let updated = builder.local(now_binding)?;
            let post = builder.nominal_record(
                "$post",
                vec![
                    ("$post_id", id),
                    ("$post_author", author),
                    ("$post_body", body),
                    ("$post_created", created),
                    ("$post_updated", updated),
                ],
            )?;
            builder.let_one("new-post", post, Some("@post"), |builder, post_binding| {
                let id = builder.local(id_binding)?;
                let primary_key = builder.call("$post_key", &[], vec![id])?;
                let value = builder.local(post_binding)?;
                let value = builder.generic_external_call("data-encode", "@post", vec![value])?;
                let expected = data_expectation_missing(builder)?;
                let primary_space = builder.static_text(POSTS_SPACE)?;
                let primary = builder.capability_call(
                    "$data",
                    builder.standard.operation("DataStore", "put")?,
                    vec![primary_space, primary_key, value, expected],
                )?;

                let created = builder.local(now_binding)?;
                let id = builder.local(id_binding)?;
                let index_key = builder.call("$index_key", &[], vec![created, id])?;
                let index_value = empty_bytes(builder)?;
                let expected = data_expectation_missing(builder)?;
                let index_space = builder.static_text(POST_INDEX_SPACE)?;
                let index = builder.capability_call(
                    "$data",
                    builder.standard.operation("DataStore", "put")?,
                    vec![index_space, index_key, index_value, expected],
                )?;
                let post = builder.local(post_binding)?;
                let found = builder.variant("$post_result_found", Some(post))?;
                let corrupt = builder.variant("$post_result_corrupt", None)?;
                let dependent = builder.if_expression(index, found, corrupt)?;
                let corrupt = builder.variant("$post_result_corrupt", None)?;
                let result = builder.if_expression(primary, dependent, corrupt)?;
                builder.transaction("$data", result)
            })
        })
    })?;
    builder.create_function(
        "$create_post",
        "create-post",
        "@post_result",
        &["$data", "$identifiers", "$clock"],
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
        let id = builder.local("$update_post_id")?;
        let key = builder.call("$post_key", &[], vec![id])?;
        let space = builder.static_text(POSTS_SPACE)?;
        let entries = builder.capability_call(
            "$data",
            builder.standard.operation("DataStore", "get")?,
            vec![space, key],
        )?;
        let result = builder.let_one(
            "update-entry-list",
            entries,
            Some("@data_entries"),
            |builder, entries_binding| {
                let entries = builder.local(entries_binding)?;
                let length =
                    builder.generic_external_call("list-length", "@data_entry", vec![entries])?;
                let one = builder.i64(1)?;
                let found_entry = builder.external_call("i64-equal", vec![length, one])?;

                let entries = builder.local(entries_binding)?;
                let zero = builder.i64(0)?;
                let entry = builder.generic_external_call(
                    "list-get",
                    "@data_entry",
                    vec![entries, zero],
                )?;
                let revision = builder.standard_field(entry, "DataEntry", "revision")?;
                let entries = builder.local(entries_binding)?;
                let zero = builder.i64(0)?;
                let entry = builder.generic_external_call(
                    "list-get",
                    "@data_entry",
                    vec![entries, zero],
                )?;
                let value = builder.standard_field(entry, "DataEntry", "value")?;
                let expected_id = builder.local("$update_post_id")?;
                let decoded = builder.call("$decode_post", &[], vec![value, expected_id])?;

                let old_binding = builder.binding_label();
                let old = builder.local(&old_binding)?;
                let id = builder.field_nominal(old, "$post_id")?;
                let input = builder.local("$update_post_input")?;
                let author = builder.field_nominal(input, "$write_author")?;
                let input = builder.local("$update_post_input")?;
                let body = builder.field_nominal(input, "$write_body")?;
                let old = builder.local(&old_binding)?;
                let created = builder.field_nominal(old, "$post_created")?;
                let updated = builder.local(now_binding)?;
                let next = builder.nominal_record(
                    "$post",
                    vec![
                        ("$post_id", id),
                        ("$post_author", author),
                        ("$post_body", body),
                        ("$post_created", created),
                        ("$post_updated", updated),
                    ],
                )?;
                let next_scope = builder.let_one(
                    "updated-post",
                    next,
                    Some("@post"),
                    |builder, next_binding| {
                        let next_value = builder.local(next_binding)?;
                        let encoded = builder.generic_external_call(
                            "data-encode",
                            "@post",
                            vec![next_value],
                        )?;
                        let id = builder.local("$update_post_id")?;
                        let key = builder.call("$post_key", &[], vec![id])?;
                        let expected = builder.call("$exact_expectation", &[], vec![revision])?;
                        let space = builder.static_text(POSTS_SPACE)?;
                        let written = builder.capability_call(
                            "$data",
                            builder.standard.operation("DataStore", "put")?,
                            vec![space, key, encoded, expected],
                        )?;
                        let next_value = builder.local(next_binding)?;
                        let written_value =
                            builder.variant("$post_result_found", Some(next_value))?;
                        let stale = builder.variant("$post_result_missing", None)?;
                        builder.if_expression(written, written_value, stale)
                    },
                )?;
                let missing = builder.variant("$post_result_missing", None)?;
                let corrupt = builder.variant("$post_result_corrupt", None)?;
                let decoded_result = builder.match_expression(
                    decoded,
                    vec![
                        MatchArm::plain("$post_result_missing", missing),
                        MatchArm::payload(
                            "$post_result_found",
                            old_binding,
                            "post",
                            "@post",
                            next_scope,
                        ),
                        MatchArm::plain("$post_result_corrupt", corrupt),
                    ],
                )?;
                let missing = builder.variant("$post_result_missing", None)?;
                builder.if_expression(found_entry, decoded_result, missing)
            },
        )?;
        builder.transaction("$data", result)
    })?;
    builder.create_function(
        "$update_post",
        "update-post",
        "@post_result",
        &["$data", "$clock"],
        update,
        &[
            ("$update_post_id", "id", "text"),
            ("$update_post_input", "input", "@write_post"),
        ],
    )?;

    let id = builder.local("$delete_post_id")?;
    let key = builder.call("$post_key", &[], vec![id])?;
    let space = builder.static_text(POSTS_SPACE)?;
    let entries = builder.capability_call(
        "$data",
        builder.standard.operation("DataStore", "get")?,
        vec![space, key],
    )?;
    let delete = builder.let_one(
        "delete-entry-list",
        entries,
        Some("@data_entries"),
        |builder, entries_binding| {
            let entries = builder.local(entries_binding)?;
            let length =
                builder.generic_external_call("list-length", "@data_entry", vec![entries])?;
            let one = builder.i64(1)?;
            let found_entry = builder.external_call("i64-equal", vec![length, one])?;

            let entries = builder.local(entries_binding)?;
            let zero = builder.i64(0)?;
            let entry =
                builder.generic_external_call("list-get", "@data_entry", vec![entries, zero])?;
            let primary_revision = builder.standard_field(entry, "DataEntry", "revision")?;
            let entries = builder.local(entries_binding)?;
            let zero = builder.i64(0)?;
            let entry =
                builder.generic_external_call("list-get", "@data_entry", vec![entries, zero])?;
            let value = builder.standard_field(entry, "DataEntry", "value")?;
            let expected_id = builder.local("$delete_post_id")?;
            let decoded = builder.call("$decode_post", &[], vec![value, expected_id])?;

            let post_binding = builder.binding_label();
            let post = builder.local(&post_binding)?;
            let created = builder.field_nominal(post, "$post_created")?;
            let id = builder.local("$delete_post_id")?;
            let index_key = builder.call("$index_key", &[], vec![created, id])?;
            let index_space = builder.static_text(POST_INDEX_SPACE)?;
            let index_entries = builder.capability_call(
                "$data",
                builder.standard.operation("DataStore", "get")?,
                vec![index_space, index_key],
            )?;
            let removed = builder.let_one(
                "delete-index-entry-list",
                index_entries,
                Some("@data_entries"),
                |builder, index_entries_binding| {
                    let entries = builder.local(index_entries_binding)?;
                    let length = builder.generic_external_call(
                        "list-length",
                        "@data_entry",
                        vec![entries],
                    )?;
                    let one = builder.i64(1)?;
                    let found_index = builder.external_call("i64-equal", vec![length, one])?;

                    let entries = builder.local(index_entries_binding)?;
                    let zero = builder.i64(0)?;
                    let index_entry = builder.generic_external_call(
                        "list-get",
                        "@data_entry",
                        vec![entries, zero],
                    )?;
                    let index_revision =
                        builder.standard_field(index_entry, "DataEntry", "revision")?;

                    let id = builder.local("$delete_post_id")?;
                    let primary_key = builder.call("$post_key", &[], vec![id])?;
                    let expected =
                        builder.call("$exact_expectation", &[], vec![primary_revision])?;
                    let primary_space = builder.static_text(POSTS_SPACE)?;
                    let primary_removed = builder.capability_call(
                        "$data",
                        builder.standard.operation("DataStore", "delete")?,
                        vec![primary_space, primary_key, expected],
                    )?;

                    let post = builder.local(&post_binding)?;
                    let created = builder.field_nominal(post, "$post_created")?;
                    let id = builder.local("$delete_post_id")?;
                    let index_key = builder.call("$index_key", &[], vec![created, id])?;
                    let expected = builder.call("$exact_expectation", &[], vec![index_revision])?;
                    let index_space = builder.static_text(POST_INDEX_SPACE)?;
                    let index_removed = builder.capability_call(
                        "$data",
                        builder.standard.operation("DataStore", "delete")?,
                        vec![index_space, index_key, expected],
                    )?;
                    let both =
                        builder.external_call("bool-and", vec![primary_removed, index_removed])?;
                    let false_value = builder.boolean(false)?;
                    builder.if_expression(found_index, both, false_value)
                },
            )?;
            let missing_post = builder.boolean(false)?;
            let corrupt_post = builder.boolean(false)?;
            let decoded_result = builder.match_expression(
                decoded,
                vec![
                    MatchArm::plain("$post_result_missing", missing_post),
                    MatchArm::payload("$post_result_found", post_binding, "post", "@post", removed),
                    MatchArm::plain("$post_result_corrupt", corrupt_post),
                ],
            )?;
            let missing = builder.boolean(false)?;
            builder.if_expression(found_entry, decoded_result, missing)
        },
    )?;
    let delete = builder.transaction("$data", delete)?;
    builder.create_function(
        "$delete_post",
        "delete-post",
        "bool",
        &["$data"],
        delete,
        &[("$delete_post_id", "id", "text")],
    )?;
    Ok(())
}

fn data_key_text(builder: &mut Builder<'_>, value: String) -> Result<String, DevError> {
    let case = builder.standard.case("DataKeyPart", "Text")?.to_owned();
    builder.variant(&case, Some(value))
}

fn data_key_i64(builder: &mut Builder<'_>, value: String) -> Result<String, DevError> {
    let case = builder.standard.case("DataKeyPart", "I64")?.to_owned();
    builder.variant(&case, Some(value))
}

fn data_expectation_missing(builder: &mut Builder<'_>) -> Result<String, DevError> {
    let case = builder
        .standard
        .case("DataExpectation", "Missing")?
        .to_owned();
    builder.variant(&case, None)
}

fn data_expectation_exact(builder: &mut Builder<'_>, revision: String) -> Result<String, DevError> {
    let case = builder
        .standard
        .case("DataExpectation", "Exact")?
        .to_owned();
    builder.variant(&case, Some(revision))
}

fn data_schema_expectation_missing(builder: &mut Builder<'_>) -> Result<String, DevError> {
    let case = builder
        .standard
        .case("DataSchemaExpectation", "Missing")?
        .to_owned();
    builder.variant(&case, None)
}

fn data_direction(builder: &mut Builder<'_>, name: &str) -> Result<String, DevError> {
    let case = builder.standard.case("DataScanDirection", name)?.to_owned();
    builder.variant(&case, None)
}

fn data_schema(builder: &mut Builder<'_>) -> Result<String, DevError> {
    let declaration = builder.standard.declaration("DataSchema")?.to_owned();
    let identity_field = builder.standard.field("DataSchema", "identity")?.to_owned();
    let digest_field = builder.standard.field("DataSchema", "digest")?.to_owned();
    let identity = builder.text(BBS_SCHEMA_IDENTITY)?;
    let digest = builder.text(BBS_SCHEMA_DIGEST)?;
    let digest = builder.external_call("bytes-from-text", vec![digest])?;
    builder.nominal_record(
        &declaration,
        vec![(&identity_field, identity), (&digest_field, digest)],
    )
}

fn empty_bytes(builder: &mut Builder<'_>) -> Result<String, DevError> {
    let empty = builder.text("")?;
    builder.external_call("bytes-from-text", vec![empty])
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
    let schema_error = error_response(builder, 500, "schema_mismatch")?;
    let body = builder.if_expression(migration, routed, schema_error)?;

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
    for (index, requirement) in [streams.as_str(), "$data", "$identifiers", "$clock"]
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
    let list_ok = json_response(builder, 200, list_value)?;
    let list_response = builder.match_expression(
        listed,
        vec![
            MatchArm::payload(
                "$posts_result_found",
                list_binding,
                "json",
                "bytes",
                list_ok,
            ),
            MatchArm::plain("$posts_result_corrupt", list_corrupt),
        ],
    )?;
    builder.create_function(
        "$handle_list",
        "handle-list-posts",
        "@response",
        &["$data"],
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
        &[streams.as_str(), "$data", "$identifiers", "$clock"],
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
        &[streams.as_str(), "$data", "$clock"],
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
        &["$data"],
        delete_response,
        &[("$delete_id", "id", "text")],
    )?;
    Ok(())
}

fn content_type_admitted(builder: &mut Builder<'_>, parameter: &str) -> Result<String, DevError> {
    let headers = builder.local(parameter)?;
    builder.call("$has_json_content_type", &[], vec![headers])
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

    let id = builder.text("post-id")?;
    let author = builder.text("agent")?;
    let body = builder.text("body")?;
    let created = builder.i64(1)?;
    let updated = builder.i64(2)?;
    let post = builder.nominal_record(
        "$post",
        vec![
            ("$post_id", id),
            ("$post_author", author),
            ("$post_body", body),
            ("$post_created", created),
            ("$post_updated", updated),
        ],
    )?;
    let encoded = builder.generic_external_call("data-encode", "@post", vec![post])?;
    let expected_id = builder.text("post-id")?;
    let decoded = builder.call("$decode_post", &[], vec![encoded, expected_id])?;
    let missing = builder.boolean(false)?;
    let found_binding = builder.binding_label();
    let found = builder.boolean(true)?;
    let corrupt = builder.boolean(false)?;
    let actual = builder.match_expression(
        decoded,
        vec![
            MatchArm::plain("$post_result_missing", missing),
            MatchArm::payload("$post_result_found", found_binding, "post", "@post", found),
            MatchArm::plain("$post_result_corrupt", corrupt),
        ],
    )?;
    let expected = builder.boolean(true)?;
    add_test(
        builder,
        "$test_data_codec",
        "data-codec-roundtrip",
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
