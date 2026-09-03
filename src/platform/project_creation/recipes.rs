//! Executable-owned recipe intent expressed only through public authored operations.

use super::{ProjectAuxiliary, ProjectRecipe, ProjectTemplate};
use crate::platform::change::{
    AuthoredChange, AuthoredDeclarationReference, AuthoredExpression, AuthoredExpressionOperation,
    AuthoredFieldSelector, AuthoredFunctionEffect, AuthoredLetBinding, AuthoredLocalReference,
    AuthoredOperationReference, AuthoredParameter, AuthoredPort, AuthoredPortImplementation,
    AuthoredPortReference, AuthoredRecordExpressionField, AuthoredRequirement,
    AuthoredRequirementReference, AuthoredResourceLimit, AuthoredStructuralTypeField, AuthoredType,
    AuthoredTypeParameterReference, DeclarationSelector, ModuleSelector,
};
use crate::platform::deployment::{
    encode_deployment, starter_http_deployment, starter_nostr_relay_deployment,
};
use crate::platform::diagnostic::{Diagnostic, DiagnosticClass};
use crate::platform::kernel::{
    DeclarationReference, DeclarationVisibility, Name, OperationReference, TypeForm,
    TypeObjectDigest, TypeObjectInterner,
};
use crate::platform::package::RunnerKind;
use crate::platform::{builtin_standard::BuiltinStandard, http_client::normalize_nostr_relay_url};

const MODULE: &str = "$application_module";
const COMPONENT: &str = "$application_component";
const PORT: &str = "$application_port";
const TARGET: &str = "$application_target";
const STREAMS: &str = "$streams_requirement";
const RELAY: &str = "$relay_requirement";

pub(super) fn minimal_recipe() -> ProjectRecipe {
    ProjectRecipe {
        changes: Vec::new(),
        transports: Vec::new(),
        template: ProjectTemplate::Minimal,
        auxiliary: None,
    }
}

pub(super) fn command_recipe() -> Result<ProjectRecipe, Diagnostic> {
    let standard = BuiltinStandard::load()?;
    let (text_from_static, static_text_type, text_type) = standard.command_text_signature()?;
    let mut interner = TypeObjectInterner::default();
    if interner.intern(TypeForm::StaticText)? != static_text_type
        || interner.intern(TypeForm::Text)? != text_type
    {
        return Err(recipe_error(
            "new_command_standard_types",
            "built-in standard primitive types disagree with canonical Graph 8 types",
        ));
    }

    let mut expressions = RecipeExpressions::default();
    let greeting_literal = expressions.static_text("hello");
    let greeting = expressions.call(
        exact_declaration(text_from_static),
        Vec::new(),
        vec![greeting_literal],
    );
    let test_actual = expressions.call(local_declaration("$greet"), Vec::new(), Vec::new());
    let test_expected = expressions.text("hello");
    Ok(ProjectRecipe {
        changes: vec![
            builtin_dependency(standard),
            module()?,
            AuthoredChange::CreateFunction {
                symbol: "$greet".to_owned(),
                module: local_module(),
                name: name("greet")?,
                visibility: DeclarationVisibility::Private,
                type_parameters: Vec::new(),
                parameters: Vec::new(),
                result: AuthoredType::Text {},
                effect: AuthoredFunctionEffect::Pure {},
                body: greeting,
            },
            component()?,
            AuthoredChange::AddPort {
                component: local_declaration_selector(COMPONENT),
                port: AuthoredPort {
                    symbol: PORT.to_owned(),
                    name: name("main")?,
                    function_type: AuthoredType::Function {
                        parameters: Vec::new(),
                        result: Box::new(AuthoredType::Text {}),
                    },
                    implementation: AuthoredPortImplementation::Function {
                        function: local_declaration("$greet"),
                    },
                },
            },
            target("main", RunnerKind::Command)?,
            AuthoredChange::CreateTest {
                symbol: "$main_returns_hello".to_owned(),
                module: local_module(),
                name: name("main-returns-hello")?,
                visibility: DeclarationVisibility::Private,
                actual: test_actual,
                expected: test_expected,
            },
        ],
        transports: vec![standard.transport()],
        template: ProjectTemplate::Command,
        auxiliary: None,
    })
}

