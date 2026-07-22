//! Resolve and type-analyze parsed programs into owned HIR.

use std::collections::{HashMap, HashSet};

use lkjscript_core::{Error, Result};

use crate::ast::Expr as AstExpr;
use crate::hir::{
    self, Binding, BindingId, BindingKind, EffectSet, Expr, ExprKind, Function, LocalDefinition,
    Operation, Origin, Source, SourceId, TopLevel, Type, ValueDefinition,
};
use crate::import::Program as AstProgram;
use crate::types::parse_one;

pub(crate) fn analyze_program(program: &AstProgram) -> Result<hir::Program> {
    let mut analyzer = Analyzer::new(program)?;
    analyzer.install_operations()?;
    let pending = analyzer.collect_headers(program)?;

    let mut forms = Vec::with_capacity(pending.len());
    for form in pending {
        forms.push(analyzer.resolve_top_level(form)?);
    }
    let global_layout = analyzer.build_global_layout(&forms)?;

    Ok(hir::Program {
        sources: analyzer.sources,
        bindings: analyzer.bindings,
        forms,
        global_layout,
        main_locals: analyzer.main_locals,
    })
}

struct Analyzer {
    sources: Vec<Source>,
    bindings: Vec<Binding>,
    globals: HashMap<String, BindingId>,
    operations: HashMap<Operation, BindingId>,
    main_locals: u8,
}

impl Analyzer {
    fn new(program: &AstProgram) -> Result<Self> {
        let mut sources = Vec::with_capacity(program.files.len());
        for file in &program.files {
            let raw = u32::try_from(sources.len())
                .map_err(|_| Error::msg("too many source files for HIR SourceId"))?;
            sources.push(Source {
                id: SourceId::new(raw),
                path: file.path.clone(),
            });
        }
        Ok(Self {
            sources,
            bindings: Vec::new(),
            globals: HashMap::new(),
            operations: HashMap::new(),
            main_locals: 0,
        })
    }

    fn install_operations(&mut self) -> Result<()> {
        for operation in Operation::ALL {
            let id = self.add_binding(
                operation.name().to_string(),
                BindingKind::BuiltinOperation(*operation),
                operation.signature(),
                Origin::Builtin,
            )?;
            self.operations.insert(*operation, id);
        }
        Ok(())
    }

