//! Resolve and type-analyze parsed programs into owned HIR.

use std::collections::{HashMap, HashSet};

use lkjscript_core::{Error, ProductId, Result, MAX_PRODUCT_FIELDS};

use crate::ast::Expr as AstExpr;
use crate::hir::{
    self, Binding, BindingId, BindingKind, BindingRef, BindingStorage, BorrowKind, CoreTrait,
    EffectSet, Expr, ExprKind, Function, GenericInstantiation, ImplDefinition, ImplId, LoanId,
    LocalDefinition, Main, Operation, Origin, PlaceId, ProductDefinition, ProductField, Source,
    SourceId, TraitBound, TraitDefinition, TraitId, TraitWitness, TraitWitnessKind, Type,
    TypeSubstitution,
};

pub const TRAIT_SOLVER_MAX_DEPTH: usize = 32;
pub const TRAIT_SOLVER_MAX_WORK: usize = 256;
use crate::import::Program as AstProgram;
use crate::types::parse_one;

pub(crate) fn analyze_program(program: &AstProgram) -> Result<hir::Program> {
    let mut program = analyze_program_without_effects(program)?;
    crate::effects::infer(&mut program);
    Ok(program)
}

pub(crate) fn analyze_program_without_effects(program: &AstProgram) -> Result<hir::Program> {
    let mut analyzer = Analyzer::new(program)?;
    analyzer.install_operations()?;
    analyzer.install_core_traits()?;
    analyzer.collect_trait_names(program)?;
    analyzer.collect_product_names(program)?;
    analyzer.collect_products(program)?;
    analyzer.collect_implementations(program)?;
    let (pending_functions, pending_main) = analyzer.collect_headers(program)?;

    let mut functions = Vec::with_capacity(pending_functions.len());
    for function in pending_functions {
        functions.push(analyzer.resolve_function(
            function.binding,
            function.origin,
            function.parsed,
            function.bounds,
        )?);
    }
    let main = analyzer.resolve_main(pending_main)?;
    let global_layout = analyzer.build_global_layout(&functions)?;

    let program = hir::Program {
        sources: analyzer.sources,
        bindings: analyzer.bindings,
        products: analyzer.products,
        traits: analyzer.traits,
        implementations: analyzer.implementations,
        functions,
        main,
        global_layout,
    };
    crate::ownership::check(&program)?;
    Ok(program)
}

struct Analyzer {
    sources: Vec<Source>,
    bindings: Vec<Binding>,
    globals: HashMap<String, BindingId>,
    operations: HashMap<Operation, BindingId>,
    product_names: HashMap<String, ProductId>,
    products: Vec<ProductDefinition>,
    trait_names: HashMap<String, TraitId>,
    traits: Vec<TraitDefinition>,
    implementations: Vec<ImplDefinition>,
    implementation_index: HashMap<(TraitId, ProductId), ImplId>,
    function_bounds: HashMap<BindingId, Vec<TraitBound>>,
    next_loan: u32,
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
            product_names: HashMap::new(),
            products: Vec::new(),
            trait_names: HashMap::new(),
            traits: Vec::new(),
            implementations: Vec::new(),
            implementation_index: HashMap::new(),
            function_bounds: HashMap::new(),
            next_loan: 0,
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

    fn install_core_traits(&mut self) -> Result<()> {
        for core in CoreTrait::ALL {
            let raw = u32::try_from(self.traits.len())
                .map_err(|_| Error::msg("too many traits for HIR TraitId"))?;
            let id = TraitId::new(raw);
            let name = core.name().to_string();
            self.trait_names.insert(name.clone(), id);
            self.traits.push(TraitDefinition {
                id,
                name,
                origin: Origin::Builtin,
                core: Some(core),
            });
        }
        Ok(())
    }

    fn collect_trait_names(&mut self, program: &AstProgram) -> Result<()> {
        for (source_index, file) in program.files.iter().enumerate() {
            let source = SourceId::new(
                u32::try_from(source_index)
                    .map_err(|_| Error::msg("too many source files for HIR SourceId"))?,
            );
            for form in &file.forms {
                let AstExpr::Call { name, args } = form else {
                    continue;
                };
                if name != "trait" {
                    continue;
                }
                let trait_name =
                    trait_declaration(args).map_err(|message| self.error(source, message))?;
                if !is_declaration_type_name(&trait_name) {
                    return Err(self.error(
                        source,
                        format!("invalid trait declaration name {trait_name}"),
                    ));
                }
                if CoreTrait::ALL.iter().any(|core| core.name() == trait_name) {
                    return Err(self.error(
                        source,
                        format!("trait {trait_name} is compiler-owned and cannot be declared"),
                    ));
                }
                if Operation::from_name(&trait_name).is_some()
                    || is_contextual_name(&trait_name)
                    || is_builtin_type_name(&trait_name)
                {
                    return Err(self.error(source, format!("trait declaration {trait_name} collides with a reserved operation, form, or type")));
                }
                if self.trait_names.contains_key(&trait_name) {
                    return Err(
                        self.error(source, format!("duplicate trait declaration {trait_name}"))
                    );
                }
                let id =
                    TraitId::new(u32::try_from(self.traits.len()).map_err(|_| {
                        self.error(source, "too many trait declarations for TraitId")
                    })?);
                self.trait_names.insert(trait_name.clone(), id);
                self.traits.push(TraitDefinition {
                    id,
                    name: trait_name,
                    origin: Origin::Source(source),
                    core: None,
                });
            }
        }
        Ok(())
    }

    fn collect_product_names(&mut self, program: &AstProgram) -> Result<()> {
        for (source_index, file) in program.files.iter().enumerate() {
            let source_raw = u32::try_from(source_index)
                .map_err(|_| Error::msg("too many source files for HIR SourceId"))?;
            let source = SourceId::new(source_raw);
            for form in &file.forms {
                let AstExpr::Call { name, args } = form else {
                    continue;
                };
                if name != "product" {
                    continue;
                }
                let (product_name, _) =
                    product_declaration(args).map_err(|message| self.error(source, message))?;
                if !is_declaration_type_name(&product_name) {
                    return Err(self.error(
                        source,
                        format!("invalid product declaration name {product_name}"),
                    ));
                }
                if Operation::from_name(&product_name).is_some()
                    || is_contextual_name(&product_name)
                    || is_builtin_type_name(&product_name)
                {
                    return Err(self.error(
                        source,
                        format!("product declaration {product_name} collides with a reserved operation, form, or type"),
                    ));
                }
                if self.product_names.contains_key(&product_name) {
                    return Err(self.error(
                        source,
                        format!("duplicate product declaration {product_name}"),
                    ));
                }
                if self.trait_names.contains_key(&product_name) {
                    return Err(self.error(
                        source,
                        format!(
                            "product declaration {product_name} collides with a trait declaration"
                        ),
                    ));
                }
                let raw = u16::try_from(self.product_names.len()).map_err(|_| {
                    self.error(source, "too many product declarations for ProductId")
                })?;
                self.product_names.insert(product_name, ProductId::new(raw));
            }
        }
        Ok(())
    }