pub(super) fn http_recipe() -> Result<ProjectRecipe, Diagnostic> {
    let standard = BuiltinStandard::load()?;
    let contract = standard.http_recipe_contract()?;
    let mut interner = TypeObjectInterner::default();
    let semantic_http = crate::platform::http::semantic_http_types(&mut interner)?;
    if interner.intern(TypeForm::StaticText)? != contract.static_text_type
        || semantic_http.text_type != contract.text_type
        || semantic_http.bytes_type != contract.bytes_type
    {
        return Err(recipe_error(
            "new_http_standard_types",
            "built-in HTTP recipe declarations disagree with canonical Graph 8 primitive types",
        ));
    }

    let request_type = authored_type(&interner, semantic_http.request_type)?;
    let response_type = authored_type(&interner, semantic_http.response_type)?;
    let header_type = authored_type(&interner, semantic_http.header_type)?;
    let function_type = authored_type(&interner, semantic_http.function_type)?;
    let mut expressions = RecipeExpressions::default();

    let response_text = expressions.static_text("hello from lkjscript");
    let status_code = expressions.i64(200);
    let response_call =
        expressions.call(local_declaration("$response_text"), Vec::new(), Vec::new());
    let text_conversion = expressions.call(
        exact_declaration(contract.text_from_static),
        Vec::new(),
        vec![response_call],
    );
    let body = expressions.call(
        exact_declaration(contract.bytes_from_text),
        Vec::new(),
        vec![text_conversion],
    );
    let headers = expressions.list(header_type, Vec::new());
    let status = expressions.call(local_declaration("$status_code"), Vec::new(), Vec::new());
    let handler_body = expressions.record(vec![
        ("body", body),
        ("headers", headers),
        ("status", status),
    ])?;
    let test_actual = expressions.call(local_declaration("$status_code"), Vec::new(), Vec::new());
    let test_expected = expressions.i64(200);
    let descriptor = encode_deployment(&starter_http_deployment()?)?;

    Ok(ProjectRecipe {
        changes: vec![
            builtin_dependency(standard),
            module()?,
            AuthoredChange::CreateFunction {
                symbol: "$response_text".to_owned(),
                module: local_module(),
                name: name("response-text")?,
                visibility: DeclarationVisibility::Private,
                type_parameters: Vec::new(),
                parameters: Vec::new(),
                result: AuthoredType::StaticText {},
                effect: AuthoredFunctionEffect::Pure {},
                body: response_text,
            },
            AuthoredChange::CreateFunction {
                symbol: "$status_code".to_owned(),
                module: local_module(),
                name: name("status-code")?,
                visibility: DeclarationVisibility::Private,
                type_parameters: Vec::new(),
                parameters: Vec::new(),
                result: AuthoredType::I64 {},
                effect: AuthoredFunctionEffect::Pure {},
                body: status_code,
            },
            AuthoredChange::CreateFunction {
                symbol: "$handle".to_owned(),
                module: local_module(),
                name: name("handle")?,
                visibility: DeclarationVisibility::Private,
                type_parameters: Vec::new(),
                parameters: Vec::new(),
                result: response_type,
                effect: AuthoredFunctionEffect::Task {
                    requirements: vec![local_requirement(STREAMS)],
                },
                body: handler_body,
            },
            parameter("$handle", "$request", "request", request_type)?,
            component()?,
            requirement(
                STREAMS,
                "streams",
                contract.byte_stream_interface,
                &contract.byte_stream_operations,
                10_000,
            )?,
            port("$handle", function_type)?,
            target("serve", RunnerKind::Http)?,
            AuthoredChange::CreateTest {
                symbol: "$status_is_200".to_owned(),
                module: local_module(),
                name: name("status-is-200")?,
                visibility: DeclarationVisibility::Private,
                actual: test_actual,
                expected: test_expected,
            },
        ],
        transports: vec![standard.transport()],
        template: ProjectTemplate::Http,
        auxiliary: Some(ProjectAuxiliary { descriptor }),
    })
}