    fn collect_headers<'a>(&mut self, program: &'a AstProgram) -> Result<Vec<PendingTop<'a>>> {
        let mut pending = Vec::new();
        for (source_index, file) in program.files.iter().enumerate() {
            let source_raw = u32::try_from(source_index)
                .map_err(|_| Error::msg("too many source files for HIR SourceId"))?;
            let source = SourceId::new(source_raw);
            for form in &file.forms {
                match form {
                    AstExpr::Call { name, .. } if name == "import" => {}
                    AstExpr::Call { name, args } if name == "def" => {
                        pending.push(self.collect_definition(source, args)?);
                    }
                    AstExpr::Call { name, args } if name == "do" => {
                        pending.push(PendingTop::Do {
                            origin: source,
                            expressions: args,
                        });
                    }
                    other => {
                        return Err(
                            self.error(source, format!("unsupported top-level form: {other:?}"))
                        );
                    }
                }
            }
        }
        Ok(pending)
    }

    fn collect_definition<'a>(
        &mut self,
        origin: SourceId,
        args: &'a [AstExpr],
    ) -> Result<PendingTop<'a>> {
        let name = definition_name(args).map_err(|message| self.error(origin, message))?;
        match args {
            [_, AstExpr::Call {
                name: tag,
                args: type_args,
            }, value]
                if tag == "type" =>
            {
                let ty = parse_type_form(type_args)
                    .map_err(|message| self.error(origin, format!("def {name}: {message}")))?;
                let mut free = HashSet::new();
                collect_type_params(&ty, &mut free);
                if let Some(parameter) = free.into_iter().next() {
                    return Err(self.error(
                        origin,
                        format!("def {name}: value type contains unbound parameter {parameter}"),
                    ));
                }
                let binding =
                    self.add_global(origin, name, BindingKind::MutableGlobalValue, ty.clone())?;
                Ok(PendingTop::Value {
                    binding,
                    origin,
                    declared_type: ty,
                    value,
                })
            }
            [_, AstExpr::Call {
                name: tag,
                args: fn_args,
            }] if tag == "fn" => {
                let parsed = parse_function(fn_args)
                    .map_err(|message| self.error(origin, format!("def {name}: {message}")))?;
                validate_function_header(&name, &parsed)
                    .map_err(|message| self.error(origin, message))?;
                let monomorphic = Type::Fn {
                    params: parsed.signature_params.clone(),
                    ret: Box::new(parsed.signature_return.clone()),
                };
                let ty = if parsed.forall_vars.is_empty() {
                    monomorphic
                } else {
                    Type::Forall {
                        vars: parsed.forall_vars.clone(),
                        body: Box::new(monomorphic),
                    }
                };
                let binding = self.add_global(origin, name, BindingKind::Function, ty)?;
                Ok(PendingTop::Function {
                    binding,
                    origin,
                    parsed,
                })
            }
            _ => Err(self.error(
                origin,
                format!("def {name}: need fn/…/fn or type/…/type value"),
            )),
        }
    }

    fn add_global(
        &mut self,
        origin: SourceId,
        name: String,
        kind: BindingKind,
        ty: Type,
    ) -> Result<BindingId> {
        if Operation::from_name(&name).is_some() || is_contextual_name(&name) {
            return Err(self.error(
                origin,
                format!("global declaration {name} collides with a reserved operation or form"),
            ));
        }
        if self.globals.contains_key(&name) {
            return Err(self.error(origin, format!("duplicate global declaration {name}")));
        }
        let id = self.add_binding(name.clone(), kind, ty, Origin::Source(origin))?;
        self.globals.insert(name, id);
        Ok(id)
    }

    fn add_binding(
        &mut self,
        name: String,
        kind: BindingKind,
        ty: Type,
        origin: Origin,
    ) -> Result<BindingId> {
        let raw = u32::try_from(self.bindings.len())
            .map_err(|_| Error::msg("too many bindings for HIR BindingId"))?;
        let id = BindingId::new(raw);
        self.bindings.push(Binding {
            id,
            name,
            kind,
            ty,
            origin,
        });
        Ok(id)
    }

    fn binding(&self, id: BindingId) -> Result<&Binding> {
        let Some(index) = id.index() else {
            return Err(Error::msg("HIR BindingId cannot index this platform"));
        };
        self.bindings
            .get(index)
            .ok_or_else(|| Error::msg(format!("unknown HIR BindingId {}", id.raw())))
    }

    fn resolve_top_level(&mut self, pending: PendingTop<'_>) -> Result<TopLevel> {
        match pending {
            PendingTop::Function {
                binding,
                origin,
                parsed,
            } => self
                .resolve_function(binding, origin, parsed)
                .map(TopLevel::Function),
            PendingTop::Value {
                binding,
                origin,
                declared_type,
                value,
            } => {
                let (value, locals) = {
                    let mut resolver =
                        Resolver::new(self, origin, HashMap::new(), HashSet::new(), 0);
                    let value = resolver.resolve_expr(value)?;
                    let locals = resolver.local_count()?;
                    (value, locals)
                };
                if !Type::unify_assignable(&value.ty, &declared_type) {
                    let name = self.binding(binding)?.name.clone();
                    return Err(self.error(
                        origin,
                        format!(
                            "def {name}: value {:?} not assignable to {declared_type:?}",
                            value.ty
                        ),
                    ));
                }
                self.main_locals = self.main_locals.max(locals);
                Ok(TopLevel::Value(ValueDefinition {
                    binding,
                    origin,
                    value,
                }))
            }
            PendingTop::Do {
                origin,
                expressions,
            } => {
                let (expression, locals) = {
                    let mut resolver =
                        Resolver::new(self, origin, HashMap::new(), HashSet::new(), 0);
                    let expression = resolver.resolve_do(expressions)?;
                    let locals = resolver.local_count()?;
                    (expression, locals)
                };
                self.main_locals = self.main_locals.max(locals);
                Ok(TopLevel::Do { origin, expression })
            }
        }
    }

    fn resolve_function(
        &mut self,
        binding: BindingId,
        origin: SourceId,
        parsed: ParsedFunction<'_>,
    ) -> Result<Function> {
        let arity = u8::try_from(parsed.param_names.len()).map_err(|_| {
            self.error(
                origin,
                "function has too many parameters for bytecode arity",
            )
        })?;
        let mut params = Vec::with_capacity(parsed.param_names.len());
        let mut scope = HashMap::new();
        for (index, (name, ty)) in parsed
            .param_names
            .iter()
            .zip(&parsed.param_types)
            .enumerate()
        {
            let _slot = u8::try_from(index)
                .map_err(|_| self.error(origin, "function has too many parameter local slots"))?;
            let id = self.add_binding(
                name.clone(),
                BindingKind::Parameter,
                ty.clone(),
                Origin::Source(origin),
            )?;
            scope.insert(name.clone(), id);
            params.push(id);
        }

        let (body, local_count) = {
            let type_variables = parsed.forall_vars.iter().cloned().collect();
            let mut resolver = Resolver::new(self, origin, scope, type_variables, params.len());
            let body = resolver.resolve_expr(parsed.body)?;
            let local_count = resolver.local_count()?;
            (body, local_count)
        };
        if !Type::unify_assignable(&body.ty, &parsed.signature_return) {
            let name = self.binding(binding)?.name.clone();
            return Err(self.error(
                origin,
                format!(
                    "def {name}: body type {:?} not assignable to {:?}",
                    body.ty, parsed.signature_return
                ),
            ));
        }

        Ok(Function {
            binding,
            origin,
            params,
            arity,
            local_count,
            body,
        })
    }

    fn build_global_layout(&self, forms: &[TopLevel]) -> Result<Vec<BindingId>> {
        let mut layout = Vec::new();
        let mut seen = HashSet::new();
        for operation in Operation::CORE_GLOBALS {
            let Some(binding) = self.operations.get(operation).copied() else {
                return Err(Error::msg(format!(
                    "missing canonical operation binding for {}",
                    operation.name()
                )));
            };
            record_global(binding, &mut layout, &mut seen)?;
        }
        for form in forms {
            match form {
                TopLevel::Function(function) => {
                    self.record_expr_globals(&function.body, &mut layout, &mut seen)?;
                    record_global(function.binding, &mut layout, &mut seen)?;
                }
                TopLevel::Value(value) => {
                    self.record_expr_globals(&value.value, &mut layout, &mut seen)?;
                    record_global(value.binding, &mut layout, &mut seen)?;
                }
                TopLevel::Do { expression, .. } => {
                    self.record_expr_globals(expression, &mut layout, &mut seen)?;
                }
            }
        }
        Ok(layout)
    }

    fn record_expr_globals(
        &self,
        expression: &Expr,
        layout: &mut Vec<BindingId>,
        seen: &mut HashSet<BindingId>,
    ) -> Result<()> {
        match &expression.kind {
            ExprKind::LitI64(_)
            | ExprKind::LitF64(_)
            | ExprKind::LitBool(_)
            | ExprKind::LitUnit
            | ExprKind::EmptyList
            | ExprKind::LitNone
            | ExprKind::LitStr(_)
            | ExprKind::QuoteSymbol(_) => {}
            ExprKind::Load(binding) => {
                if self.is_global_storage(*binding)? {
                    record_global(*binding, layout, seen)?;
                }
            }
            ExprKind::Call { callee, args } => {
                for argument in args {
                    self.record_expr_globals(argument, layout, seen)?;
                }
                if self.is_global_storage(*callee)? {
                    record_global(*callee, layout, seen)?;
                }
            }
            ExprKind::Operation { args, .. } | ExprKind::Do(args) => {
                for argument in args {
                    self.record_expr_globals(argument, layout, seen)?;
                }
            }
            ExprKind::If {
                condition,
                then_branch,
                else_branch,
            } => {
                self.record_expr_globals(condition, layout, seen)?;
                self.record_expr_globals(then_branch, layout, seen)?;
                self.record_expr_globals(else_branch, layout, seen)?;
            }
            ExprKind::While { condition, body } => {
                self.record_expr_globals(condition, layout, seen)?;
                for expression in body {
                    self.record_expr_globals(expression, layout, seen)?;
                }
            }
            ExprKind::Let { bindings, body } => {
                for binding in bindings {
                    self.record_expr_globals(&binding.value, layout, seen)?;
                }
                self.record_expr_globals(body, layout, seen)?;
            }
            ExprKind::SetGlobal { target, value } => {
                self.record_expr_globals(value, layout, seen)?;
                record_global(*target, layout, seen)?;
            }
        }
        Ok(())
    }

    fn is_global_storage(&self, binding: BindingId) -> Result<bool> {
        Ok(matches!(
            self.binding(binding)?.kind,
            BindingKind::Function | BindingKind::MutableGlobalValue
        ))
    }

    fn error(&self, origin: SourceId, message: impl Into<String>) -> Error {
        let label = origin
            .index()
            .and_then(|index| self.sources.get(index))
            .map_or_else(
                || format!("source#{}", origin.raw()),
                |source| source.path.display().to_string(),
            );
        Error::msg(format!("{label}: {}", message.into()))
    }
}