    fn collect_products(&mut self, program: &AstProgram) -> Result<()> {
        for (source_index, file) in program.files.iter().enumerate() {
            let source_raw = u32::try_from(source_index)
                .map_err(|_| Error::msg("too many source files for HIR SourceId"))?;
            let source = SourceId::new(source_raw);
            for form in &file.forms {
                let AstExpr::Call { name, args } = form else {
                    continue;
                };
                if name != "product" {
                    continue;
                }
                let (product_name, field_forms) =
                    product_declaration(args).map_err(|message| self.error(source, message))?;
                if field_forms.len() > MAX_PRODUCT_FIELDS {
                    return Err(self.error(
                        source,
                        format!(
                            "product {product_name}: too many fields ({} > {MAX_PRODUCT_FIELDS})",
                            field_forms.len()
                        ),
                    ));
                }
                let product = self
                    .product_names
                    .get(&product_name)
                    .copied()
                    .ok_or_else(|| {
                        self.error(
                            source,
                            format!("unknown product declaration {product_name}"),
                        )
                    })?;
                let mut names = HashSet::new();
                let mut fields = Vec::with_capacity(field_forms.len());
                for field_form in field_forms {
                    let (field_name, ty) = parse_product_field(field_form).map_err(|message| {
                        self.error(source, format!("product {product_name}: {message}"))
                    })?;
                    if !names.insert(field_name.clone()) {
                        return Err(self.error(
                            source,
                            format!("product {product_name}: duplicate field {field_name}"),
                        ));
                    }
                    self.validate_product_type(&ty).map_err(|message| {
                        self.error(
                            source,
                            format!("product {product_name} field {field_name}: {message}"),
                        )
                    })?;
                    if contains_ownership_type(&ty) {
                        return Err(self.error(
                            source,
                            format!("product {product_name} field {field_name}: ownership/reference types cannot be stored in products"),
                        ));
                    }
                    let mut free = HashSet::new();
                    collect_type_params(&ty, &mut free);
                    if let Some(parameter) = free.into_iter().next() {
                        return Err(self.error(
                            source,
                            format!("product {product_name} field {field_name}: type contains unbound parameter {parameter}"),
                        ));
                    }
                    fields.push(ProductField {
                        name: field_name,
                        ty,
                    });
                }
                if product.index() != self.products.len() {
                    return Err(self.error(source, "product declaration order is inconsistent"));
                }
                self.products.push(ProductDefinition {
                    id: product,
                    name: product_name,
                    origin: source,
                    fields,
                });
            }
        }
        Ok(())
    }

    fn collect_implementations(&mut self, program: &AstProgram) -> Result<()> {
        let mut coherent = HashSet::new();
        for (source_index, file) in program.files.iter().enumerate() {
            let source = SourceId::new(
                u32::try_from(source_index)
                    .map_err(|_| Error::msg("too many source files for HIR SourceId"))?,
            );
            for form in &file.forms {
                let AstExpr::Call { name, args } = form else {
                    continue;
                };
                if name != "impl" {
                    continue;
                }
                let (trait_name, target) =
                    impl_declaration(args).map_err(|message| self.error(source, message))?;
                let trait_id = self.trait_names.get(&trait_name).copied().ok_or_else(|| {
                    self.error(
                        source,
                        format!("impl references unknown trait {trait_name}"),
                    )
                })?;
                let trait_definition = self
                    .traits
                    .get(trait_id.index().unwrap_or(usize::MAX))
                    .ok_or_else(|| self.error(source, "impl resolved an unknown TraitId"))?;
                if trait_definition.core.is_some() {
                    return Err(self.error(source, format!("core trait {trait_name} cannot be explicitly implemented in the marker-trait slice")));
                }
                let Type::Product(product_name) = target else {
                    return Err(self.error(
                        source,
                        "marker impl target must be one exact nominal Product type",
                    ));
                };
                let product = self
                    .product_names
                    .get(&product_name)
                    .copied()
                    .ok_or_else(|| {
                        self.error(
                            source,
                            format!("impl references unknown product {product_name}"),
                        )
                    })?;
                if !coherent.insert((trait_id, product)) {
                    return Err(self.error(source, format!("overlapping marker impl for trait {trait_name} and product {product_name} in the current program closure")));
                }
                let id = ImplId::new(
                    u32::try_from(self.implementations.len())
                        .map_err(|_| self.error(source, "too many implementations for ImplId"))?,
                );
                self.implementations.push(ImplDefinition {
                    id,
                    trait_id,
                    product,
                    origin: source,
                });
                self.implementation_index.insert((trait_id, product), id);
            }
        }
        Ok(())
    }

    fn validate_product_type(&self, ty: &Type) -> std::result::Result<(), String> {
        match ty {
            Type::Product(name) => {
                if self.product_names.contains_key(name) {
                    Ok(())
                } else {
                    Err(format!("unknown product type {name}"))
                }
            }
            Type::Owned(inner) | Type::Ref(inner) | Type::RefMut(inner) => {
                if inner.as_ref() == &Type::Buf {
                    Ok(())
                } else {
                    Err("ownership types accept only exact Buf in this slice".into())
                }
            }
            Type::List(inner) | Type::Option(inner) => {
                if contains_ownership_type(inner) {
                    return Err(
                        "ownership/reference types cannot be stored in List or Option".into(),
                    );
                }
                self.validate_product_type(inner)
            }
            Type::Result(ok, error) => {
                if contains_ownership_type(ok) || contains_ownership_type(error) {
                    return Err("ownership/reference types cannot be stored in Result".into());
                }
                self.validate_product_type(ok)?;
                self.validate_product_type(error)
            }
            Type::Fn { params, ret } => {
                for parameter in params {
                    self.validate_product_type(parameter)?;
                }
                self.validate_product_type(ret)
            }
            Type::Forall { body, .. } => self.validate_product_type(body),
            _ => Ok(()),
        }
    }

    fn product_by_name(&self, name: &str) -> Result<&ProductDefinition> {
        let id = self
            .product_names
            .get(name)
            .copied()
            .ok_or_else(|| Error::msg(format!("unknown product type {name}")))?;
        self.products
            .get(id.index())
            .filter(|product| product.id == id)
            .ok_or_else(|| Error::msg(format!("missing HIR product metadata for {name}")))
    }