pub(super) fn nostr_relay_info_recipe(relay_url: &str) -> Result<ProjectRecipe, Diagnostic> {
    let relay = normalize_nostr_relay_url(relay_url)?;
    let standard = BuiltinStandard::load()?;
    let contract = standard.http_recipe_contract()?;
    let mut interner = TypeObjectInterner::default();
    let semantic_http = crate::platform::http::semantic_http_types(&mut interner)?;
    let semantic_client = crate::platform::http_client::semantic_http_client_types(&mut interner)?;
    let bool_digest = interner.intern(TypeForm::Bool)?;
    if contract.text_type != semantic_http.text_type
        || contract.bytes_type != semantic_http.bytes_type
        || semantic_client.i64_type != semantic_http.i64_type
        || semantic_client.bytes_type != semantic_http.bytes_type
        || semantic_client.text_type != semantic_http.text_type
        || semantic_client.header_type != semantic_http.header_type
        || semantic_client.header_list_type != semantic_http.header_list_type
        || semantic_client.response_type != semantic_http.response_type
    {
        return Err(recipe_error(
            "new_nostr_standard_types",
            "built-in standard HTTP server and client types disagree with canonical Graph 8 types",
        ));
    }

    let bool_type = authored_type(&interner, bool_digest)?;
    let text_type = authored_type(&interner, semantic_http.text_type)?;
    let request_type = authored_type(&interner, semantic_http.request_type)?;
    let response_type = authored_type(&interner, semantic_http.response_type)?;
    let header_type = authored_type(&interner, semantic_http.header_type)?;
    let client_response_type = authored_type(&interner, semantic_client.response_type)?;
    let function_type = authored_type(&interner, semantic_http.function_type)?;
    let mut expressions = RecipeExpressions::default();

    let header_name_source = expressions.local("$header");
    let header_name = expressions.field(header_name_source, "name")?;
    let content_type_name = expressions.text("content-type");
    let name_matches = expressions.call(
        exact_declaration(contract.text_equal),
        Vec::new(),
        vec![header_name, content_type_name],
    );
    let header_value_source = expressions.local("$header");
    let header_value = expressions.field(header_value_source, "value")?;
    let expected_media = expressions.text("application/nostr+json");
    let media_matches = expressions.call(
        exact_declaration(contract.media_type_is),
        Vec::new(),
        vec![header_value, expected_media],
    );
    let this_header_matches = expressions.call(
        exact_declaration(contract.bool_and),
        Vec::new(),
        vec![name_matches, media_matches],
    );
    let prior_match = expressions.local("$matched");
    let reducer_body = expressions.call(
        exact_declaration(contract.bool_or),
        Vec::new(),
        vec![prior_match, this_header_matches],
    );

    let method_value = expressions.local("$method");
    let get_text = expressions.text("GET");
    let method_matches = expressions.call(
        exact_declaration(contract.text_equal),
        Vec::new(),
        vec![method_value, get_text],
    );
    let path_value = expressions.local("$path");
    let relay_path = expressions.text("/relay-info");
    let path_matches = expressions.call(
        exact_declaration(contract.text_equal),
        Vec::new(),
        vec![path_value, relay_path],
    );
    let route_body = expressions.call(
        exact_declaration(contract.bool_and),
        Vec::new(),
        vec![method_matches, path_matches],
    );

    let accept_name = expressions.text("accept");
    let accept_value_text = expressions.text("application/nostr+json");
    let accept_value = expressions.call(
        exact_declaration(contract.bytes_from_text),
        Vec::new(),
        vec![accept_value_text],
    );
    let accept_header = expressions.record(vec![("name", accept_name), ("value", accept_value)])?;
    let request_headers = expressions.list(header_type.clone(), vec![accept_header]);
    let remote_call = expressions.capability(
        RELAY,
        exact_operation(contract.http_client_get),
        vec![request_headers],
    );

    let remote_status_source = expressions.lexical("$remote_response");
    let remote_status = expressions.field(remote_status_source, "status")?;
    let status_200 = expressions.i64(200);
    let status_matches = expressions.call(
        exact_declaration(contract.i64_equal),
        Vec::new(),
        vec![remote_status, status_200],
    );
    let remote_headers_source = expressions.lexical("$remote_response");
    let remote_headers = expressions.field(remote_headers_source, "headers")?;
    let no_media_match = expressions.bool(false);
    let reducer =
        expressions.function_value(local_declaration("$content_type_is_nostr"), Vec::new());
    let media_matches = expressions.call(
        exact_declaration(contract.list_fold_left),
        vec![header_type.clone(), bool_type.clone()],
        vec![remote_headers, no_media_match, reducer],
    );
    let remote_is_valid = expressions.call(
        exact_declaration(contract.bool_and),
        Vec::new(),
        vec![status_matches, media_matches],
    );
    let remote_body_source = expressions.lexical("$remote_response");
    let remote_body = expressions.field(remote_body_source, "body")?;
    let response_content_name = expressions.text("content-type");
    let response_content_text = expressions.text("application/nostr+json");
    let response_content_value = expressions.call(
        exact_declaration(contract.bytes_from_text),
        Vec::new(),
        vec![response_content_text],
    );
    let response_content_header = expressions.record(vec![
        ("name", response_content_name),
        ("value", response_content_value),
    ])?;
    let response_headers = expressions.list(header_type.clone(), vec![response_content_header]);
    let successful_status = expressions.i64(200);
    let successful_response = expressions.record(vec![
        ("body", remote_body),
        ("headers", response_headers),
        ("status", successful_status),
    ])?;
    let bad_gateway = expressions.http_response(
        exact_declaration(contract.bytes_from_text),
        header_type.clone(),
        502,
        "bad gateway",
    )?;
    let selected_remote = expressions.if_value(remote_is_valid, successful_response, bad_gateway);
    let remote_scope = expressions.let_value(
        AuthoredLetBinding {
            symbol: "$remote_response".to_owned(),
            name: name("remote-response")?,
            value: remote_call,
            declared_type: Some(client_response_type),
        },
        selected_remote,
    );

    let request_method_source = expressions.local("$request");
    let request_method = expressions.field(request_method_source, "method")?;
    let request_path_source = expressions.local("$request");
    let request_path = expressions.field(request_path_source, "path")?;
    let route_matches = expressions.call(
        local_declaration("$is_relay_info_request"),
        Vec::new(),
        vec![request_method, request_path],
    );
    let not_found = expressions.http_response(
        exact_declaration(contract.bytes_from_text),
        header_type.clone(),
        404,
        "not found",
    )?;
    let handler_body = expressions.if_value(route_matches, remote_scope, not_found);

    let valid_method = expressions.text("GET");
    let valid_path = expressions.text("/relay-info");
    let valid_actual = expressions.call(
        local_declaration("$is_relay_info_request"),
        Vec::new(),
        vec![valid_method, valid_path],
    );
    let valid_expected = expressions.bool(true);
    let invalid_method = expressions.text("POST");
    let invalid_path = expressions.text("/relay-info");
    let invalid_actual = expressions.call(
        local_declaration("$is_relay_info_request"),
        Vec::new(),
        vec![invalid_method, invalid_path],
    );
    let invalid_expected = expressions.bool(false);
    let descriptor = encode_deployment(&starter_nostr_relay_deployment(
        &relay.endpoint,
        relay.address_policy,
    )?)?;

    Ok(ProjectRecipe {
        changes: vec![
            builtin_dependency(standard),
            module()?,
            AuthoredChange::CreateFunction {
                symbol: "$content_type_is_nostr".to_owned(),
                module: local_module(),
                name: name("content-type-is-nostr")?,
                visibility: DeclarationVisibility::Private,
                type_parameters: Vec::new(),
                parameters: Vec::new(),
                result: bool_type.clone(),
                effect: AuthoredFunctionEffect::Pure {},
                body: reducer_body,
            },
            parameter(
                "$content_type_is_nostr",
                "$matched",
                "matched",
                bool_type.clone(),
            )?,
            parameter(
                "$content_type_is_nostr",
                "$header",
                "header",
                header_type.clone(),
            )?,
            AuthoredChange::CreateFunction {
                symbol: "$is_relay_info_request".to_owned(),
                module: local_module(),
                name: name("is-relay-info-request")?,
                visibility: DeclarationVisibility::Private,
                type_parameters: Vec::new(),
                parameters: Vec::new(),
                result: bool_type.clone(),
                effect: AuthoredFunctionEffect::Pure {},
                body: route_body,
            },
            parameter(
                "$is_relay_info_request",
                "$method",
                "method",
                text_type.clone(),
            )?,
            parameter("$is_relay_info_request", "$path", "path", text_type)?,
            AuthoredChange::CreateFunction {
                symbol: "$handle".to_owned(),
                module: local_module(),
                name: name("handle")?,
                visibility: DeclarationVisibility::Private,
                type_parameters: Vec::new(),
                parameters: Vec::new(),
                result: response_type,
                effect: AuthoredFunctionEffect::Task {
                    requirements: vec![local_requirement(STREAMS), local_requirement(RELAY)],
                },
                body: handler_body,
            },
            parameter("$handle", "$request", "request", request_type)?,
            component()?,
            requirement(
                STREAMS,
                "streams",
                contract.byte_stream_interface,
                &contract.byte_stream_operations,
                10_000,
            )?,
            requirement(
                RELAY,
                "relay",
                contract.http_client_interface,
                &contract.http_client_operations,
                64,
            )?,
            port("$handle", function_type)?,
            target("serve", RunnerKind::Http)?,
            AuthoredChange::CreateTest {
                symbol: "$relay_info_route_is_exact".to_owned(),
                module: local_module(),
                name: name("relay-info-route-is-exact")?,
                visibility: DeclarationVisibility::Private,
                actual: valid_actual,
                expected: valid_expected,
            },
            AuthoredChange::CreateTest {
                symbol: "$relay_info_route_rejects_other_method".to_owned(),
                module: local_module(),
                name: name("relay-info-route-rejects-other-method")?,
                visibility: DeclarationVisibility::Private,
                actual: invalid_actual,
                expected: invalid_expected,
            },
        ],
        transports: vec![standard.transport()],
        template: ProjectTemplate::NostrRelayInfo,
        auxiliary: Some(ProjectAuxiliary { descriptor }),
    })
}