fn record_global(
    binding: BindingId,
    layout: &mut Vec<BindingId>,
    seen: &mut HashSet<BindingId>,
) -> Result<()> {
    if seen.insert(binding) {
        let _slot = u16::try_from(layout.len())
            .map_err(|_| Error::msg("too many resolved globals for bytecode u16 slots"))?;
        layout.push(binding);
    }
    Ok(())
}

struct Resolver<'a> {
    analyzer: &'a mut Analyzer,
    origin: SourceId,
    scopes: Vec<HashMap<String, BindingId>>,
    type_variables: HashSet<String>,
    next_slot: usize,
    max_slots: usize,
}

impl<'a> Resolver<'a> {
    fn new(
        analyzer: &'a mut Analyzer,
        origin: SourceId,
        function_scope: HashMap<String, BindingId>,
        type_variables: HashSet<String>,
        parameter_count: usize,
    ) -> Self {
        Self {
            analyzer,
            origin,
            scopes: vec![function_scope],
            type_variables,
            next_slot: parameter_count,
            max_slots: parameter_count,
        }
    }

    fn local_count(&self) -> Result<u8> {
        u8::try_from(self.max_slots)
            .map_err(|_| self.error("expression needs more than 255 bytecode local slots"))
    }

    fn resolve_expr(&mut self, expression: &AstExpr) -> Result<Expr> {
        match expression {
            AstExpr::LitUnit => Ok(self.expression(Type::Unit, ExprKind::LitUnit)),
            AstExpr::LitBool(value) => Ok(self.expression(Type::Bool, ExprKind::LitBool(*value))),
            AstExpr::LitI64(value) => Ok(self.expression(Type::I64, ExprKind::LitI64(*value))),
            AstExpr::LitF64(value) => Ok(self.expression(Type::F64, ExprKind::LitF64(*value))),
            AstExpr::LitStr(value) => {
                Ok(self.expression(Type::Str, ExprKind::LitStr(value.clone())))
            }
            AstExpr::Symbol(name) => self.resolve_load(name),
            AstExpr::List(_) => Err(self.error("raw list literal needs typed construction")),
            AstExpr::Call { name, args } => self.resolve_call(name, args),
        }
    }