    fn collect_headers<'a>(
        &mut self,
        program: &'a AstProgram,
    ) -> Result<(Vec<PendingFunction<'a>>, PendingMain<'a>)> {
        let mut functions = Vec::new();
        let mut main = None;
        for (source_index, file) in program.files.iter().enumerate() {
            let source_raw = u32::try_from(source_index)
                .map_err(|_| Error::msg("too many source files for HIR SourceId"))?;
            let source = SourceId::new(source_raw);
            let is_root = file.path == program.root;
            for form in &file.forms {
                match form {
                    AstExpr::Call { name, .. }
                        if matches!(name.as_str(), "import" | "product" | "trait" | "impl") => {}
                    AstExpr::Call { name, args } if name == "def" => {
                        functions.push(self.collect_definition(source, args)?);
                    }
                    AstExpr::Call { name, args } if name == "main" => {
                        if !is_root {
                            return Err(self.error(source, "imported file may not declare main"));
                        }
                        if main.is_some() {
                            return Err(
                                self.error(source, "executable root declares duplicate main")
                            );
                        }
                        let (return_type, body) = parse_main(args)
                            .map_err(|message| self.error(source, format!("main: {message}")))?;
                        self.validate_product_type(&return_type)
                            .map_err(|message| self.error(source, format!("main: {message}")))?;
                        if matches!(return_type, Type::Ref(_) | Type::RefMut(_)) {
                            return Err(self.error(
                                source,
                                "main cannot return a lexical reference in the initial ownership slice",
                            ));
                        }
                        let mut free = HashSet::new();
                        collect_type_params(&return_type, &mut free);
                        if let Some(parameter) = free.into_iter().next() {
                            return Err(self.error(
                                source,
                                format!("main: return type contains unbound parameter {parameter}"),
                            ));
                        }
                        main = Some(PendingMain {
                            origin: source,
                            return_type,
                            body,
                        });
                    }
                    AstExpr::Call { name, .. } if name == "do" => {
                        return Err(self.error(source, "top-level do was removed; use root main"));
                    }
                    other => {
                        return Err(
                            self.error(source, format!("unsupported top-level form: {other:?}"))
                        );
                    }
                }
            }
        }
        let main =
            main.ok_or_else(|| Error::msg("executable root must declare exactly one main"))?;
        Ok((functions, main))
    }

    fn collect_definition<'a>(
        &mut self,
        origin: SourceId,
        args: &'a [AstExpr],
    ) -> Result<PendingFunction<'a>> {
        let name = definition_name(args).map_err(|message| self.error(origin, message))?;
        let [_, AstExpr::Call {
            name: tag,
            args: fn_args,
        }] = args
        else {
            return Err(self.error(
                origin,
                format!("def {name}: top-level def must declare an immutable fn"),
            ));
        };
        if tag != "fn" {
            return Err(self.error(
                origin,
                format!("def {name}: top-level def must declare an immutable fn"),
            ));
        }
        let parsed = parse_function(fn_args)
            .map_err(|message| self.error(origin, format!("def {name}: {message}")))?;
        validate_function_header(&name, &parsed).map_err(|message| self.error(origin, message))?;
        for ty in parsed
            .signature_params
            .iter()
            .chain(parsed.param_types.iter())
            .chain(std::iter::once(&parsed.signature_return))
        {
            self.validate_product_type(ty)
                .map_err(|message| self.error(origin, format!("def {name}: {message}")))?;
        }
        if matches!(parsed.signature_return, Type::Ref(_) | Type::RefMut(_)) {
            return Err(self.error(
                origin,
                format!("def {name}: lexical references cannot be returned in the initial ownership slice"),
            ));
        }
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
        let mut bounds = Vec::with_capacity(parsed.bounds.len());
        let mut seen = HashSet::new();
        for bound in &parsed.bounds {
            if !parsed
                .forall_vars
                .iter()
                .any(|variable| variable == &bound.parameter)
            {
                return Err(self.error(
                    origin,
                    format!(
                        "bound parameter {} is not declared by forall",
                        bound.parameter
                    ),
                ));
            }
            let trait_id = self
                .trait_names
                .get(&bound.trait_name)
                .copied()
                .ok_or_else(|| {
                    self.error(
                        origin,
                        format!("bound references unknown trait {}", bound.trait_name),
                    )
                })?;
            let trait_definition = self
                .traits
                .get(trait_id.index().unwrap_or(usize::MAX))
                .ok_or_else(|| self.error(origin, "bound resolved an unknown TraitId"))?;
            if matches!(
                trait_definition.core,
                Some(CoreTrait::Clone | CoreTrait::Drop)
            ) {
                return Err(self.error(
                    origin,
                    format!(
                        "core trait {} requires methods and is unavailable in the marker-trait slice",
                        trait_definition.name
                    ),
                ));
            }
            if !seen.insert((bound.parameter.as_str(), trait_id)) {
                return Err(self.error(
                    origin,
                    format!("duplicate bound {} {}", bound.parameter, bound.trait_name),
                ));
            }
            bounds.push(TraitBound {
                parameter: bound.parameter.clone(),
                trait_id,
            });
        }
        self.function_bounds.insert(binding, bounds.clone());
        Ok(PendingFunction {
            binding,
            origin,
            parsed,
            bounds,
        })
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
        if self.globals.contains_key(&name)
            || self.product_names.contains_key(&name)
            || self.trait_names.contains_key(&name)
        {
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

    fn resolve_main(&mut self, pending: PendingMain<'_>) -> Result<Main> {
        let (body, local_count) = {
            let mut resolver = Resolver::new(
                self,
                pending.origin,
                HashMap::new(),
                HashMap::new(),
                HashSet::new(),
                0,
            );
            let body = resolver.resolve_expr(pending.body)?;
            let local_count = resolver.local_count()?;
            (body, local_count)
        };
        if body.ty != pending.return_type {
            return Err(self.error(
                pending.origin,
                format!(
                    "main body type {:?} does not exactly equal declared return {:?}",
                    body.ty, pending.return_type
                ),
            ));
        }
        Ok(Main {
            origin: pending.origin,
            return_type: pending.return_type,
            local_count,
            body,
        })
    }

    fn resolve_function(
        &mut self,
        binding: BindingId,
        origin: SourceId,
        parsed: ParsedFunction<'_>,
        bounds: Vec<TraitBound>,
    ) -> Result<Function> {
        let arity = u8::try_from(parsed.param_names.len()).map_err(|_| {
            self.error(
                origin,
                "function has too many parameters for bytecode arity",
            )
        })?;
        let mut params = Vec::with_capacity(parsed.param_names.len());
        let mut scope = HashMap::new();
        let mut local_slots = HashMap::new();
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
            local_slots.insert(
                id,
                u8::try_from(index).map_err(|_| {
                    self.error(origin, "function has too many parameter local slots")
                })?,
            );
            params.push(id);
        }

        let (body, local_count, param_places) = {
            let type_variables = parsed.forall_vars.iter().cloned().collect();
            let mut resolver = Resolver::new(
                self,
                origin,
                scope,
                local_slots,
                type_variables,
                params.len(),
            );
            let param_places = params
                .iter()
                .map(|parameter| resolver.place(*parameter))
                .collect::<Result<Vec<_>>>()?;
            let body = resolver.resolve_expr(parsed.body)?;
            let local_count = resolver.local_count()?;
            (body, local_count, param_places)
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
            param_places,
            bounds,
            arity,
            local_count,
            summary: EffectSet::PURE,
            body,
        })
    }

    fn build_global_layout(&self, functions: &[Function]) -> Result<Vec<BindingId>> {
        let mut layout = Vec::with_capacity(functions.len());
        let mut seen = HashSet::new();
        for function in functions {
            record_global(function.binding, &mut layout, &mut seen)?;
        }
        Ok(layout)
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
    local_slots: HashMap<BindingId, u8>,
    local_places: HashMap<BindingId, PlaceId>,
    type_variables: HashSet<String>,
    next_slot: usize,
    max_slots: usize,
    next_place: u32,
}