fn module() -> Result<AuthoredChange, Diagnostic> {
    Ok(AuthoredChange::CreateModule {
        symbol: MODULE.to_owned(),
        name: name("application")?,
    })
}

fn component() -> Result<AuthoredChange, Diagnostic> {
    Ok(AuthoredChange::CreateComponent {
        symbol: COMPONENT.to_owned(),
        module: local_module(),
        name: name("application")?,
        visibility: DeclarationVisibility::Package,
        requirements: Vec::new(),
        ports: Vec::new(),
    })
}

fn target(target_name: &str, runner: RunnerKind) -> Result<AuthoredChange, Diagnostic> {
    Ok(AuthoredChange::CreateTarget {
        symbol: TARGET.to_owned(),
        name: name(target_name)?,
        component: local_declaration(COMPONENT),
        port: AuthoredPortReference::Symbol {
            symbol: PORT.to_owned(),
        },
        runner,
    })
}

fn port(function: &str, function_type: AuthoredType) -> Result<AuthoredChange, Diagnostic> {
    Ok(AuthoredChange::AddPort {
        component: local_declaration_selector(COMPONENT),
        port: AuthoredPort {
            symbol: PORT.to_owned(),
            name: name("http")?,
            function_type,
            implementation: AuthoredPortImplementation::Function {
                function: local_declaration(function),
            },
        },
    })
}