    fn resolve_load(&self, name: &str) -> Result<Expr> {
        let binding = self
            .lookup(name)
            .ok_or_else(|| self.error(format!("unknown symbol {name}")))?;
        let resolved = self.analyzer.binding(binding)?;
        if matches!(resolved.kind, BindingKind::BuiltinOperation(_)) {
            return Err(self.error(format!(
                "operation {name} is not a first-class value; call it directly"
            )));
        }
        let ty = resolved.ty.clone();
        Ok(self.expression(ty, ExprKind::Load(binding)))
    }

    fn resolve_call(&mut self, name: &str, args: &[AstExpr]) -> Result<Expr> {
        match name {
            "if" => self.resolve_if(args),
            "while" => self.resolve_while(args),
            "do" => self.resolve_do(args),
            "let" => self.resolve_let(args),
            "quote" => self.resolve_quote(args),
            "set" => self.resolve_set(args),
            "empty-list" => self.resolve_empty_list(args),
            "none" => self.resolve_none(args),
            "bind" => Err(self.error("bind is only valid inside let")),
            "fn" | "def" | "sig" | "params" | "forall" | "type" | "import" | "name" => {
                Err(self.error(format!("{name} is only valid in its declaration context")))
            }
            _ => self.resolve_plain_call(name, args),
        }
    }

    fn resolve_plain_call(&mut self, name: &str, args: &[AstExpr]) -> Result<Expr> {
        let callee = self
            .lookup_call(name)
            .ok_or_else(|| self.error(format!("unknown call {name}")))?;
        let (kind, callee_type) = {
            let binding = self.analyzer.binding(callee)?;
            (binding.kind.clone(), binding.ty.clone())
        };
        let expected = callable_arity(&callee_type)
            .ok_or_else(|| self.error(format!("{name} is not a function ({callee_type:?})")))?;
        if expected != args.len() {
            return Err(self.error(format!(
                "{name}: expected {expected} args, got {}",
                args.len()
            )));
        }
        let _arity = u8::try_from(args.len())
            .map_err(|_| self.error(format!("{name}: too many call arguments")))?;
        let mut resolved_args = Vec::with_capacity(args.len());
        for argument in args {
            resolved_args.push(self.resolve_expr(argument)?);
        }

        if let BindingKind::BuiltinOperation(operation) = kind {
            let argument_types: Vec<_> = resolved_args
                .iter()
                .map(|argument| argument.ty.clone())
                .collect();
            let (resolved_signature, ty) = operation
                .resolve_types(&argument_types)
                .map_err(|message| self.error(message))?;
            Ok(self.expression(
                ty,
                ExprKind::Operation {
                    binding: callee,
                    operation,
                    resolved_signature,
                    args: resolved_args,
                },
            ))
        } else {
            let ty = self.call_result(name, callee_type, &resolved_args)?;
            Ok(self.expression(
                ty,
                ExprKind::Call {
                    callee,
                    args: resolved_args,
                },
            ))
        }
    }

    fn call_result(&self, name: &str, callable: Type, args: &[Expr]) -> Result<Type> {
        let instantiated = self.instantiate(name, callable, args)?;
        let Type::Fn { params, ret } = instantiated else {
            return Err(self.error(format!("{name} is not a function")));
        };
        if params.len() != args.len() {
            return Err(self.error(format!(
                "{name}: expected {} args, got {}",
                params.len(),
                args.len()
            )));
        }
        for (parameter, argument) in params.iter().zip(args) {
            if !Type::unify_assignable(&argument.ty, parameter) {
                return Err(self.error(format!(
                    "{name}: arg type {:?} not assignable to {parameter:?}",
                    argument.ty
                )));
            }
        }
        Ok(*ret)
    }

    fn instantiate(&self, name: &str, callable: Type, args: &[Expr]) -> Result<Type> {
        let Type::Forall { vars, body } = callable else {
            return Ok(callable);
        };
        let Type::Fn { params, ret } = *body else {
            return Err(self.error("forall body must be a function type"));
        };
        if params.len() != args.len() {
            return Ok(Type::Fn { params, ret });
        }
        let mut substitutions = HashMap::new();
        for (pattern, argument) in params.iter().zip(args) {
            self.bind_type_params(name, pattern, &argument.ty, &vars, &mut substitutions)?;
        }
        for variable in &vars {
            if !substitutions.contains_key(variable) {
                return Err(self.error(format!(
                    "{name}: cannot infer type parameter {variable} from arguments"
                )));
            }
        }
        Ok(Type::Fn {
            params: params
                .iter()
                .map(|parameter| parameter.subst(&substitutions))
                .collect(),
            ret: Box::new(ret.subst(&substitutions)),
        })
    }