impl<'a> Resolver<'a> {
    fn new(
        analyzer: &'a mut Analyzer,
        origin: SourceId,
        function_scope: HashMap<String, BindingId>,
        local_slots: HashMap<BindingId, u8>,
        type_variables: HashSet<String>,
        parameter_count: usize,
    ) -> Self {
        let local_places = local_slots
            .iter()
            .map(|(binding, slot)| (*binding, PlaceId::new(u32::from(*slot))))
            .collect();
        Self {
            analyzer,
            origin,
            scopes: vec![function_scope],
            local_slots,
            local_places,
            type_variables,
            next_slot: parameter_count,
            max_slots: parameter_count,
            next_place: u32::try_from(parameter_count).unwrap_or(u32::MAX),
        }
    }

    fn place(&self, binding: BindingId) -> Result<PlaceId> {
        self.local_places.get(&binding).copied().ok_or_else(|| {
            self.error(format!(
                "binding {} has no whole-local PlaceId",
                binding.raw()
            ))
        })
    }

    fn allocate_place(&mut self, binding: BindingId) -> Result<PlaceId> {
        let place = PlaceId::new(self.next_place);
        self.next_place = self
            .next_place
            .checked_add(1)
            .ok_or_else(|| self.error("too many ownership places"))?;
        self.local_places.insert(binding, place);
        Ok(place)
    }

    fn allocate_loan(&mut self) -> Result<LoanId> {
        let loan = LoanId::new(self.analyzer.next_loan);
        self.analyzer.next_loan = self
            .analyzer
            .next_loan
            .checked_add(1)
            .ok_or_else(|| self.error("too many ownership loans in program closure"))?;
        Ok(loan)
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
        if resolved.kind == BindingKind::Function
            && self
                .analyzer
                .function_bounds
                .get(&binding)
                .is_some_and(|bounds| !bounds.is_empty())
        {
            return Err(self.error(format!(
                "bounded generic function {name} is not a first-class value in the marker slice; call it directly"
            )));
        }
        let ty = resolved.ty.clone();
        let binding = self.binding_ref(binding)?;
        Ok(self.expression(ty, ExprKind::Load(binding)))
    }