fn requirement(
    symbol: &str,
    requirement_name: &str,
    interface: DeclarationReference,
    operations: &[OperationReference],
    maximum_calls: u64,
) -> Result<AuthoredChange, Diagnostic> {
    Ok(AuthoredChange::AddRequirement {
        component: local_declaration_selector(COMPONENT),
        requirement: AuthoredRequirement {
            symbol: symbol.to_owned(),
            name: name(requirement_name)?,
            interface: exact_declaration(interface),
            operations: operations.iter().copied().map(exact_operation).collect(),
            limits: vec![AuthoredResourceLimit {
                name: name("maximum_calls")?,
                maximum: maximum_calls,
                unit: crate::platform::kernel::ResourceUnit::Calls,
            }],
        },
    })
}

fn parameter(
    function: &str,
    symbol: &str,
    parameter_name: &str,
    ty: AuthoredType,
) -> Result<AuthoredChange, Diagnostic> {
    Ok(AuthoredChange::AddParameter {
        parent: crate::platform::change::ParameterParentSelector::Declaration {
            declaration: local_declaration_selector(function),
        },
        parameter: AuthoredParameter {
            symbol: symbol.to_owned(),
            name: name(parameter_name)?,
            ty,
            use_mode: crate::platform::kernel::ParameterUse::Unrestricted,
            resource_requirement: None,
        },
    })
}

fn builtin_dependency(standard: &BuiltinStandard) -> AuthoredChange {
    AuthoredChange::AddDependency {
        package: standard.package,
        semantic_revision: standard.semantic_revision,
        package_revision: standard.package_revision,
    }
}