    fn bind_type_params(
        &self,
        function: &str,
        pattern: &Type,
        got: &Type,
        variables: &[String],
        substitutions: &mut HashMap<String, Type>,
    ) -> Result<()> {
        match (pattern, got) {
            (Type::Param(parameter), got)
                if variables.iter().any(|variable| variable == parameter) =>
            {
                if let Some(previous) = substitutions.get(parameter) {
                    if previous != got {
                        return Err(self.error(format!(
                            "{function}: type param {parameter} conflict: {previous:?} vs {got:?}"
                        )));
                    }
                } else {
                    substitutions.insert(parameter.clone(), got.clone());
                }
                Ok(())
            }
            (Type::List(pattern), Type::List(got)) => {
                self.bind_type_params(function, pattern, got, variables, substitutions)
            }
            (Type::Option(pattern), Type::Option(got)) => {
                self.bind_type_params(function, pattern, got, variables, substitutions)
            }
            (Type::Result(ok_pattern, err_pattern), Type::Result(ok_got, err_got)) => {
                self.bind_type_params(function, ok_pattern, ok_got, variables, substitutions)?;
                self.bind_type_params(function, err_pattern, err_got, variables, substitutions)
            }
            (pattern, got) if Type::unify_assignable(got, pattern) => Ok(()),
            (pattern, got) => Err(self.error(format!(
                "{function}: cannot instantiate {pattern:?} from {got:?}"
            ))),
        }
    }

    fn resolve_do(&mut self, args: &[AstExpr]) -> Result<Expr> {
        let mut expressions = Vec::with_capacity(args.len());
        for argument in args {
            expressions.push(self.resolve_expr(argument)?);
        }
        let ty = expressions
            .last()
            .map_or(Type::Unit, |expression| expression.ty.clone());
        Ok(self.expression(ty, ExprKind::Do(expressions)))
    }

    fn resolve_if(&mut self, args: &[AstExpr]) -> Result<Expr> {
        let [condition, then_branch, else_branch] = args else {
            return Err(self.error("if expects condition, then, and else"));
        };
        let condition = self.resolve_expr(condition)?;
        if condition.ty != Type::Bool {
            return Err(self.error("if condition must be Bool"));
        }
        let then_branch = self.resolve_expr(then_branch)?;
        let else_branch = self.resolve_expr(else_branch)?;
        if then_branch.ty != else_branch.ty {
            return Err(self.error(format!(
                "if branches must have the same type: {:?} vs {:?}",
                then_branch.ty, else_branch.ty
            )));
        }
        let ty = then_branch.ty.clone();
        Ok(self.expression(
            ty,
            ExprKind::If {
                condition: Box::new(condition),
                then_branch: Box::new(then_branch),
                else_branch: Box::new(else_branch),
            },
        ))
    }

    fn resolve_while(&mut self, args: &[AstExpr]) -> Result<Expr> {
        let Some((condition, body)) = args.split_first() else {
            return Err(self.error("while needs a condition"));
        };
        let condition = self.resolve_expr(condition)?;
        if !Type::unify_assignable(&condition.ty, &Type::Bool) {
            return Err(self.error("while condition must be Bool"));
        }
        let mut resolved_body = Vec::with_capacity(body.len());
        for expression in body {
            resolved_body.push(self.resolve_expr(expression)?);
        }
        Ok(self.expression(
            Type::Unit,
            ExprKind::While {
                condition: Box::new(condition),
                body: resolved_body,
            },
        ))
    }

    fn resolve_let(&mut self, args: &[AstExpr]) -> Result<Expr> {
        let Some((body, bindings)) = args.split_last() else {
            return Err(self.error("let needs body"));
        };
        self.scopes.push(HashMap::new());
        let saved_slot = self.next_slot;
        let mut resolved_bindings = Vec::with_capacity(bindings.len());
        for binding in bindings {
            let (name, value) = match binding {
                AstExpr::Call { name, args } if name == "bind" => match args.as_slice() {
                    [name, value] => (
                        symbolic_name(name).map_err(|message| self.error(message))?,
                        value,
                    ),
                    _ => return Err(self.error("bind needs name and value")),
                },
                _ => return Err(self.error("let bindings must be bind/…/bind")),
            };
            let value = self.resolve_expr(value)?;
            let slot = u8::try_from(self.next_slot)
                .map_err(|_| self.error("let needs more than 255 bytecode local slots"))?;
            self.next_slot = self
                .next_slot
                .checked_add(1)
                .ok_or_else(|| self.error("local slot count overflow"))?;
            self.max_slots = self.max_slots.max(self.next_slot);
            let binding_id = self.analyzer.add_binding(
                name.clone(),
                BindingKind::ImmutableLocal,
                value.ty.clone(),
                Origin::Source(self.origin),
            )?;
            if self
                .scopes
                .last()
                .is_some_and(|scope| scope.contains_key(&name))
            {
                return Err(self.error(format!("duplicate let binding {name}")));
            }
            let Some(scope) = self.scopes.last_mut() else {
                return Err(self.error("missing lexical scope while resolving let"));
            };
            scope.insert(name, binding_id);
            resolved_bindings.push(LocalDefinition {
                binding: binding_id,
                slot,
                value,
            });
        }
        let body = self.resolve_expr(body)?;
        self.next_slot = saved_slot;
        let _removed_scope = self.scopes.pop();
        let ty = body.ty.clone();
        Ok(self.expression(
            ty,
            ExprKind::Let {
                bindings: resolved_bindings,
                body: Box::new(body),
            },
        ))
    }