    fn resolve_call(&mut self, name: &str, args: &[AstExpr]) -> Result<Expr> {
        match name {
            "if" => self.resolve_if(args),
            "while" => self.resolve_while(args),
            "do" => self.resolve_do(args),
            "let" => self.resolve_let(args),
            "var" => self.resolve_var(args),
            "quote" => self.resolve_quote(args),
            "set" => self.resolve_set(args),
            "move" => self.resolve_move(args),
            "borrow" => self.resolve_borrow(args, BorrowKind::Shared),
            "borrow-mut" => self.resolve_borrow(args, BorrowKind::Mutable),
            "empty-list" => self.resolve_empty_list(args),
            "none" => self.resolve_none(args),
            "product-value" => self.resolve_product_value(args),
            "field" => self.resolve_product_field(args),
            "with-field" => self.resolve_with_product_field(args),
            "bind" => Err(self.error("bind is only valid inside let")),
            "fn" | "def" | "main" | "sig" | "params" | "forall" | "bounds" | "bound" | "type"
            | "import" | "name" | "product" | "fields" | "trait" | "impl" | "for" => {
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
            if resolved_args
                .iter()
                .any(|argument| matches!(argument.ty, Type::RefMut(_)))
            {
                return Err(
                    self.error("RefMut forwarding is unsupported in the initial ownership slice")
                );
            }
            let (ty, instantiation) =
                self.call_result(name, callee, callee_type, &resolved_args)?;
            let callee = self.binding_ref(callee)?;
            Ok(self.expression(
                ty,
                ExprKind::Call {
                    callee,
                    args: resolved_args,
                    instantiation,
                },
            ))
        }
    }

    fn call_result(
        &self,
        name: &str,
        callee: BindingId,
        callable: Type,
        args: &[Expr],
    ) -> Result<(Type, Option<GenericInstantiation>)> {
        let is_generic = matches!(&callable, Type::Forall { .. });
        let generic_signature_has_ownership = is_generic && contains_ownership_type(&callable);
        let (instantiated, substitutions) = self.instantiate(name, callable, args)?;
        if is_generic
            && (generic_signature_has_ownership
                || substitutions
                    .iter()
                    .any(|substitution| contains_ownership_type(&substitution.ty)))
        {
            return Err(self.error(format!(
                "{name}: ownership/reference generic instantiation is unavailable in the initial ownership slice"
            )));
        }
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
        if contains_reference_type(&ret) {
            return Err(self.error(format!(
                "{name}: user-call results cannot be lexical references in the initial ownership slice"
            )));
        }
        let instantiation = if substitutions.is_empty() {
            None
        } else {
            let bounds = self
                .analyzer
                .function_bounds
                .get(&callee)
                .cloned()
                .unwrap_or_default();
            if !bounds.is_empty() {
                for substitution in &substitutions {
                    let mut unresolved = HashSet::new();
                    collect_type_params(&substitution.ty, &mut unresolved);
                    if !unresolved.is_empty() {
                        return Err(self.error(format!(
                            "{name}: forwarding bounded calls from a generic context is unavailable in the marker-trait slice"
                        )));
                    }
                }
            }
            let mut witnesses = Vec::with_capacity(bounds.len());
            for bound in bounds {
                let ty = substitutions
                    .iter()
                    .find(|substitution| substitution.parameter == bound.parameter)
                    .map(|substitution| substitution.ty.clone())
                    .ok_or_else(|| {
                        self.error(format!(
                            "{name}: missing substitution for bound parameter {}",
                            bound.parameter
                        ))
                    })?;
                witnesses.push(self.solve_trait_bound(name, bound.trait_id, &ty)?);
            }
            Some(GenericInstantiation {
                substitutions,
                witnesses,
            })
        };
        Ok((*ret, instantiation))
    }

    fn instantiate(
        &self,
        name: &str,
        callable: Type,
        args: &[Expr],
    ) -> Result<(Type, Vec<TypeSubstitution>)> {
        let Type::Forall { vars, body } = callable else {
            return Ok((callable, Vec::new()));
        };
        let Type::Fn { params, ret } = *body else {
            return Err(self.error("forall body must be a function type"));
        };
        if params.len() != args.len() {
            return Ok((Type::Fn { params, ret }, Vec::new()));
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
        let canonical = vars
            .iter()
            .map(|parameter| TypeSubstitution {
                parameter: parameter.clone(),
                ty: substitutions
                    .get(parameter)
                    .cloned()
                    .unwrap_or(Type::Param(parameter.clone())),
            })
            .collect();
        Ok((
            Type::Fn {
                params: params
                    .iter()
                    .map(|parameter| parameter.subst(&substitutions))
                    .collect(),
                ret: Box::new(ret.subst(&substitutions)),
            },
            canonical,
        ))
    }

    fn solve_trait_bound(
        &self,
        function: &str,
        trait_id: TraitId,
        ty: &Type,
    ) -> Result<TraitWitness> {
        let definition = self
            .analyzer
            .traits
            .get(trait_id.index().unwrap_or(usize::MAX))
            .filter(|definition| definition.id == trait_id)
            .ok_or_else(|| {
                self.error(format!(
                    "{function}: bound references unknown TraitId {}",
                    trait_id.raw()
                ))
            })?;
        let kind = if let Some(core_trait) = definition.core.filter(|role| role.is_auto()) {
            let mut work = 0;
            let mut active = HashSet::new();
            let mut memo = HashMap::new();
            match self.auto_trait_holds(core_trait, ty, 0, &mut work, &mut active, &mut memo)? {
                true => TraitWitnessKind::AutoTrait,
                false => {
                    return Err(self.error(format!(
                        "{function}: type {ty:?} does not satisfy trait {}",
                        definition.name
                    )))
                }
            }
        } else {
            let Type::Product(name) = ty else {
                return Err(self.error(format!(
                    "{function}: type {ty:?} has no exact implementation of trait {}",
                    definition.name
                )));
            };
            let product = self
                .analyzer
                .product_names
                .get(name)
                .copied()
                .ok_or_else(|| self.error(format!("{function}: unknown product type {name}")))?;
            let implementation = self
                .analyzer
                .implementation_index
                .get(&(trait_id, product))
                .copied()
                .ok_or_else(|| {
                    self.error(format!(
                        "{function}: product {name} does not implement trait {}",
                        definition.name
                    ))
                })?;
            TraitWitnessKind::Explicit(implementation)
        };
        Ok(TraitWitness {
            trait_id,
            ty: ty.clone(),
            kind,
        })
    }

    fn auto_trait_holds(
        &self,
        core_trait: CoreTrait,
        ty: &Type,
        depth: usize,
        work: &mut usize,
        active: &mut HashSet<ProductId>,
        memo: &mut HashMap<Type, bool>,
    ) -> Result<bool> {
        if let Some(result) = memo.get(ty) {
            return Ok(*result);
        }
        if depth > TRAIT_SOLVER_MAX_DEPTH {
            return Err(self.error(format!(
                "trait solver depth exceeded {TRAIT_SOLVER_MAX_DEPTH}"
            )));
        }
        *work = work
            .checked_add(1)
            .ok_or_else(|| self.error("trait solver work overflow"))?;
        if *work > TRAIT_SOLVER_MAX_WORK {
            return Err(self.error(format!(
                "trait solver work exceeded {TRAIT_SOLVER_MAX_WORK}"
            )));
        }
        let result = match core_trait {
            CoreTrait::Copy => match ty {
                Type::Unit | Type::Bool | Type::I64 | Type::F64 | Type::Str | Type::Symbol => true,
                Type::Ref(inner) if inner.as_ref() == &Type::Buf => true,
                Type::Buf
                | Type::Owned(_)
                | Type::Ref(_)
                | Type::RefMut(_)
                | Type::Handle
                | Type::Fn { .. }
                | Type::Forall { .. }
                | Type::Param(_) => false,
                Type::List(inner) | Type::Option(inner) => {
                    self.auto_trait_holds(core_trait, inner, depth + 1, work, active, memo)?
                }
                Type::Result(ok, error) => {
                    self.auto_trait_holds(core_trait, ok, depth + 1, work, active, memo)?
                        && self.auto_trait_holds(
                            core_trait,
                            error,
                            depth + 1,
                            work,
                            active,
                            memo,
                        )?
                }
                Type::Product(name) => {
                    let product = self.analyzer.product_by_name(name)?;
                    if !active.insert(product.id) {
                        return Err(self.error(format!(
                            "trait solver encountered recursive product cycle at {name}"
                        )));
                    }
                    let mut result = true;
                    for field in &product.fields {
                        if !self.auto_trait_holds(
                            core_trait,
                            &field.ty,
                            depth + 1,
                            work,
                            active,
                            memo,
                        )? {
                            result = false;
                            break;
                        }
                    }
                    active.remove(&product.id);
                    result
                }
            },
            CoreTrait::Send | CoreTrait::Sync => {
                matches!(ty, Type::Unit | Type::Bool | Type::I64 | Type::F64)
            }
            CoreTrait::Clone | CoreTrait::Drop => false,
        };
        memo.insert(ty.clone(), result);
        Ok(result)
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
            (Type::Owned(pattern), Type::Owned(got))
            | (Type::Ref(pattern), Type::Ref(got))
            | (Type::RefMut(pattern), Type::RefMut(got))
            | (Type::List(pattern), Type::List(got))
            | (Type::Option(pattern), Type::Option(got)) => {
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
            self.local_slots.insert(binding_id, slot);
            let place = self.allocate_place(binding_id)?;
            resolved_bindings.push(LocalDefinition {
                binding: binding_id,
                place,
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

    fn resolve_var(&mut self, args: &[AstExpr]) -> Result<Expr> {
        let [name_form, type_form, initial_ast, body_ast] = args else {
            return Err(
                self.error("var expects name/, type/, initial expression, and body expression")
            );
        };
        let name = declared_name_form(name_form, "var").map_err(|message| self.error(message))?;
        let AstExpr::Call {
            name: type_tag,
            args: type_args,
        } = type_form
        else {
            return Err(self.error("var expects type/…/type second"));
        };
        if type_tag != "type" {
            return Err(self.error("var expects type/…/type second"));
        }
        let declared_type = parse_type_form(type_args)
            .map_err(|message| self.error(format!("var {name}: {message}")))?;
        if matches!(declared_type, Type::Ref(_) | Type::RefMut(_)) {
            return Err(self.error(format!(
                "var {name}: lexical references may only be inferred let bindings or parameters"
            )));
        }
        self.analyzer
            .validate_product_type(&declared_type)
            .map_err(|message| self.error(format!("var {name}: {message}")))?;
        let mut parameters = HashSet::new();
        collect_type_params(&declared_type, &mut parameters);
        if let Some(parameter) = parameters
            .into_iter()
            .find(|parameter| !self.type_variables.contains(*parameter))
        {
            return Err(self.error(format!(
                "var {name}: type parameter {parameter} is not declared by forall"
            )));
        }

        // The initializer is deliberately resolved before the new binding exists.
        let initial = self.resolve_expr(initial_ast)?;
        if initial.ty != declared_type {
            return Err(self.error(format!(
                "var {name}: initializer type {:?} does not exactly equal {declared_type:?}",
                initial.ty
            )));
        }

        let saved_slot = self.next_slot;
        let slot = u8::try_from(self.next_slot)
            .map_err(|_| self.error("var needs more than 255 bytecode local slots"))?;
        self.next_slot = self
            .next_slot
            .checked_add(1)
            .ok_or_else(|| self.error("local slot count overflow"))?;
        self.max_slots = self.max_slots.max(self.next_slot);
        let binding = self.analyzer.add_binding(
            name.clone(),
            BindingKind::MutableLocal,
            declared_type,
            Origin::Source(self.origin),
        )?;
        self.local_slots.insert(binding, slot);
        let place = self.allocate_place(binding)?;
        self.scopes.push(HashMap::from([(name, binding)]));
        let body = self.resolve_expr(body_ast)?;
        let _removed_scope = self.scopes.pop();
        self.next_slot = saved_slot;
        let ty = body.ty.clone();
        Ok(self.expression(
            ty,
            ExprKind::MutableLocal {
                binding,
                place,
                slot,
                initial: Box::new(initial),
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
        let target = self.lookup_lexical(&target_name).ok_or_else(|| {
            if self.analyzer.globals.contains_key(&target_name)
                || Operation::from_name(&target_name).is_some()
            {
                self.error(format!(
                    "set target {target_name} is not a function-local mutable var"
                ))
            } else {
                self.error(format!("unknown set target {target_name}"))
            }
        })?;
        let (kind, target_type) = {
            let binding = self.analyzer.binding(target)?;
            (binding.kind.clone(), binding.ty.clone())
        };
        if kind != BindingKind::MutableLocal {
            return Err(self.error(format!(
                "set target {target_name} is not a function-local mutable var"
            )));
        }
        let slot =
            self.local_slots.get(&target).copied().ok_or_else(|| {
                self.error(format!("set target {target_name} has no HIR local slot"))
            })?;
        let value = self.resolve_expr(value_ast)?;
        if value.ty != target_type {
            return Err(self.error(format!(
                "set target {target_name}: value type {:?} does not exactly equal {target_type:?}",
                value.ty
            )));
        }
        Ok(self.expression(
            Type::Unit,
            ExprKind::SetLocal {
                target,
                slot,
                value: Box::new(value),
            },
        ))
    }

    fn resolve_move(&mut self, args: &[AstExpr]) -> Result<Expr> {
        let [AstExpr::Symbol(name)] = args else {
            return Err(self.error("move expects exactly one whole local or parameter name"));
        };
        let binding = self.lookup_lexical(name).ok_or_else(|| {
            self.error(format!(
                "move target {name} is not a whole local or parameter"
            ))
        })?;
        let ty = self.analyzer.binding(binding)?.ty.clone();
        match ty {
            Type::Owned(ref inner) if inner.as_ref() == &Type::Buf => {}
            Type::RefMut(_) => {
                return Err(
                    self.error("RefMut forwarding is unsupported in the initial ownership slice")
                );
            }
            _ => return Err(self.error("move requires an affine Owned Buf place")),
        }
        let place = self.place(binding)?;
        let binding = self.binding_ref(binding)?;
        Ok(self.expression(ty, ExprKind::Move { place, binding }))
    }

    fn resolve_borrow(&mut self, args: &[AstExpr], kind: BorrowKind) -> Result<Expr> {
        let [AstExpr::Symbol(name)] = args else {
            return Err(
                self.error("borrow expects exactly one whole Owned Buf local or parameter name")
            );
        };
        let binding = self.lookup_lexical(name).ok_or_else(|| {
            self.error(format!(
                "borrow target {name} is not a whole local or parameter"
            ))
        })?;
        let owner_ty = self.analyzer.binding(binding)?.ty.clone();
        if owner_ty != Type::Owned(Box::new(Type::Buf)) {
            return Err(self.error(
                "borrow target must have exact type Owned Buf; reborrow and legacy Buf are unsupported",
            ));
        }
        let place = self.place(binding)?;
        let loan = self.allocate_loan()?;
        let binding = self.binding_ref(binding)?;
        let ty = match kind {
            BorrowKind::Shared => Type::Ref(Box::new(Type::Buf)),
            BorrowKind::Mutable => Type::RefMut(Box::new(Type::Buf)),
        };
        Ok(self.expression(
            ty,
            ExprKind::Borrow {
                place,
                loan,
                kind,
                binding,
            },
        ))
    }

    fn resolve_product_value(&mut self, args: &[AstExpr]) -> Result<Expr> {
        let Some((name_expression, field_forms)) = args.split_first() else {
            return Err(self.error("product-value expects a product name"));
        };
        let product_name = symbolic_name(name_expression)
            .map_err(|_| self.error("product-value name must be a symbol"))?;
        let product = self
            .analyzer
            .product_by_name(&product_name)
            .map_err(|_| self.error(format!("unknown product type {product_name}")))?
            .clone();
        if field_forms.len() != product.fields.len() {
            return Err(self.error(format!(
                "product-value {product_name}: expected {} fields, got {}",
                product.fields.len(),
                field_forms.len()
            )));
        }
        let mut fields = Vec::with_capacity(field_forms.len());
        for (index, (field_form, declared)) in field_forms.iter().zip(&product.fields).enumerate() {
            let AstExpr::Call { name, args } = field_form else {
                return Err(self.error(format!(
                    "product-value {product_name}: field {} must be field/…/field",
                    declared.name
                )));
            };
            if name != "field" {
                return Err(self.error(format!(
                    "product-value {product_name}: field {} must be field/…/field",
                    declared.name
                )));
            }
            let [name_expression, value_expression] = args.as_slice() else {
                return Err(self.error(format!(
                    "product-value {product_name}: constructor field expects name and value"
                )));
            };
            let field_name = symbolic_name(name_expression)
                .map_err(|_| self.error("constructor field name must be a symbol"))?;
            if field_name != declared.name {
                return Err(self.error(format!(
                    "product-value {product_name}: field {} must be {} in declaration order, got {field_name}",
                    index + 1,
                    declared.name
                )));
            }
            let value = self.resolve_expr(value_expression)?;
            if !Type::unify_assignable(&value.ty, &declared.ty) {
                return Err(self.error(format!(
                    "product-value {product_name} field {field_name}: value type {:?} not assignable to {:?}",
                    value.ty, declared.ty
                )));
            }
            fields.push(value);
        }
        Ok(self.expression(
            Type::Product(product.name),
            ExprKind::ProductValue {
                product: product.id,
                fields,
            },
        ))
    }

    fn resolve_product_field(&mut self, args: &[AstExpr]) -> Result<Expr> {
        let [value_expression, name_expression] = args else {
            return Err(self.error("field expects a product value and field name"));
        };
        let value = self.resolve_expr(value_expression)?;
        let Type::Product(product_name) = &value.ty else {
            return Err(self.error("field value must have a concrete Product type"));
        };
        let field_name = symbolic_name(name_expression)
            .map_err(|_| self.error("field name must be a symbol"))?;
        let product = self
            .analyzer
            .product_by_name(product_name)
            .map_err(|_| self.error(format!("unknown product type {product_name}")))?
            .clone();
        let (field_index, field) = product
            .fields
            .iter()
            .enumerate()
            .find(|(_, field)| field.name == field_name)
            .ok_or_else(|| {
                self.error(format!(
                    "product {} has no field {field_name}",
                    product.name
                ))
            })?;
        let field_index = u8::try_from(field_index)
            .map_err(|_| self.error("product field index does not fit u8"))?;
        Ok(self.expression(
            field.ty.clone(),
            ExprKind::ProductField {
                product: product.id,
                field: field_index,
                value: Box::new(value),
            },
        ))
    }

    fn resolve_with_product_field(&mut self, args: &[AstExpr]) -> Result<Expr> {
        let [value_expression, name_expression, replacement_expression] = args else {
            return Err(
                self.error("with-field expects a product value, field name, and replacement")
            );
        };
        let value = self.resolve_expr(value_expression)?;
        let Type::Product(product_name) = &value.ty else {
            return Err(self.error("with-field value must have a concrete Product type"));
        };
        let product_name = product_name.clone();
        let field_name = symbolic_name(name_expression)
            .map_err(|_| self.error("with-field name must be a symbol"))?;
        let product = self
            .analyzer
            .product_by_name(&product_name)
            .map_err(|_| self.error(format!("unknown product type {product_name}")))?
            .clone();
        let (field_index, field) = product
            .fields
            .iter()
            .enumerate()
            .find(|(_, field)| field.name == field_name)
            .ok_or_else(|| {
                self.error(format!(
                    "product {} has no field {field_name}",
                    product.name
                ))
            })?;
        let field_index = u8::try_from(field_index)
            .map_err(|_| self.error("product field index does not fit u8"))?;
        let replacement = self.resolve_expr(replacement_expression)?;
        if !Type::unify_assignable(&replacement.ty, &field.ty) {
            return Err(self.error(format!(
                "with-field {}.{field_name}: replacement type {:?} not assignable to {:?}",
                product.name, replacement.ty, field.ty
            )));
        }
        Ok(self.expression(
            Type::Product(product.name),
            ExprKind::WithProductField {
                product: product.id,
                field: field_index,
                value: Box::new(value),
                replacement: Box::new(replacement),
            },
        ))
    }

    fn resolve_empty_list(&mut self, args: &[AstExpr]) -> Result<Expr> {
        let element = parse_type_form(args)
            .map_err(|message| self.error(format!("empty-list: {message}")))?;
        self.analyzer
            .validate_product_type(&element)
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
        self.analyzer
            .validate_product_type(&value_type)
            .map_err(|message| self.error(format!("none: {message}")))?;
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

    fn binding_ref(&self, binding: BindingId) -> Result<BindingRef> {
        let storage = match self.analyzer.binding(binding)?.kind {
            BindingKind::Parameter | BindingKind::ImmutableLocal | BindingKind::MutableLocal => {
                BindingStorage::Local(self.local_slots.get(&binding).copied().ok_or_else(|| {
                    self.error(format!("binding {} has no HIR local slot", binding.raw()))
                })?)
            }
            BindingKind::Function => BindingStorage::Function,
            BindingKind::BuiltinOperation(_) => {
                return Err(self.error("built-in operation cannot be loaded as a binding"));
            }
        };
        Ok(BindingRef { binding, storage })
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
            ExprKind::Load(_) | ExprKind::Move { .. } | ExprKind::Borrow { .. } => EffectSet::PURE,
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
            ExprKind::MutableLocal { initial, body, .. } => initial.effects.union(body.effects),
            ExprKind::SetLocal { value, .. } => value.effects.union(EffectSet::MUTATES_LOCAL),
            ExprKind::ProductValue { fields, .. } => {
                fold_effects(fields).union(EffectSet::ALLOCATES)
            }
            ExprKind::ProductField { value, .. } => value.effects.union(EffectSet::READS_MEMORY),
            ExprKind::WithProductField {
                value, replacement, ..
            } => value
                .effects
                .union(replacement.effects)
                .union(EffectSet::READS_MEMORY)
                .union(EffectSet::ALLOCATES),
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
            | "var"
            | "quote"
            | "set"
            | "move"
            | "borrow"
            | "borrow-mut"
            | "empty-list"
            | "none"
            | "product"
            | "fields"
            | "field"
            | "product-value"
            | "with-field"
            | "bind"
            | "fn"
            | "def"
            | "main"
            | "sig"
            | "params"
            | "forall"
            | "bounds"
            | "bound"
            | "trait"
            | "impl"
            | "for"
            | "type"
            | "import"
            | "name"
    )
}

fn trait_declaration(args: &[AstExpr]) -> std::result::Result<String, String> {
    let [name_form] = args else {
        return Err("marker trait expects exactly one name/ form; methods and associated types are unsupported".into());
    };
    declared_name_form(name_form, "trait")
}

fn impl_declaration(args: &[AstExpr]) -> std::result::Result<(String, Type), String> {
    let [trait_form, for_form] = args else {
        return Err("marker impl expects exactly trait/ and for/ forms; methods, associated values, and generics are unsupported".into());
    };
    let trait_name = match trait_form {
        AstExpr::Call { name, args } if name == "trait" => match args.as_slice() {
            [trait_name] => symbolic_name(trait_name)?,
            _ => return Err("impl trait/ must contain exactly one trait name".into()),
        },
        _ => return Err("marker impl expects trait/ first".into()),
    };
    let target = match for_form {
        AstExpr::Call { name, args } if name == "for" => parse_type_form(args)?,
        _ => return Err("marker impl expects for/ second".into()),
    };
    Ok((trait_name, target))
}

fn product_declaration(args: &[AstExpr]) -> std::result::Result<(String, &[AstExpr]), String> {
    let [name_form, fields_form] = args else {
        return Err("product expects exactly name/ and fields/ forms".into());
    };
    let name = match name_form {
        AstExpr::Call {
            name,
            args: name_args,
        } if name == "name" => match name_args.as_slice() {
            [AstExpr::LitStr(name)] => name.clone(),
            _ => return Err("product name must be one non-empty name/ text line".into()),
        },
        _ => return Err("product expects name/…/name first".into()),
    };
    let fields = match fields_form {
        AstExpr::Call { name, args } if name == "fields" => args.as_slice(),
        _ => return Err("product expects fields/…/fields second".into()),
    };
    Ok((name, fields))
}

fn parse_product_field(expression: &AstExpr) -> std::result::Result<(String, Type), String> {
    let AstExpr::Call { name, args } = expression else {
        return Err("fields must contain field/…/field forms".into());
    };
    if name != "field" {
        return Err("fields must contain field/…/field forms".into());
    }
    let [name_form, type_form] = args.as_slice() else {
        return Err("field expects exactly name/ and type/ forms".into());
    };
    let field_name = match name_form {
        AstExpr::Call {
            name,
            args: name_args,
        } if name == "name" => match name_args.as_slice() {
            [AstExpr::LitStr(name)] => name.clone(),
            _ => return Err("field name must be one non-empty name/ text line".into()),
        },
        _ => return Err("field expects name/…/name first".into()),
    };
    let ty = match type_form {
        AstExpr::Call { name, args } if name == "type" => parse_type_form(args)?,
        _ => return Err("field expects type/…/type second".into()),
    };
    Ok((field_name, ty))
}

fn is_declaration_type_name(name: &str) -> bool {
    !name.is_empty()
        && name
            .chars()
            .next()
            .is_some_and(|character| character.is_ascii_uppercase())
        && name
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || character == '-')
}

fn is_builtin_type_name(name: &str) -> bool {
    matches!(
        name,
        "Unit"
            | "Bool"
            | "I64"
            | "F64"
            | "Str"
            | "Buf"
            | "Symbol"
            | "Handle"
            | "List"
            | "Option"
            | "Result"
            | "Product"
            | "Any"
            | "Int"
            | "Float"
    )
}

struct PendingFunction<'a> {
    binding: BindingId,
    origin: SourceId,
    parsed: ParsedFunction<'a>,
    bounds: Vec<TraitBound>,
}

struct PendingMain<'a> {
    origin: SourceId,
    return_type: Type,
    body: &'a AstExpr,
}

struct ParsedBound {
    parameter: String,
    trait_name: String,
}

struct ParsedFunction<'a> {
    signature_params: Vec<Type>,
    signature_return: Type,
    param_names: Vec<String>,
    param_types: Vec<Type>,
    body: &'a AstExpr,
    forall_vars: Vec<String>,
    bounds: Vec<ParsedBound>,
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

fn parse_main(args: &[AstExpr]) -> std::result::Result<(Type, &AstExpr), String> {
    let [signature_form, body] = args else {
        return Err("expected exactly sig/…/sig and one body expression".into());
    };
    let AstExpr::Call {
        name,
        args: signature_args,
    } = signature_form
    else {
        return Err("expected sig/…/sig first".into());
    };
    if name != "sig" {
        return Err("expected sig/…/sig first".into());
    }
    let (params, return_type) = parse_signature(signature_args)?;
    if !params.is_empty() {
        return Err("signature must have no parameters".into());
    }
    Ok((return_type, body))
}

fn parse_function(args: &[AstExpr]) -> std::result::Result<ParsedFunction<'_>, String> {
    let mut index = 0;
    let mut forall_vars = Vec::new();
    if let Some(AstExpr::Call { name, args }) = args.get(index) {
        if name == "forall" {
            if args.is_empty() {
                return Err("forall must declare at least one type parameter".into());
            }
            for variable in args {
                forall_vars.push(symbolic_name(variable)?);
            }
            index += 1;
        }
    }

    let mut bounds = Vec::new();
    if let Some(AstExpr::Call { name, args }) = args.get(index) {
        if name == "bounds" {
            if args.is_empty() {
                return Err("bounds must contain at least one bound/ form".into());
            }
            for expression in args {
                let AstExpr::Call { name, args } = expression else {
                    return Err("bounds may contain only bound/ forms".into());
                };
                if name != "bound" {
                    return Err("bounds may contain only bound/ forms".into());
                }
                let [parameter, trait_name] = args.as_slice() else {
                    return Err("bound expects exactly a type parameter and trait name".into());
                };
                bounds.push(ParsedBound {
                    parameter: symbolic_name(parameter)?,
                    trait_name: symbolic_name(trait_name)?,
                });
            }
            index += 1;
        }
    }

    let signature = match args.get(index) {
        Some(AstExpr::Call { name, args }) if name == "sig" => parse_signature(args)?,
        _ => return Err("fn expects sig/ after optional forall/ and bounds/".into()),
    };
    index += 1;
    let params = match args.get(index) {
        Some(AstExpr::Call { name, args }) if name == "params" => parse_typed_params(args)?,
        _ => return Err("fn expects params/ immediately after sig/".into()),
    };
    index += 1;
    let Some(body) = args.get(index) else {
        return Err("fn missing body".into());
    };
    index += 1;
    if index != args.len() {
        return Err("fn has extra children or multiple body expressions; wrap executable expressions in do/".into());
    }
    Ok(ParsedFunction {
        signature_params: signature.0,
        signature_return: signature.1,
        param_names: params.0,
        param_types: params.1,
        body,
        forall_vars,
        bounds,
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
        Type::Owned(inner)
        | Type::Ref(inner)
        | Type::RefMut(inner)
        | Type::List(inner)
        | Type::Option(inner) => collect_type_params(inner, output),
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

fn contains_ownership_type(ty: &Type) -> bool {
    match ty {
        Type::Owned(_) | Type::Ref(_) | Type::RefMut(_) => true,
        Type::List(inner) | Type::Option(inner) => contains_ownership_type(inner),
        Type::Result(ok, error) => contains_ownership_type(ok) || contains_ownership_type(error),
        Type::Fn { params, ret } => {
            params.iter().any(contains_ownership_type) || contains_ownership_type(ret)
        }
        Type::Forall { body, .. } => contains_ownership_type(body),
        _ => false,
    }
}

fn contains_reference_type(ty: &Type) -> bool {
    match ty {
        Type::Ref(_) | Type::RefMut(_) => true,
        Type::Owned(inner) | Type::List(inner) | Type::Option(inner) => {
            contains_reference_type(inner)
        }
        Type::Result(ok, error) => contains_reference_type(ok) || contains_reference_type(error),
        Type::Fn { params, ret } => {
            params.iter().any(contains_reference_type) || contains_reference_type(ret)
        }
        Type::Forall { body, .. } => contains_reference_type(body),
        _ => false,
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
        AstExpr::Call { name, args }
            if matches!(name.as_str(), "Owned" | "Ref" | "RefMut") && args.len() == 1 =>
        {
            let inner = parameter_type(&args[0])?;
            if inner != Type::Buf {
                return Err(format!(
                    "{name} accepts only exact Buf in the initial ownership slice"
                ));
            }
            Ok(match name.as_str() {
                "Owned" => Type::Owned(Box::new(inner)),
                "Ref" => Type::Ref(Box::new(inner)),
                "RefMut" => Type::RefMut(Box::new(inner)),
                _ => return Err("invalid ownership parameter type".into()),
            })
        }
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
        AstExpr::Call { name, args } if name == "Product" && args.len() == 1 => {
            Ok(Type::Product(symbolic_name(&args[0])?))
        }
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

fn declared_name_form(expression: &AstExpr, context: &str) -> std::result::Result<String, String> {
    match expression {
        AstExpr::Call { name, args } if name == "name" => match args.as_slice() {
            [AstExpr::LitStr(name)] if !name.is_empty() => Ok(name.clone()),
            _ => Err(format!(
                "{context} name must be one non-empty name/ text line"
            )),
        },
        _ => Err(format!("{context} expects name/…/name first")),
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