fn local_module() -> ModuleSelector {
    ModuleSelector::Symbol {
        symbol: MODULE.to_owned(),
    }
}

fn local_declaration(symbol: &str) -> AuthoredDeclarationReference {
    AuthoredDeclarationReference::Local {
        declaration: local_declaration_selector(symbol),
    }
}

fn local_declaration_selector(symbol: &str) -> DeclarationSelector {
    DeclarationSelector::Symbol {
        symbol: symbol.to_owned(),
    }
}

fn exact_declaration(reference: DeclarationReference) -> AuthoredDeclarationReference {
    AuthoredDeclarationReference::Exact {
        package: reference.package,
        declaration: reference.declaration,
    }
}

fn exact_operation(reference: OperationReference) -> AuthoredOperationReference {
    AuthoredOperationReference::Exact {
        package: reference.package,
        operation: reference.operation,
    }
}

fn local_requirement(symbol: &str) -> AuthoredRequirementReference {
    AuthoredRequirementReference::Symbol {
        symbol: symbol.to_owned(),
    }
}

fn name(value: &str) -> Result<Name, Diagnostic> {
    Name::new(value)
}

fn authored_type(
    interner: &TypeObjectInterner,
    digest: TypeObjectDigest,
) -> Result<AuthoredType, Diagnostic> {
    let object = interner.get(digest).ok_or_else(|| {
        recipe_error(
            "new_recipe_type_missing",
            format!("recipe semantic type {digest} is absent from its canonical type closure"),
        )
    })?;
    Ok(match &object.form {
        TypeForm::Unit => AuthoredType::Unit {},
        TypeForm::Bool => AuthoredType::Bool {},
        TypeForm::I64 => AuthoredType::I64 {},
        TypeForm::Bytes => AuthoredType::Bytes {},
        TypeForm::Text => AuthoredType::Text {},
        TypeForm::StaticText => AuthoredType::StaticText {},
        TypeForm::Secret => AuthoredType::Secret {},
        TypeForm::TypeParameter { parameter } => AuthoredType::TypeParameter {
            parameter: AuthoredTypeParameterReference::Id {
                parameter: *parameter,
            },
        },
        TypeForm::Named { declaration } => AuthoredType::Named {
            declaration: exact_declaration(*declaration),
        },
        TypeForm::CapabilityResource { interface } => AuthoredType::CapabilityResource {
            interface: exact_declaration(*interface),
        },
        TypeForm::StructuralRecord { fields } => AuthoredType::StructuralRecord {
            fields: fields
                .iter()
                .map(|field| {
                    Ok(AuthoredStructuralTypeField {
                        name: field.name.clone(),
                        ty: authored_type(interner, field.ty)?,
                    })
                })
                .collect::<Result<Vec<_>, Diagnostic>>()?,
        },
        TypeForm::List { item } => AuthoredType::List {
            item: Box::new(authored_type(interner, *item)?),
        },
        TypeForm::Map { key, value } => AuthoredType::Map {
            key: Box::new(authored_type(interner, *key)?),
            value: Box::new(authored_type(interner, *value)?),
        },
        TypeForm::Option { item } => AuthoredType::Option {
            item: Box::new(authored_type(interner, *item)?),
        },
        TypeForm::Result { ok, error } => AuthoredType::Result {
            ok: Box::new(authored_type(interner, *ok)?),
            error: Box::new(authored_type(interner, *error)?),
        },
        TypeForm::Stream { item } => AuthoredType::Stream {
            item: Box::new(authored_type(interner, *item)?),
        },
        TypeForm::Function { parameters, result } => AuthoredType::Function {
            parameters: parameters
                .iter()
                .map(|parameter| authored_type(interner, *parameter))
                .collect::<Result<Vec<_>, Diagnostic>>()?,
            result: Box::new(authored_type(interner, *result)?),
        },
    })
}

#[derive(Default)]
struct RecipeExpressions {
    next: u64,
}

impl RecipeExpressions {
    fn expression(&mut self, operation: AuthoredExpressionOperation) -> AuthoredExpression {
        let symbol = format!("$recipe_expression_{}", self.next);
        self.next = self.next.saturating_add(1);
        AuthoredExpression {
            symbol: Some(symbol),
            operation,
        }
    }

    fn bool(&mut self, value: bool) -> AuthoredExpression {
        self.expression(AuthoredExpressionOperation::Bool { value })
    }