    fn resolve_set(&mut self, args: &[AstExpr]) -> Result<Expr> {
        let (target_name, value_ast) = match args {
            [target, value] => (
                symbolic_name(target).map_err(|message| self.error(message))?,
                value,
            ),
            _ => return Err(self.error("set needs name and value")),
        };
        let target = self
            .analyzer
            .globals
            .get(&target_name)
            .copied()
            .ok_or_else(|| self.error(format!("unknown set target {target_name}")))?;
        let (kind, target_type) = {
            let binding = self.analyzer.binding(target)?;
            (binding.kind.clone(), binding.ty.clone())
        };
        if kind != BindingKind::MutableGlobalValue {
            return Err(self.error(format!(
                "set target {target_name} is not a mutable global value"
            )));
        }
        let value = self.resolve_expr(value_ast)?;
        if !Type::unify_assignable(&value.ty, &target_type) {
            return Err(self.error(format!(
                "set target {target_name}: value type {:?} not assignable to {target_type:?}",
                value.ty
            )));
        }
        Ok(self.expression(
            Type::Unit,
            ExprKind::SetGlobal {
                target,
                value: Box::new(value),
            },
        ))
    }

    fn resolve_empty_list(&mut self, args: &[AstExpr]) -> Result<Expr> {
        let element = parse_type_form(args)
            .map_err(|message| self.error(format!("empty-list: {message}")))?;
        let mut parameters = HashSet::new();
        collect_type_params(&element, &mut parameters);
        if let Some(parameter) = parameters
            .into_iter()
            .find(|parameter| !self.type_variables.contains(*parameter))
        {
            return Err(self.error(format!(
                "empty-list: type parameter {parameter} is not declared by forall"
            )));
        }
        Ok(self.expression(Type::List(Box::new(element)), ExprKind::EmptyList))
    }

    fn resolve_none(&mut self, args: &[AstExpr]) -> Result<Expr> {
        let value_type =
            parse_type_form(args).map_err(|message| self.error(format!("none: {message}")))?;
        let mut parameters = HashSet::new();
        collect_type_params(&value_type, &mut parameters);
        if let Some(parameter) = parameters
            .into_iter()
            .find(|parameter| !self.type_variables.contains(*parameter))
        {
            return Err(self.error(format!(
                "none: type parameter {parameter} is not declared by forall"
            )));
        }
        Ok(self.expression(Type::Option(Box::new(value_type)), ExprKind::LitNone))
    }

    fn resolve_quote(&mut self, args: &[AstExpr]) -> Result<Expr> {
        let symbol = match args {
            [AstExpr::Symbol(symbol)] => symbol.clone(),
            [_] => return Err(self.error("quote accepts only a symbol")),
            _ => return Err(self.error("quote expects one symbol")),
        };
        Ok(self.expression(Type::Symbol, ExprKind::QuoteSymbol(symbol)))
    }

    fn lookup(&self, name: &str) -> Option<BindingId> {
        self.lookup_lexical(name)
            .or_else(|| self.analyzer.globals.get(name).copied())
            .or_else(|| {
                Operation::from_name(name)
                    .and_then(|operation| self.analyzer.operations.get(&operation).copied())
            })
    }

    fn lookup_call(&self, name: &str) -> Option<BindingId> {
        self.lookup_lexical(name)
            .or_else(|| self.analyzer.globals.get(name).copied())
            .or_else(|| {
                Operation::from_name(name)
                    .and_then(|operation| self.analyzer.operations.get(&operation).copied())
            })
    }

    fn lookup_lexical(&self, name: &str) -> Option<BindingId> {
        for scope in self.scopes.iter().rev() {
            if let Some(binding) = scope.get(name) {
                return Some(*binding);
            }
        }
        None
    }

    fn expression(&self, ty: Type, kind: ExprKind) -> Expr {
        let effects = self.effects(&kind);
        Expr {
            ty,
            effects,
            origin: self.origin,
            kind,
        }
    }