    fn i64(&mut self, value: i64) -> AuthoredExpression {
        self.expression(AuthoredExpressionOperation::I64 { value })
    }

    fn text(&mut self, value: &str) -> AuthoredExpression {
        self.expression(AuthoredExpressionOperation::Text {
            value: value.to_owned(),
        })
    }

    fn static_text(&mut self, value: &str) -> AuthoredExpression {
        self.expression(AuthoredExpressionOperation::StaticText {
            value: value.to_owned(),
        })
    }

    fn local(&mut self, symbol: &str) -> AuthoredExpression {
        self.expression(AuthoredExpressionOperation::Local {
            value: AuthoredLocalReference::Symbol {
                symbol: symbol.to_owned(),
            },
        })
    }

    fn lexical(&mut self, symbol: &str) -> AuthoredExpression {
        self.local(symbol)
    }

    fn call(
        &mut self,
        function: AuthoredDeclarationReference,
        type_arguments: Vec<AuthoredType>,
        arguments: Vec<AuthoredExpression>,
    ) -> AuthoredExpression {
        self.expression(AuthoredExpressionOperation::Call {
            function,
            type_arguments,
            arguments,
        })
    }

    fn capability(
        &mut self,
        requirement: &str,
        operation: AuthoredOperationReference,
        arguments: Vec<AuthoredExpression>,
    ) -> AuthoredExpression {
        self.expression(AuthoredExpressionOperation::CapabilityCall {
            requirement: local_requirement(requirement),
            operation,
            arguments,
        })
    }

    fn function_value(
        &mut self,
        function: AuthoredDeclarationReference,
        type_arguments: Vec<AuthoredType>,
    ) -> AuthoredExpression {
        self.expression(AuthoredExpressionOperation::FunctionValue {
            function,
            type_arguments,
        })
    }

    fn field(
        &mut self,
        value: AuthoredExpression,
        field_name: &str,
    ) -> Result<AuthoredExpression, Diagnostic> {
        Ok(self.expression(AuthoredExpressionOperation::Field {
            value: Box::new(value),
            selector: AuthoredFieldSelector::Structural {
                name: name(field_name)?,
            },
        }))
    }

    fn list(
        &mut self,
        item_type: AuthoredType,
        items: Vec<AuthoredExpression>,
    ) -> AuthoredExpression {
        self.expression(AuthoredExpressionOperation::List { item_type, items })
    }

    fn record(
        &mut self,
        fields: Vec<(&str, AuthoredExpression)>,
    ) -> Result<AuthoredExpression, Diagnostic> {
        Ok(self.expression(AuthoredExpressionOperation::Record {
            nominal_type: None,
            fields: fields
                .into_iter()
                .map(|(field_name, value)| {
                    Ok(AuthoredRecordExpressionField {
                        selector: AuthoredFieldSelector::Structural {
                            name: name(field_name)?,
                        },
                        value,
                    })
                })
                .collect::<Result<Vec<_>, Diagnostic>>()?,
        }))
    }

    fn if_value(
        &mut self,
        condition: AuthoredExpression,
        when_true: AuthoredExpression,
        when_false: AuthoredExpression,
    ) -> AuthoredExpression {
        self.expression(AuthoredExpressionOperation::If {
            condition: Box::new(condition),
            when_true: Box::new(when_true),
            when_false: Box::new(when_false),
        })
    }

    fn let_value(
        &mut self,
        binding: AuthoredLetBinding,
        body: AuthoredExpression,
    ) -> AuthoredExpression {
        self.expression(AuthoredExpressionOperation::Let {
            bindings: vec![binding],
            body: Box::new(body),
        })
    }

    fn http_response(
        &mut self,
        bytes_from_text: AuthoredDeclarationReference,
        header_type: AuthoredType,
        status: i64,
        body: &str,
    ) -> Result<AuthoredExpression, Diagnostic> {
        let body_text = self.text(body);
        let body = self.call(bytes_from_text, Vec::new(), vec![body_text]);
        let headers = self.list(header_type, Vec::new());
        let status = self.i64(status);
        self.record(vec![
            ("body", body),
            ("headers", headers),
            ("status", status),
        ])
    }
}

fn recipe_error(code: &'static str, message: impl Into<String>) -> Diagnostic {
    Diagnostic::new(DiagnosticClass::Corrupt, code, message)
}