    fn effects(&self, kind: &ExprKind) -> EffectSet {
        match kind {
            ExprKind::LitI64(_)
            | ExprKind::LitF64(_)
            | ExprKind::LitBool(_)
            | ExprKind::LitUnit
            | ExprKind::EmptyList
            | ExprKind::LitNone
            | ExprKind::LitStr(_)
            | ExprKind::QuoteSymbol(_) => EffectSet::PURE,
            ExprKind::Load(binding) => self
                .analyzer
                .binding(*binding)
                .ok()
                .and_then(|binding| {
                    (binding.kind == BindingKind::MutableGlobalValue)
                        .then_some(EffectSet::READS_MEMORY)
                })
                .unwrap_or(EffectSet::PURE),
            ExprKind::Call { args, .. } => fold_effects(args).union(EffectSet::CONSERVATIVE_CALL),
            ExprKind::Operation {
                operation, args, ..
            } => fold_effects(args).union(operation.effects()),
            ExprKind::Do(expressions) => fold_effects(expressions),
            ExprKind::If {
                condition,
                then_branch,
                else_branch,
            } => condition
                .effects
                .union(then_branch.effects)
                .union(else_branch.effects),
            ExprKind::While { condition, body } => condition
                .effects
                .union(fold_effects(body))
                .union(EffectSet::MAY_DIVERGE),
            ExprKind::Let { bindings, body } => bindings
                .iter()
                .fold(EffectSet::PURE, |effects, binding| {
                    effects.union(binding.value.effects)
                })
                .union(body.effects),
            ExprKind::SetGlobal { value, .. } => value.effects.union(EffectSet::WRITES_MEMORY),
        }
    }

    fn error(&self, message: impl Into<String>) -> Error {
        self.analyzer.error(self.origin, message)
    }
}

fn fold_effects(expressions: &[Expr]) -> EffectSet {
    expressions
        .iter()
        .fold(EffectSet::PURE, |effects, expression| {
            effects.union(expression.effects)
        })
}

fn callable_arity(ty: &Type) -> Option<usize> {
    match ty {
        Type::Fn { params, .. } => Some(params.len()),
        Type::Forall { body, .. } => callable_arity(body),
        _ => None,
    }
}

fn is_contextual_name(name: &str) -> bool {
    matches!(
        name,
        "if" | "while"
            | "do"
            | "let"
            | "quote"
            | "set"
            | "empty-list"
            | "none"
            | "bind"
            | "fn"
            | "def"
            | "sig"
            | "params"
            | "forall"
            | "type"
            | "import"
            | "name"
    )
}

enum PendingTop<'a> {
    Function {
        binding: BindingId,
        origin: SourceId,
        parsed: ParsedFunction<'a>,
    },
    Value {
        binding: BindingId,
        origin: SourceId,
        declared_type: Type,
        value: &'a AstExpr,
    },
    Do {
        origin: SourceId,
        expressions: &'a [AstExpr],
    },
}

struct ParsedFunction<'a> {
    signature_params: Vec<Type>,
    signature_return: Type,
    param_names: Vec<String>,
    param_types: Vec<Type>,
    body: &'a AstExpr,
    forall_vars: Vec<String>,
}

fn definition_name(args: &[AstExpr]) -> std::result::Result<String, String> {
    match args.first() {
        Some(AstExpr::Call { name, args }) if name == "name" => match args.as_slice() {
            [AstExpr::LitStr(name)] | [AstExpr::Symbol(name)] => Ok(name.clone()),
            [AstExpr::Call { name, args }] if args.is_empty() => Ok(name.clone()),
            _ => Err("def name must be a symbol/string".into()),
        },
        _ => Err("def expects name/…/name first".into()),
    }
}

fn parse_function(args: &[AstExpr]) -> std::result::Result<ParsedFunction<'_>, String> {
    let mut signature = None;
    let mut params = None;
    let mut body = None;
    let mut forall_vars = Vec::new();
    let mut saw_forall = false;
    for argument in args {
        match argument {
            AstExpr::Call { name, args } if name == "forall" => {
                if saw_forall {
                    return Err("fn has multiple forall blocks".into());
                }
                saw_forall = true;
                for variable in args {
                    forall_vars.push(symbolic_name(variable)?);
                }
            }
            AstExpr::Call { name, args } if name == "sig" => {
                if signature.is_some() {
                    return Err("fn has multiple sig blocks".into());
                }
                signature = Some(parse_signature(args)?);
            }
            AstExpr::Call { name, args } if name == "params" => {
                if params.is_some() {
                    return Err("fn has multiple params blocks".into());
                }
                params = Some(parse_typed_params(args)?);
            }
            other => {
                if body.is_some() {
                    return Err("fn has multiple body expressions; wrap in do/".into());
                }
                body = Some(other);
            }
        }
    }
    let (signature_params, signature_return) =
        signature.ok_or_else(|| "fn missing mandatory sig/…/sig".to_string())?;
    let (param_names, param_types) =
        params.ok_or_else(|| "fn missing params/…/params".to_string())?;
    let body = body.ok_or_else(|| "fn missing body".to_string())?;
    Ok(ParsedFunction {
        signature_params,
        signature_return,
        param_names,
        param_types,
        body,
        forall_vars,
    })
}

fn validate_function_header(
    name: &str,
    parsed: &ParsedFunction<'_>,
) -> std::result::Result<(), String> {
    if parsed.signature_params.len() != parsed.param_types.len()
        || parsed.signature_params.len() != parsed.param_names.len()
    {
        return Err(format!("def {name}: sig/params arity mismatch"));
    }
    let mut names = HashSet::new();
    for parameter in &parsed.param_names {
        if !names.insert(parameter) {
            return Err(format!("def {name}: duplicate parameter {parameter}"));
        }
    }
    for (signature, parameter) in parsed.signature_params.iter().zip(&parsed.param_types) {
        if signature != parameter {
            return Err(format!(
                "def {name}: parameter type mismatch between sig and params"
            ));
        }
    }

    let mut declared = HashSet::new();
    for variable in &parsed.forall_vars {
        if !declared.insert(variable.as_str()) {
            return Err(format!("def {name}: duplicate forall variable {variable}"));
        }
    }
    let mut used = HashSet::new();
    for ty in parsed
        .signature_params
        .iter()
        .chain(parsed.param_types.iter())
        .chain(std::iter::once(&parsed.signature_return))
    {
        collect_type_params(ty, &mut used);
    }
    for variable in &used {
        if !declared.contains(*variable) {
            return Err(format!(
                "def {name}: type parameter {variable} is not declared by forall"
            ));
        }
    }
    for variable in &parsed.forall_vars {
        if !used.contains(variable.as_str()) {
            return Err(format!("def {name}: unused forall variable {variable}"));
        }
    }
    Ok(())
}

fn collect_type_params<'a>(ty: &'a Type, output: &mut HashSet<&'a str>) {
    match ty {
        Type::Param(parameter) => {
            output.insert(parameter);
        }
        Type::List(inner) | Type::Option(inner) => collect_type_params(inner, output),
        Type::Result(ok, error) => {
            collect_type_params(ok, output);
            collect_type_params(error, output);
        }
        Type::Fn { params, ret } => {
            for parameter in params {
                collect_type_params(parameter, output);
            }
            collect_type_params(ret, output);
        }
        Type::Forall { body, .. } => collect_type_params(body, output),
        _ => {}
    }
}

fn parse_signature(args: &[AstExpr]) -> std::result::Result<(Vec<Type>, Type), String> {
    let atoms = type_atoms(args)?;
    Type::parse_atoms(&atoms)
}

fn parse_type_form(args: &[AstExpr]) -> std::result::Result<Type, String> {
    if args.len() == 1 {
        return parameter_type(&args[0]);
    }
    let atoms = type_atoms(args)?;
    let (ty, end) = parse_one(&atoms, 0)?;
    if end != atoms.len() {
        return Err("trailing tokens in type/".into());
    }
    Ok(ty)
}

fn type_atoms(args: &[AstExpr]) -> std::result::Result<Vec<String>, String> {
    let mut atoms = Vec::with_capacity(args.len());
    for argument in args {
        match argument {
            AstExpr::Symbol(atom) => atoms.push(atom.clone()),
            AstExpr::Call { name, args } if args.is_empty() => atoms.push(name.clone()),
            _ => return Err("type atoms must be names or ->".into()),
        }
    }
    Ok(atoms)
}

fn parse_typed_params(args: &[AstExpr]) -> std::result::Result<(Vec<String>, Vec<Type>), String> {
    if !args.len().is_multiple_of(2) {
        return Err("params must be name Type pairs".into());
    }
    let mut names = Vec::with_capacity(args.len() / 2);
    let mut types = Vec::with_capacity(args.len() / 2);
    let mut index = 0;
    while index < args.len() {
        names.push(symbolic_name(&args[index])?);
        types.push(parameter_type(&args[index + 1])?);
        index += 2;
    }
    Ok((names, types))
}

fn parameter_type(expression: &AstExpr) -> std::result::Result<Type, String> {
    match expression {
        AstExpr::Symbol(name) => atom_type(name),
        AstExpr::Call { name, args } if args.is_empty() => atom_type(name),
        AstExpr::Call { name, args } if name == "List" && args.len() == 1 => {
            Ok(Type::List(Box::new(parameter_type(&args[0])?)))
        }
        AstExpr::Call { name, args } if name == "Option" && args.len() == 1 => {
            Ok(Type::Option(Box::new(parameter_type(&args[0])?)))
        }
        AstExpr::Call { name, args } if name == "Result" && args.len() == 2 => Ok(Type::Result(
            Box::new(parameter_type(&args[0])?),
            Box::new(parameter_type(&args[1])?),
        )),
        _ => Err("invalid parameter type expression".into()),
    }
}

fn atom_type(name: &str) -> std::result::Result<Type, String> {
    let (ty, end) = parse_one(&[name.to_string()], 0)?;
    if end == 1 {
        Ok(ty)
    } else {
        Err(format!("bad type {name}"))
    }
}

fn symbolic_name(expression: &AstExpr) -> std::result::Result<String, String> {
    match expression {
        AstExpr::Symbol(name) => Ok(name.clone()),
        AstExpr::Call { name, args } if args.is_empty() => Ok(name.clone()),
        _ => Err("name must be a symbol".into()),
    }
}

#[cfg(test)]
include!("analyze/tests.rs");
