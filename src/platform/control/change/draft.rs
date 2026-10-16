//! Deterministic native proposals from one accepted revision. The accepted graph remains the
//! authority; generated aliases, whitespace and comments are disposable presentation choices.
use super::canonical::{Reader, error};
use super::*;
use crate::platform::execution::ExecutionControl;
use crate::platform::kernel::{self as k, DeclarationPayload as D, OwnerRecord as O};
use crate::platform::publication::RepositoryView;

pub(crate) fn render(
    view: &RepositoryView,
    selected: &[NativeDraftSelection],
    maximum: usize,
    control: ExecutionControl,
) -> Result<Vec<u8>, Diagnostic> {
    render_with_reader(view, selected, maximum, Reader::new(view, control))
}

/// Raw candidate helper evidence only. The view remains an ordinary admitted revision;
/// neither this renderer nor its seeded Reader admits or publishes associated meaning.
#[cfg(test)]
pub(super) fn render_raw_lowered_candidate(
    view: &RepositoryView,
    selected: &[NativeDraftSelection],
    maximum: usize,
    control: ExecutionControl,
    owners: BTreeMap<OwnerKey, O>,
    types: BTreeMap<k::TypeObjectDigest, k::TypeObject>,
) -> Result<Vec<u8>, Diagnostic> {
    render_with_reader(
        view,
        selected,
        maximum,
        Reader::for_raw_lowered_candidate(view, control, owners, types),
    )
}

fn render_with_reader(
    view: &RepositoryView,
    selected: &[NativeDraftSelection],
    maximum: usize,
    reader: Reader<'_>,
) -> Result<Vec<u8>, Diagnostic> {
    let mut writer = Renderer {
        reader,
        aliases: BTreeMap::new(),
        types: Vec::new(),
        references: BTreeMap::new(),
        reserved_names: BTreeSet::new(),
        maximum,
        pool_bytes: 0,
    };
    let mut modules: BTreeMap<ModuleId, BTreeSet<OwnerKey>> = BTreeMap::new();
    let mut targets = BTreeSet::new();
    if selected.is_empty() {
        return Err(error(
            "draft requires at least one explicit owner selection",
        ));
    }
    if selected.len() > crate::platform::change::MAXIMUM_AUTHORED_CHANGES {
        return Err(Diagnostic::new(
            DiagnosticClass::Resource,
            "change_draft_capacity",
            "draft selection count exceeds complete change admission",
        ));
    }
    let selected = selected
        .iter()
        .map(|selection| selection.resolve(&mut writer.reader))
        .collect::<Result<Vec<_>, _>>()?;
    for owner in &selected {
        match writer.reader.owner(*owner)? {
            O::Module(_) => {
                let OwnerKey::Module(module) = owner else {
                    return Err(error("module selector has wrong identity"));
                };
                let declarations = modules.entry(*module).or_default();
                for edge in writer.reader.reader.incoming(
                    *owner,
                    k::RelationKind::DeclarationModule,
                    crate::platform::change::MAXIMUM_AUTHORED_CHANGES,
                )? {
                    let k::RelationEndpoint::Owner(source) = edge.source else {
                        return Err(error("module ownership edge has no owner source"));
                    };
                    if source.package != view.package() {
                        return Err(error("module ownership escaped its package"));
                    }
                    let O::Declaration(d) = writer.reader.owner(source.owner)? else {
                        return Err(error("module owns a non-declaration"));
                    };
                    if d.module != *module {
                        return Err(error(
                            "module ownership disagrees with canonical declaration",
                        ));
                    }
                    declarations.insert(source.owner);
                }
            }
            O::Declaration(d) => {
                modules.entry(d.module).or_default().insert(*owner);
            }
            O::Target(_) => {
                targets.insert(*owner);
            }
            _ => {
                return Err(error(
                    "draft selections must be exact module, declaration or target identities",
                ));
            }
        }
    }
    // Generated collection-scope aliases must not be shadowed by a selected declaration
    // or signature child. Reserve all such names before rendering any reference or type.
    for (module, owners) in &modules {
        writer.reserve_names(OwnerKey::Module(*module))?;
        for owner in owners {
            writer.reserve_names(*owner)?;
        }
    }
    for target in &targets {
        writer.reserve_names(*target)?;
    }
    let mut body = String::new();
    for (module, owners) in modules {
        let O::Module(m) = writer.reader.owner(OwnerKey::Module(module))? else {
            return Err(error("declaration's module is absent"));
        };
        append(
            &mut body,
            &format!("  (module edit {module} {}\n", m.name),
            maximum,
        )?;
        for owner in owners {
            let unit = writer.unit(owner, 4)?;
            append(&mut body, &unit, maximum)?;
        }
        append(&mut body, "  )\n", maximum)?;
    }
    for target in targets {
        let unit = writer.unit(target, 2)?;
        append(&mut body, &unit, maximum)?;
    }
    let mut output = format!(
        "request base={} repository={} package={}\n\ndeclarations.begin\n(units\n",
        view.revision(),
        view.current().head.repository_id,
        view.package()
    );
    append(
        &mut output,
        "  ; Reconstructed from canonical meaning. Original comments, formatting and type aliases are not stored.\n",
        maximum,
    )?;
    for ((class, exact), alias) in &writer.references {
        append(
            &mut output,
            &format!("  (reference {alias} {class} {exact})\n"),
            maximum,
        )?;
    }
    for (name, ty) in &writer.types {
        append(
            &mut output,
            &format!("  (type-alias {name} {ty})\n"),
            maximum,
        )?;
    }
    append(&mut output, &body, maximum)?;
    append(&mut output, ")\ndeclarations.end\n", maximum)?;
    let output = wrap_lines(&output, maximum)?;
    writer.reader.check()?;
    // Output admission includes the public input decoder. A canonical form that cannot be
    // expressed completely must fail before the caller atomically exposes the destination.
    let parsed = input::parse("<native-draft>", output.as_bytes()).map_err(|errors| {
        errors
            .into_iter()
            .next()
            .unwrap_or_else(|| error("draft input admission failed"))
    })?;
    super::decode_parsed_change(parsed, Some(&mut writer.reader)).map_err(|errors| {
        errors
            .into_iter()
            .next()
            .unwrap_or_else(|| error("draft normalization failed"))
    })?;
    writer.reader.check()?;
    Ok(output.into_bytes())
}

fn wrap_lines(source: &str, maximum: usize) -> Result<String, Diagnostic> {
    let mut output = String::new();
    let mut quoted = false;
    let mut escaped = false;
    let mut comment = false;
    let mut depth = 0_usize;
    let mut column = 0_usize;
    for character in source.chars() {
        if !quoted && !comment && character == '(' {
            if column > 100 && output.ends_with(' ') {
                output.pop();
                let padding = " ".repeat(depth.saturating_mul(2).min(80));
                append(&mut output, &format!("\n{padding}"), maximum)?;
                column = padding.len();
            }
            depth = depth
                .checked_add(1)
                .ok_or_else(|| error("draft indentation overflow"))?;
        } else if !quoted && !comment && character == ')' {
            depth = depth
                .checked_sub(1)
                .ok_or_else(|| error("draft delimiters are unbalanced"))?;
        }
        let mut bytes = [0; 4];
        append(&mut output, character.encode_utf8(&mut bytes), maximum)?;
        if character == '\n' {
            column = 0;
            comment = false;
        } else {
            column += character.len_utf8();
        }
        if quoted {
            if escaped {
                escaped = false;
            } else if character == '\\' {
                escaped = true;
            } else if character == '"' {
                quoted = false;
            }
        } else if !comment {
            if character == '"' {
                quoted = true;
            } else if character == ';' {
                comment = true;
            }
        }
    }
    Ok(output)
}

fn append(output: &mut String, text: &str, maximum: usize) -> Result<(), Diagnostic> {
    if output
        .len()
        .checked_add(text.len())
        .is_none_or(|size| size > maximum)
    {
        return Err(Diagnostic::new(
            DiagnosticClass::Resource,
            "change_draft_capacity",
            "complete native draft exceeds the selected output admission; select smaller ownership scopes",
        ));
    }
    output
        .try_reserve(text.len())
        .map_err(|_| error("draft output allocation failed"))?;
    output.push_str(text);
    Ok(())
}

struct Renderer<'a> {
    reader: Reader<'a>,
    aliases: BTreeMap<String, String>,
    types: Vec<(String, String)>,
    references: BTreeMap<(String, String), String>,
    reserved_names: BTreeSet<String>,
    maximum: usize,
    pool_bytes: usize,
}

impl Renderer<'_> {
    fn reserve_names(&mut self, owner: OwnerKey) -> Result<(), Diagnostic> {
        let mut pending = vec![owner];
        while let Some(owner) = pending.pop() {
            let record = self.reader.owner(owner)?;
            if let Some(name) = record.name()
                && !self.reserved_names.contains(name.as_str())
            {
                self.admit_pool(name.as_str().len())?;
                self.reserved_names.insert(name.to_string());
            }
            pending.extend(declarations::contract_children(&record));
        }
        Ok(())
    }

    fn alias(&mut self, prefix: &str, mut index: usize) -> Result<String, Diagnostic> {
        loop {
            self.reader.check()?;
            let name = format!("{prefix}_{index}");
            if !self.reserved_names.contains(&name) {
                self.admit_pool(name.len())?;
                self.reserved_names.insert(name.clone());
                return Ok(name);
            }
            index = index
                .checked_add(1)
                .ok_or_else(|| error("draft alias index overflow"))?;
        }
    }

    fn reference(&mut self, class: &str, exact: String) -> Result<String, Diagnostic> {
        let key = (class.to_owned(), exact);
        if let Some(alias) = self.references.get(&key) {
            return Ok(alias.clone());
        }
        let selector = key.1.strip_prefix("parameter:").unwrap_or(&key.1);
        let (package, owner) = selector
            .split_once('/')
            .ok_or_else(|| error("canonical reference lacks a package"))?;
        let name = self
            .reader
            .reference_name(package.parse()?, owner.parse()?)?;
        let prefix = format!("ref_{}", &name[..name.len().min(96)]);
        let alias = self.alias(&prefix, self.references.len())?;
        self.admit_pool(key.0.len() + key.1.len())?;
        self.references.insert(key, alias.clone());
        Ok(alias)
    }

    fn admit_pool(&mut self, bytes: usize) -> Result<(), Diagnostic> {
        self.pool_bytes = self
            .pool_bytes
            .checked_add(bytes)
            .filter(|n| *n <= self.maximum)
            .ok_or_else(|| {
                Diagnostic::new(
                    DiagnosticClass::Resource,
                    "change_draft_capacity",
                    "draft aliases exceed complete output admission",
                )
            })?;
        Ok(())
    }

    fn declaration(&mut self, d: &AuthoredDeclarationReference) -> Result<String, Diagnostic> {
        match d {
            AuthoredDeclarationReference::Exact {
                package,
                declaration,
            } => self.reference("declaration", format!("{package}/{declaration}")),
            _ => Err(error("canonical declaration is not an exact reference")),
        }
    }
    fn case(&mut self, c: &AuthoredCaseReference) -> Result<String, Diagnostic> {
        match c {
            AuthoredCaseReference::Exact { package, case } => {
                self.reference("case", format!("{package}/{case}"))
            }
            _ => Err(error("canonical case is not an exact reference")),
        }
    }
    fn requirement(&mut self, r: &AuthoredRequirementReference) -> Result<String, Diagnostic> {
        match r {
            AuthoredRequirementReference::Exact {
                package,
                requirement,
            } => self.reference("requirement", format!("{package}/{requirement}")),
            AuthoredRequirementReference::ParameterExact { package, parameter } => self.reference(
                "requirement-parameter",
                format!("parameter:{package}/{parameter}"),
            ),
            _ => Err(error("canonical requirement is not an exact reference")),
        }
    }
    fn effect_parameter(
        &mut self,
        p: &AuthoredEffectParameterReference,
    ) -> Result<String, Diagnostic> {
        match p {
            AuthoredEffectParameterReference::Exact { package, parameter } => {
                self.reference("effect-parameter", format!("{package}/{parameter}"))
            }
            _ => Err(error("canonical effect parameter is not exact")),
        }
    }
    fn row(&mut self, row: &AuthoredEffectRow, head: &str) -> Result<String, Diagnostic> {
        let mut text = format!("({head}");
        for r in &row.requirements {
            append(
                &mut text,
                &format!(" (requirement {})", self.requirement(r)?),
                self.maximum,
            )?;
        }
        for p in &row.parameters {
            append(
                &mut text,
                &format!(" (parameter {})", self.effect_parameter(p)?),
                self.maximum,
            )?;
        }
        text.push(')');
        Ok(text)
    }

    fn read_scope(&mut self, scope: &AuthoredReadScopeArgument) -> Result<String, Diagnostic> {
        self.reader.check()?;
        Ok(match scope {
            AuthoredReadScopeArgument::Parameter { binder, parameter } => {
                let binder = match binder {
                    AuthoredReadScopeBinder::Declaration(d) => {
                        format!("(declaration {})", self.declaration(d)?)
                    }
                    AuthoredReadScopeBinder::ContractMethod { contract, method } => {
                        format!("(contract-method {} {method})", self.declaration(contract)?)
                    }
                    AuthoredReadScopeBinder::AssociatedMember { contract, member } => format!(
                        "(associated-member {} {member})",
                        self.declaration(contract)?
                    ),
                    AuthoredReadScopeBinder::AssociatedBinding {
                        implementation,
                        member,
                    } => format!(
                        "(associated-binding {} {member})",
                        self.declaration(implementation)?
                    ),
                };
                format!("(read-scope {binder} {parameter})")
            }
            AuthoredReadScopeArgument::Lexical { callable, binding } => format!(
                "(lexical-read-scope {} {})",
                self.declaration(callable)?,
                self.local_token(binding)?
            ),
        })
    }
    fn canonical_read_scope(&mut self, scope: k::ReadScopeArgument) -> Result<String, Diagnostic> {
        let scope = self.reader.read_scope(scope)?;
        self.read_scope(&scope)
    }
    fn local_token(&self, local: &AuthoredLocalReference) -> Result<String, Diagnostic> {
        Ok(match local {
            AuthoredLocalReference::FunctionParameter { parameter }
            | AuthoredLocalReference::OperationParameter { parameter } => {
                format!("(exact {parameter})")
            }
            AuthoredLocalReference::Symbol { symbol } => symbol.clone(),
            _ => {
                return Err(error(
                    "canonical local is not an exact or lexical reference",
                ));
            }
        })
    }
    fn read_scope_arguments(
        &mut self,
        scopes: &[AuthoredReadScopeArgument],
    ) -> Result<String, Diagnostic> {
        let mut text = String::from(" (scopes");
        for scope in scopes {
            append(
                &mut text,
                &format!(" {}", self.read_scope(scope)?),
                self.maximum,
            )?;
        }
        append(&mut text, ")", self.maximum)?;
        Ok(text)
    }
    fn parameter_name(&mut self, id: ParameterId) -> Result<String, Diagnostic> {
        let O::Parameter(parameter) = self.reader.owner(OwnerKey::Parameter(id))? else {
            return Err(error("result or scope source is not a parameter"));
        };
        Ok(parameter.name.to_string())
    }
    fn named_result(
        &mut self,
        result: &k::CallableResult<ParameterId>,
    ) -> Result<String, Diagnostic> {
        let ty = self.type_digest(result.ty)?;
        let relation = match &result.relation {
            k::ResultRelation::Value => "(value)".to_owned(),
            k::ResultRelation::ReadFrom { source } => {
                format!("(read-from {})", self.parameter_name(*source)?)
            }
            k::ResultRelation::ViewFrom { source, scope } => format!(
                "(view-from {} {})",
                self.parameter_name(*source)?,
                self.canonical_read_scope(*scope)?
            ),
        };
        Ok(format!("(returns {ty} {relation})"))
    }
    fn method_result(&mut self, result: &k::CallableResult<u32>) -> Result<String, Diagnostic> {
        let ty = self.type_digest(result.ty)?;
        let relation = match &result.relation {
            k::ResultRelation::Value => "(value)".to_owned(),
            k::ResultRelation::ReadFrom { source } => format!("(read-from {source})"),
            k::ResultRelation::ViewFrom { source, scope } => format!(
                "(view-from {source} {})",
                self.canonical_read_scope(*scope)?
            ),
        };
        Ok(format!("(returns {ty} {relation})"))
    }

    fn ty(&mut self, ty: &AuthoredType) -> Result<String, Diagnostic> {
        use AuthoredType as T;
        let primitive = match ty {
            T::Unit {} => Some("Unit"),
            T::Bool {} => Some("Bool"),
            T::I64 {} => Some("I64"),
            T::F64 {} => Some("F64"),
            T::ByteBuffer {} => Some("ByteBuffer"),
            T::OwnedI64Cell {} => Some("OwnedI64Cell"),
            T::Bytes {} => Some("Bytes"),
            T::Text {} => Some("Text"),
            T::StaticText {} => Some("StaticText"),
            T::Secret {} => Some("Secret"),
            _ => None,
        };
        if let Some(value) = primitive {
            return Ok(value.to_owned());
        }
        let value = match ty {
            T::RegionBrand { callable, binding } => format!(
                "(region-brand {} {})",
                self.declaration(callable)?,
                self.local_token(binding)?
            ),
            T::Region { region } => format!("(region {})", self.ty(region)?),
            T::RelPtr { region, item } => {
                format!("(rel-ptr {} {})", self.ty(region)?, self.ty(item)?)
            }
            T::RegionSlice { region, item } => {
                format!("(region-slice {} {})", self.ty(region)?, self.ty(item)?)
            }
            T::RegionCap { region, mode } => format!(
                "(region-cap {} {})",
                self.ty(region)?,
                match mode {
                    crate::platform::change::AuthoredRegionMode::Unique => "unique".to_owned(),
                    crate::platform::change::AuthoredRegionMode::Frozen => "frozen".to_owned(),
                    crate::platform::change::AuthoredRegionMode::Pinned => "pinned".to_owned(),
                    crate::platform::change::AuthoredRegionMode::Shared { scope } =>
                        format!("(shared {})", self.read_scope(scope)?),
                }
            ),
            T::TypeParameter {
                parameter: AuthoredTypeParameterReference::Id { parameter },
            } => format!("(parameter-type {parameter})"),
            T::Named { declaration } => self.declaration(declaration)?,
            T::ReadViewApplied {
                declaration,
                arguments,
                read_scope,
            } => format!(
                "(read-view-application {} (types {}) {})",
                self.declaration(declaration)?,
                self.type_items(arguments)?,
                self.read_scope(read_scope)?
            ),
            T::AssociatedReadViewMember {
                contract,
                member,
                read_scope,
            } => format!(
                "(associated-read-view-member {} {member} {})",
                self.declaration(contract)?,
                self.read_scope(read_scope)?
            ),
            T::AssociatedReadViewProjection {
                scope,
                parameter,
                contract,
                member,
                read_scope,
            } => format!(
                "(associated-read-view-projection {} {parameter} {} {member} {})",
                self.declaration(scope)?,
                self.declaration(contract)?,
                self.read_scope(read_scope)?
            ),
            T::AssociatedContractMember { contract, member } => format!(
                "(associated-contract-member {} {member})",
                self.declaration(contract)?
            ),
            T::AssociatedWitnessProjection {
                scope,
                parameter,
                contract,
                member,
            } => format!(
                "(associated-witness-projection {} {parameter} {} {member})",
                self.declaration(scope)?,
                self.declaration(contract)?
            ),
            T::Applied {
                declaration,
                arguments,
            } => {
                let d = self.declaration(declaration)?;
                self.typed_list(&d, arguments)?
            }
            T::CapabilityResource { interface } => {
                format!("(resource {})", self.declaration(interface)?)
            }
            T::OwnedSequence { item } => format!("(owned-sequence {})", self.ty(item)?),
            T::List { item } => format!("(list {})", self.ty(item)?),
            T::Option { item } => format!("(option {})", self.ty(item)?),
            T::Stream { item } => format!("(stream {})", self.ty(item)?),
            T::Map { key, value } => format!("(map {} {})", self.ty(key)?, self.ty(value)?),
            T::Result { ok, error } => format!("(result {} {})", self.ty(ok)?, self.ty(error)?),
            T::Function { parameters, result } => {
                let p = self.type_items(parameters)?;
                format!("(function ({p}) {})", self.ty(result)?)
            }
            T::TaskFunction {
                parameters,
                result,
                effect,
            } => {
                let p = self.type_items(parameters)?;
                format!(
                    "(task-function ({p}) {} {})",
                    self.ty(result)?,
                    self.row(effect, "row")?
                )
            }
            T::OwnedChoice { cases } => {
                let mut text = "(owned-choice".to_owned();
                for case in cases {
                    append(
                        &mut text,
                        &format!(" (case {} {})", case.name, self.ty(&case.ty)?),
                        self.maximum,
                    )?;
                }
                text.push(')');
                text
            }
            T::OwnedProduct { fields } => {
                let mut text = "(owned-product".to_owned();
                for field in fields {
                    append(
                        &mut text,
                        &format!(" (field {} {})", field.name, self.ty(&field.ty)?),
                        self.maximum,
                    )?;
                }
                text.push(')');
                text
            }
            T::StructuralRecord { fields } => {
                let mut text = "(record".to_owned();
                for field in fields {
                    append(
                        &mut text,
                        &format!(" ({} {})", field.name, self.ty(&field.ty)?),
                        self.maximum,
                    )?;
                }
                text.push(')');
                text
            }
            _ => return Err(error("unsupported noncanonical type reference")),
        };
        if let Some(alias) = self.aliases.get(&value) {
            return Ok(alias.clone());
        }
        let alias = self.alias("type", self.types.len())?;
        self.admit_pool(value.len())?;
        self.aliases.insert(value.clone(), alias.clone());
        self.types.push((alias.clone(), value));
        Ok(alias)
    }
    fn type_items(&mut self, types: &[AuthoredType]) -> Result<String, Diagnostic> {
        let mut text = String::new();
        for ty in types {
            if !text.is_empty() {
                append(&mut text, " ", self.maximum)?;
            }
            let ty = self.ty(ty)?;
            append(&mut text, &ty, self.maximum)?;
        }
        Ok(text)
    }
    fn typed_list(&mut self, head: &str, types: &[AuthoredType]) -> Result<String, Diagnostic> {
        Ok(format!("({head} {})", self.type_items(types)?))
    }
    fn implementation_operand(
        &mut self,
        operand: &AuthoredImplementationOperand,
    ) -> Result<String, Diagnostic> {
        Ok(match operand {
            AuthoredImplementationOperand::Concrete {
                implementation,
                type_arguments,
                implementations,
            } => {
                if type_arguments.is_empty() && implementations.is_empty() {
                    self.declaration(implementation)?
                } else {
                    let mut application =
                        format!("(implementation {}", self.declaration(implementation)?);
                    if !type_arguments.is_empty() {
                        append(
                            &mut application,
                            &format!(" {}", self.typed_list("types", type_arguments)?),
                            self.maximum,
                        )?;
                    }
                    append(
                        &mut application,
                        &self.implementation_arguments(implementations)?,
                        self.maximum,
                    )?;
                    append(&mut application, ")", self.maximum)?;
                    application
                }
            }
            AuthoredImplementationOperand::Parameter { scope, parameter } => {
                format!("parameter@{}@{parameter}", self.declaration(scope)?)
            }
        })
    }

    fn implementation_arguments(
        &mut self,
        operands: &[AuthoredImplementationOperand],
    ) -> Result<String, Diagnostic> {
        if operands.is_empty() {
            return Ok(String::new());
        }
        let mut text = String::from(" (implementations");
        for operand in operands {
            let operand = self.implementation_operand(operand)?;
            append(&mut text, &format!(" {operand}"), self.maximum)?;
        }
        append(&mut text, ")", self.maximum)?;
        Ok(text)
    }

    fn implementation_parameter(
        &mut self,
        parameter: &k::ImplementationParameter,
    ) -> Result<String, Diagnostic> {
        let arguments = if parameter.type_arguments.is_empty() {
            String::new()
        } else {
            let types = parameter
                .type_arguments
                .iter()
                .map(|ty| self.type_digest(*ty))
                .collect::<Result<Vec<_>, _>>()?;
            format!(" (types {})", types.join(" "))
        };
        Ok(format!(
            "(implementation-parameter {} {} {} {}{})",
            parameter.id,
            parameter.name,
            self.declaration(&canonical::declaration(parameter.contract))?,
            self.type_digest(parameter.self_type)?,
            arguments,
        ))
    }
    fn type_digest(&mut self, ty: k::TypeObjectDigest) -> Result<String, Diagnostic> {
        let ty = self.reader.ty(ty)?;
        self.ty(&ty)
    }

    fn unit(&mut self, owner: OwnerKey, indent: usize) -> Result<String, Diagnostic> {
        self.reader.check()?;
        let record = self.reader.owner(owner)?;
        let kind = declarations::owner_family(&record);
        let name = record
            .name()
            .map(ToString::to_string)
            .unwrap_or_else(|| owner.to_string());
        let padding = " ".repeat(indent);
        let mut text = format!("{padding}({kind} edit {owner} {name}");
        let mut clauses = Vec::new();
        let mut children = Vec::new();
        match record {
            O::Declaration(d) => {
                clauses.push(format!("(visibility {})", visibility(d.visibility)));
                match d.payload {
                    D::OwnedContract(c) => {
                        children.push(OwnerKey::TypeParameter(c.self_parameter));
                        children.extend(
                            c.type_parameters
                                .iter()
                                .copied()
                                .map(OwnerKey::TypeParameter),
                        );
                        let self_type = self.ty(&AuthoredType::TypeParameter {
                            parameter: AuthoredTypeParameterReference::Id {
                                parameter: c.self_parameter,
                            },
                        })?;
                        clauses.push(format!("(self {self_type})"));
                        for member in self.reader.associated_members(&c.associated_types)? {
                            let bound = match member.bound {
                                AssociatedTypeBound::Owned => "(bound owned)".to_owned(),
                                AssociatedTypeBound::ReadViewFamily { scope_parameter } => format!(
                                    "(read-view-family (scope-parameter {} {}))",
                                    scope_parameter.id, scope_parameter.name
                                ),
                            };
                            clauses.push(format!(
                                "(associated-type {} {} {bound})",
                                member.id, member.name
                            ));
                        }
                        for method in c.methods {
                            let mut parameters = String::new();
                            for p in method.parameters {
                                append(
                                    &mut parameters,
                                    &format!(
                                        " ({} {})",
                                        self.type_digest(p.ty)?,
                                        match p.use_mode {
                                            k::ParameterUse::Unrestricted => "unrestricted",
                                            k::ParameterUse::Borrow => "borrow",
                                            k::ParameterUse::Consume => "consume",
                                        }
                                    ),
                                    self.maximum,
                                )?;
                            }
                            let effect = match method.effect {
                                k::FunctionEffect::Pure => String::new(),
                                effect => format!(
                                    " (effect {})",
                                    self.row(&canonical::row(effect.row()), "task")?
                                ),
                            };
                            self.reader
                                .admit_associated_inventory(method.read_scope_parameters.len())?;
                            self.reader
                                .admit_associated_inventory(method.read_scope_sources.len())?;
                            let mut scope_parameters = String::new();
                            for parameter in method.read_scope_parameters {
                                self.reader.check()?;
                                append(
                                    &mut scope_parameters,
                                    &format!(
                                        " (scope-parameter {} {})",
                                        parameter.id, parameter.name
                                    ),
                                    self.maximum,
                                )?;
                            }
                            let mut scope_sources = String::new();
                            for source in method.read_scope_sources {
                                append(
                                    &mut scope_sources,
                                    &format!(
                                        " (scope-source {} {})",
                                        source.source,
                                        self.canonical_read_scope(source.scope)?
                                    ),
                                    self.maximum,
                                )?;
                            }
                            clauses.push(format!("(method {} {} (parameters{}) {}{} (scope-parameters{}) (scope-sources{}))", method.id, method.name, parameters, self.method_result(&method.result)?, effect, scope_parameters, scope_sources));
                        }
                    }
                    D::OwnedImplementation(i) => {
                        children.extend(i.type_parameters.into_iter().map(OwnerKey::TypeParameter));
                        clauses.push(format!(
                            "(contract {})",
                            self.declaration(&canonical::declaration(i.contract))?
                        ));
                        clauses.push(format!("(self {})", self.type_digest(i.self_type)?));
                        if !i.type_arguments.is_empty() {
                            let arguments = i
                                .type_arguments
                                .iter()
                                .map(|ty| self.type_digest(*ty))
                                .collect::<Result<Vec<_>, _>>()?;
                            clauses.push(format!("(types {})", arguments.join(" ")));
                        }
                        for parameter in &i.implementation_parameters {
                            clauses.push(self.implementation_parameter(parameter)?);
                        }
                        for binding in self.reader.associated_bindings(&i.associated_types)? {
                            let value = match binding.value {
                                AuthoredAssociatedTypeBindingValue::Owned { ty } => self.ty(&ty)?,
                                AuthoredAssociatedTypeBindingValue::ReadViewFamily {
                                    scope_parameter,
                                    ty,
                                } => format!(
                                    "(read-view-family (scope-parameter {} {}) {})",
                                    scope_parameter.id,
                                    scope_parameter.name,
                                    self.ty(&ty)?
                                ),
                            };
                            clauses.push(format!("(associated-type {} {value})", binding.member));
                        }
                        for m in i.methods {
                            let arguments = if m.type_arguments.is_empty() {
                                String::new()
                            } else {
                                let types = m
                                    .type_arguments
                                    .into_iter()
                                    .map(|ty| self.type_digest(ty))
                                    .collect::<Result<Vec<_>, _>>()?;
                                format!(" (types {})", types.join(" "))
                            };
                            let implementations = m
                                .implementations
                                .into_iter()
                                .map(|operand| self.reader.implementation_operand(operand))
                                .collect::<Result<Vec<_>, _>>()?;
                            let implementations =
                                self.implementation_arguments(&implementations)?;
                            clauses.push(format!(
                                "(method {} {}{}{}{})",
                                m.method,
                                self.declaration(&canonical::declaration(m.function))?,
                                arguments,
                                implementations,
                                {
                                    let scopes = self.reader.read_scopes(m.read_scope_arguments)?;
                                    self.read_scope_arguments(&scopes)?
                                },
                            ));
                        }
                    }
                    D::ReadView(view) => {
                        children.extend(
                            view.type_parameters
                                .into_iter()
                                .map(OwnerKey::TypeParameter),
                        );
                        clauses.push(format!(
                            "(scope-parameter {} {})",
                            view.scope_parameter.id, view.scope_parameter.name
                        ));
                        for field in self.reader.view_fields(view.fields)? {
                            let kind = match field.kind {
                                AuthoredReadViewFieldKind::Ordinary => "(ordinary)".to_owned(),
                                AuthoredReadViewFieldKind::ReadReference { read_scope } => {
                                    format!("(read-reference {})", self.read_scope(&read_scope)?)
                                }
                            };
                            clauses.push(format!(
                                "(view-field {} {} (type {}) {kind})",
                                field.id,
                                field.name,
                                self.ty(&field.ty)?
                            ));
                        }
                    }
                    D::Record {
                        type_parameters,
                        fields,
                    }
                    | D::OwnedRecord {
                        type_parameters,
                        fields,
                    } => {
                        children.extend(type_parameters.into_iter().map(OwnerKey::TypeParameter));
                        children.extend(fields.into_iter().map(OwnerKey::Field));
                    }
                    D::Variant {
                        type_parameters,
                        cases,
                    } => {
                        children.extend(type_parameters.into_iter().map(OwnerKey::TypeParameter));
                        children.extend(cases.into_iter().map(OwnerKey::Case));
                    }
                    D::Interface { operations } => {
                        children.extend(operations.into_iter().map(OwnerKey::Operation))
                    }
                    D::Component {
                        requirements,
                        ports,
                    } => {
                        children.extend(requirements.into_iter().map(OwnerKey::Requirement));
                        children.extend(ports.into_iter().map(OwnerKey::Port));
                    }
                    D::Function(f) => {
                        for p in &f.implementation_parameters {
                            clauses.push(self.implementation_parameter(p)?);
                        }
                        children.extend(f.type_parameters.into_iter().map(OwnerKey::TypeParameter));
                        children.extend(
                            f.effect_parameters
                                .into_iter()
                                .map(OwnerKey::EffectParameter),
                        );
                        children.extend(
                            f.requirement_parameters
                                .into_iter()
                                .map(OwnerKey::RequirementParameter),
                        );
                        children.extend(f.parameters.iter().copied().map(OwnerKey::Parameter));
                        clauses.push(self.named_result(&f.result)?);
                        self.reader
                            .admit_associated_inventory(f.read_scope_parameters.len())?;
                        self.reader
                            .admit_associated_inventory(f.read_scope_sources.len())?;
                        for parameter in f.read_scope_parameters {
                            clauses.push(format!(
                                "(scope-parameter {} {})",
                                parameter.id, parameter.name
                            ));
                        }
                        for source in f.read_scope_sources {
                            clauses.push(format!(
                                "(scope-source {} {})",
                                self.parameter_name(source.source)?,
                                self.canonical_read_scope(source.scope)?
                            ));
                        }
                        let effect = match f.effect {
                            k::FunctionEffect::Pure => "pure".to_owned(),
                            effect => self.row(&canonical::row(effect.row()), "task")?,
                        };
                        clauses.push(format!("(effect {effect})"));
                        let mut env = BTreeMap::new();
                        for id in f.parameters {
                            let O::Parameter(p) = self.reader.owner(OwnerKey::Parameter(id))?
                            else {
                                return Err(error("signature parameter is not a parameter owner"));
                            };
                            env.insert(p.name.to_string(), vec![id.to_string()]);
                        }
                        let body = self.reader.expression(f.body)?;
                        clauses.push(format!("(body {})", self.expression(&body, &mut env)?));
                    }
                    D::External(f) => {
                        children.extend(f.type_parameters.into_iter().map(OwnerKey::TypeParameter));
                        children.extend(f.parameters.into_iter().map(OwnerKey::Parameter));
                        clauses.push(format!("(returns {})", self.type_digest(f.result)?));
                        clauses.push(format!("(implementation {})", f.implementation.as_str()));
                    }
                    D::Constant { ty, value } => {
                        clauses.push(format!("(type {})", self.type_digest(ty)?));
                        let value = self.reader.expression(value)?;
                        clauses.push(format!(
                            "(value {})",
                            self.expression(&value, &mut BTreeMap::new())?
                        ));
                    }
                    D::Test {
                        actual, expected, ..
                    } => {
                        for (name, id) in [("actual", actual), ("expected", expected)] {
                            let value = self.reader.expression(id)?;
                            clauses.push(format!(
                                "({name} {})",
                                self.expression(&value, &mut BTreeMap::new())?
                            ));
                        }
                    }
                }
            }
            O::TypeParameter(p) => clauses.push(format!("(constraint {})", p.constraints.name())),
            O::EffectParameter(_) => {}
            O::RequirementParameter(p) => {
                clauses.push(format!(
                    "(interface {})",
                    self.declaration(&canonical::declaration(p.constraint.interface))?
                ));
                clauses.push(self.operations(&p.constraint.operations)?);
            }
            O::Field(f) => clauses.push(format!("(type {})", self.type_digest(f.ty)?)),
            O::Case(c) => {
                if let Some(ty) = c.payload {
                    clauses.push(format!("(payload {})", self.type_digest(ty)?));
                }
            }
            O::Parameter(p) => {
                clauses.push(format!("(type {})", self.type_digest(p.ty)?));
                clauses.push(format!(
                    "(use {})",
                    match p.use_mode {
                        k::ParameterUse::Unrestricted => "unrestricted",
                        k::ParameterUse::Consume => "consume",
                        k::ParameterUse::Borrow => "borrow",
                    }
                ));
                if let Some(r) = p.resource_requirement {
                    clauses.push(format!(
                        "(requirement {})",
                        self.requirement(&canonical::requirement(r.into()))?
                    ));
                }
            }
            O::Operation(o) => {
                children.extend(o.parameters.into_iter().map(OwnerKey::Parameter));
                clauses.push(format!("(returns {})", self.type_digest(o.result)?));
                clauses.push(format!(
                    "(idempotency {})",
                    match o.idempotency {
                        k::Idempotency::Idempotent => "idempotent",
                        k::Idempotency::IdempotentWithKey => "idempotent-with-key",
                        k::Idempotency::NonIdempotent => "non-idempotent",
                    }
                ));
                clauses.push(format!(
                    "(external-visibility {})",
                    match o.external_visibility {
                        k::ExternalVisibility::None => "none",
                        k::ExternalVisibility::Possible => "possible",
                    }
                ));
            }
            O::Requirement(r) => {
                clauses.push(format!(
                    "(interface {})",
                    self.declaration(&canonical::declaration(r.interface))?
                ));
                clauses.push(self.operations(&r.operations)?);
                let mut limits = "(limits".to_owned();
                for limit in r.limits {
                    append(
                        &mut limits,
                        &format!(
                            " ({} {} {})",
                            limit.name,
                            limit.maximum,
                            unit_name(limit.unit)
                        ),
                        self.maximum,
                    )?;
                }
                limits.push(')');
                clauses.push(limits);
            }
            O::Port(p) => {
                clauses.push(format!("(type {})", self.type_digest(p.function_type)?));
                match p.implementation {
                    k::PortImplementation::Function(f) => clauses.push(format!(
                        "(function {})",
                        self.declaration(&canonical::declaration(f))?
                    )),
                    k::PortImplementation::Expression(value) => {
                        let value = self.reader.expression(value)?;
                        clauses.push(format!(
                            "(value {})",
                            self.expression(&value, &mut BTreeMap::new())?
                        ));
                    }
                }
            }
            O::Target(t) => {
                clauses.push(format!(
                    "(component {})",
                    self.declaration(&canonical::declaration(t.component))?
                ));
                clauses.push(format!(
                    "(runner {})",
                    match t.runner {
                        RunnerKind::Command => "command",
                        RunnerKind::Http => "http",
                        RunnerKind::Interactive => "interactive",
                        RunnerKind::Batch => "batch",
                        RunnerKind::Worker => "worker",
                        RunnerKind::Test => "test",
                    }
                ));
                if let Some(p) = t.port {
                    clauses.push(format!(
                        "(port {})",
                        self.reference("port", format!("{}/{}", p.package, p.port))?
                    ));
                }
                for edge in self.reader.reader.incoming(
                    owner,
                    k::RelationKind::HttpRouteTarget,
                    crate::platform::change::MAXIMUM_AUTHORED_CHANGES,
                )? {
                    let k::RelationEndpoint::Owner(source) = edge.source else {
                        return Err(error("route ownership has no source"));
                    };
                    if source.package != self.reader.view.package() {
                        return Err(error("route ownership escaped its package"));
                    }
                    let O::HttpRoute(route) = self.reader.owner(source.owner)? else {
                        return Err(error("target owns a non-route"));
                    };
                    if OwnerKey::Target(route.target) != owner {
                        return Err(error("route target differs from ownership edge"));
                    }
                    children.push(source.owner);
                }
                children.sort();
            }
            O::HttpRoute(route) => {
                clauses.push(format!("(method {})", route.method));
                clauses.push(format!(
                    "({} {})",
                    if matches!(route.selector, k::HttpRouteSelector::Exact { .. }) {
                        "path"
                    } else {
                        "pattern"
                    },
                    quote(&route.selector.display(), self.maximum)?
                ));
                clauses.push(format!(
                    "(port {})",
                    self.reference(
                        "port",
                        format!("{}/{}", route.port.package, route.port.port)
                    )?
                ));
            }
            _ => {
                return Err(error(
                    "selected canonical owner has no complete native declaration form",
                ));
            }
        }
        for clause in clauses {
            append(&mut text, &format!("\n{padding}  {clause}"), self.maximum)?;
        }
        for child in children {
            let value = self.unit(child, indent + 2)?;
            append(&mut text, "\n", self.maximum)?;
            append(&mut text, value.trim_end_matches('\n'), self.maximum)?;
        }
        append(&mut text, ")\n", self.maximum)?;
        Ok(text)
    }

    fn operations(&mut self, operations: &[k::OperationReference]) -> Result<String, Diagnostic> {
        let mut text = "(operations".to_owned();
        for o in operations {
            append(
                &mut text,
                &format!(
                    " {}",
                    self.reference("operation", format!("{}/{}", o.package, o.operation))?
                ),
                self.maximum,
            )?;
        }
        append(&mut text, ")", self.maximum)?;
        Ok(text)
    }
    fn field(
        &mut self,
        selector: &AuthoredFieldSelector,
        access: bool,
    ) -> Result<String, Diagnostic> {
        match selector {
            AuthoredFieldSelector::Structural { name } => Ok(if access {
                format!("(name {name})")
            } else {
                name.to_string()
            }),
            AuthoredFieldSelector::Nominal {
                field: AuthoredFieldReference::Exact { package, field },
            } => self.reference("field", format!("{package}/{field}")),
            _ => Err(error("canonical field selector is not exact")),
        }
    }
    fn application(
        &mut self,
        types: &[AuthoredType],
        effects: &[AuthoredEffectRow],
        requirements: &[AuthoredRequirementReference],
    ) -> Result<String, Diagnostic> {
        let mut text = String::new();
        if !types.is_empty() {
            append(
                &mut text,
                &format!(" (types {})", self.type_items(types)?),
                self.maximum,
            )?;
        }
        if !effects.is_empty() {
            append(&mut text, " (effects", self.maximum)?;
            for row in effects {
                append(
                    &mut text,
                    &format!(" {}", self.row(row, "row")?),
                    self.maximum,
                )?;
            }
            text.push(')');
        }
        if !requirements.is_empty() {
            append(&mut text, " (requirements", self.maximum)?;
            for r in requirements {
                append(
                    &mut text,
                    &format!(" {}", self.requirement(r)?),
                    self.maximum,
                )?;
            }
            text.push(')');
        }
        Ok(text)
    }

    fn expression(
        &mut self,
        expression: &AuthoredExpression,
        env: &mut BTreeMap<String, Vec<String>>,
    ) -> Result<String, Diagnostic> {
        self.reader.check()?;
        use AuthoredExpressionOperation as E;
        let value = match &expression.operation {
            E::Region { operation } => {
                use crate::platform::kernel::RegionExpression as R;
                let mut text = format!("({}", operation.name());
                if let R::With {
                    binding,
                    capacity,
                    body,
                } = operation
                {
                    let ty = binding
                        .declared_type
                        .as_ref()
                        .ok_or_else(|| error("region binder has no exact type"))?;
                    append(
                        &mut text,
                        &format!(
                            " (binding {} (as {}) (type {})) {}",
                            binding.name,
                            binding.symbol,
                            self.ty(ty)?,
                            self.expression(capacity, env)?
                        ),
                        self.maximum,
                    )?;
                    env.entry(binding.name.to_string())
                        .or_default()
                        .push(binding.symbol.clone());
                    append(
                        &mut text,
                        &format!(" (in {})", self.expression(body, env)?),
                        self.maximum,
                    )?;
                    if let Some(values) = env.get_mut(binding.name.as_str()) {
                        values.pop();
                    }
                } else {
                    if let Some(ty) = operation.ty() {
                        append(
                            &mut text,
                            &format!(" (type {})", self.ty(ty)?),
                            self.maximum,
                        )?;
                    }
                    if let Some(capability) = operation.capability() {
                        append(
                            &mut text,
                            &format!(" {}", self.local_token(capability)?),
                            self.maximum,
                        )?;
                    }
                    if let Some((requirement, operation)) = operation.store() {
                        let AuthoredOperationReference::Exact { package, operation } = operation
                        else {
                            return Err(error("region storage operation is not exact"));
                        };
                        append(
                            &mut text,
                            &format!(
                                " {} {}",
                                self.requirement(requirement)?,
                                self.reference("operation", format!("{package}/{operation}"))?
                            ),
                            self.maximum,
                        )?;
                    }
                    for child in operation.expressions().into_iter().flatten() {
                        append(
                            &mut text,
                            &format!(" {}", self.expression(child, env)?),
                            self.maximum,
                        )?;
                    }
                }
                text.push(')');
                text
            }
            E::Parallel { left, right } => format!(
                "(parallel {} {})",
                self.expression(left, env)?,
                self.expression(right, env)?,
            ),
            E::Unit {} => "(unit)".to_owned(),
            E::Bool { value } => format!("(bool {value})"),
            E::I64 { value } => format!("(i64 {value})"),
            E::F64 { value } => format!("(f64 {value})"),
            E::Text { value } | E::StaticText { value } => format!(
                "({} {})",
                if matches!(expression.operation, E::Text { .. }) {
                    "text"
                } else {
                    "static-text"
                },
                quote(value, self.maximum)?
            ),
            E::Local { value } => {
                let key = match value {
                    AuthoredLocalReference::FunctionParameter { parameter }
                    | AuthoredLocalReference::OperationParameter { parameter } => {
                        parameter.to_string()
                    }
                    AuthoredLocalReference::Symbol { symbol } => symbol.clone(),
                    _ => return Err(error("canonical local has unsupported reference")),
                };
                let name = env.iter().find_map(|(name, values)| {
                    (values.last() == Some(&key)).then_some(name.as_str())
                });
                match name {
                    Some(name) => format!("(local {name})"),
                    None if key.starts_with('$') => format!("(local {key})"),
                    None => format!("(local (exact {key}))"),
                }
            }
            E::Constant { declaration } => format!("(constant {})", self.declaration(declaration)?),
            E::If {
                condition,
                when_true,
                when_false,
            } => format!(
                "(if {} {} {})",
                self.expression(condition, env)?,
                self.expression(when_true, env)?,
                self.expression(when_false, env)?
            ),
            E::ChooseOwned {
                choice_type,
                case,
                value,
            } => format!(
                "(choose-owned (type {}) (case {case}) {})",
                self.ty(choice_type)?,
                self.expression(value, env)?,
            ),
            E::MatchOwned {
                choice_type,
                source,
                arms,
            }
            | E::MatchBorrowedOwned {
                choice_type,
                source,
                arms,
            } => {
                let mut text = format!(
                    "({} (type {}) {}",
                    if matches!(expression.operation, E::MatchBorrowedOwned { .. }) {
                        "match-borrowed-owned"
                    } else {
                        "match-owned"
                    },
                    self.ty(choice_type)?,
                    self.expression(source, env)?
                );
                for (name, binding, body) in arms {
                    let ty = self.ty(binding
                        .declared_type
                        .as_ref()
                        .ok_or_else(|| error("owned choice binding requires a type"))?)?;
                    env.entry(binding.name.to_string())
                        .or_default()
                        .push(binding.symbol.clone());
                    let body = self.expression(body, env)?;
                    if let Some(values) = env.get_mut(binding.name.as_str()) {
                        values.pop();
                    }
                    append(
                        &mut text,
                        &format!(
                            " (case {name} (binding {} (as {}) (type {ty})) (in {body}))",
                            binding.name, binding.symbol,
                        ),
                        self.maximum,
                    )?;
                }
                text.push(')');
                text
            }
            E::PackOwnedRecord {
                record_type,
                fields,
            } => {
                let mut text = format!("(pack-owned (type {})", self.ty(record_type)?);
                for (field, value) in fields {
                    let field = self.field(
                        &AuthoredFieldSelector::Nominal {
                            field: field.clone(),
                        },
                        false,
                    )?;
                    append(
                        &mut text,
                        &format!(" (field {field} {})", self.expression(value, env)?),
                        self.maximum,
                    )?;
                }
                text.push(')');
                text
            }
            E::UnpackOwnedRecord {
                record_type,
                source,
                fields,
                body,
            } => {
                let mut text = format!(
                    "(unpack-owned (type {}) {}",
                    self.ty(record_type)?,
                    self.expression(source, env)?
                );
                for (field, binding) in fields {
                    let field = self.field(
                        &AuthoredFieldSelector::Nominal {
                            field: field.clone(),
                        },
                        false,
                    )?;
                    let ty = self.ty(binding
                        .declared_type
                        .as_ref()
                        .ok_or_else(|| error("unpack binding requires a type"))?)?;
                    append(
                        &mut text,
                        &format!(
                            " (field {field} (binding {} (as {}) (type {ty})))",
                            binding.name, binding.symbol
                        ),
                        self.maximum,
                    )?;
                    env.entry(binding.name.to_string())
                        .or_default()
                        .push(binding.symbol.clone());
                }
                append(
                    &mut text,
                    &format!(" (in {}))", self.expression(body, env)?),
                    self.maximum,
                )?;
                for (_, binding) in fields.iter().rev() {
                    if let Some(values) = env.get_mut(binding.name.as_str()) {
                        values.pop();
                    }
                }
                text
            }
            E::BorrowOwnedRecordField {
                record_type,
                source,
                field,
                binding,
                body,
            } => {
                let field = self.field(
                    &AuthoredFieldSelector::Nominal {
                        field: field.clone(),
                    },
                    false,
                )?;
                let ty = self.ty(binding
                    .declared_type
                    .as_ref()
                    .ok_or_else(|| error("borrowed field binding requires a type"))?)?;
                let source = self.expression(source, env)?;
                env.entry(binding.name.to_string())
                    .or_default()
                    .push(binding.symbol.clone());
                let body = self.expression(body, env)?;
                if let Some(values) = env.get_mut(binding.name.as_str()) {
                    values.pop();
                }
                format!(
                    "(borrow-owned-field (type {}) {source} (field {field} (binding {} (as {}) (type {ty}))) (in {body}))",
                    self.ty(record_type)?,
                    binding.name,
                    binding.symbol
                )
            }
            E::PackOwned {
                product_type,
                fields,
            } => {
                let mut text = format!("(pack-owned (type {})", self.ty(product_type)?);
                for (name, value) in fields {
                    append(
                        &mut text,
                        &format!(" (field {name} {})", self.expression(value, env)?),
                        self.maximum,
                    )?;
                }
                text.push(')');
                text
            }
            E::UnpackOwned {
                product_type,
                source,
                fields,
                body,
            } => {
                let mut text = format!(
                    "(unpack-owned (type {}) {}",
                    self.ty(product_type)?,
                    self.expression(source, env)?
                );
                for (name, b) in fields {
                    let ty = self.ty(b
                        .declared_type
                        .as_ref()
                        .ok_or_else(|| error("unpack binding requires a type"))?)?;
                    append(
                        &mut text,
                        &format!(
                            " (field {name} (binding {} (as {}) (type {ty})))",
                            b.name, b.symbol
                        ),
                        self.maximum,
                    )?;
                    env.entry(b.name.to_string())
                        .or_default()
                        .push(b.symbol.clone());
                }
                append(
                    &mut text,
                    &format!(" (in {}))", self.expression(body, env)?),
                    self.maximum,
                )?;
                for (_, b) in fields.iter().rev() {
                    if let Some(v) = env.get_mut(b.name.as_str()) {
                        v.pop();
                    }
                }
                text
            }
            E::CreateReadView {
                declaration,
                type_arguments,
                read_scope,
                source,
                fields,
            } => {
                let mut text = format!(
                    "(view-create {} (types {}) (scope {}) (source {})",
                    self.declaration(declaration)?,
                    self.type_items(type_arguments)?,
                    self.read_scope(read_scope)?,
                    self.local_token(source)?
                );
                for field in fields {
                    append(
                        &mut text,
                        &format!(
                            " (field {} {})",
                            field.field,
                            self.expression(&field.value, env)?
                        ),
                        self.maximum,
                    )?;
                }
                append(&mut text, ")", self.maximum)?;
                text
            }
            E::BorrowReadView {
                value,
                binding,
                body,
            }
            | E::BorrowReadViewField {
                value,
                binding,
                body,
                ..
            } => {
                let selector = match &expression.operation {
                    E::BorrowReadViewField { field, .. } => {
                        format!(" {} {}", self.declaration(&field.declaration)?, field.field)
                    }
                    _ => String::new(),
                };
                let head = if matches!(&expression.operation, E::BorrowReadView { .. }) {
                    "view-borrow"
                } else {
                    "borrow-view-field"
                };
                let ty = binding
                    .declared_type
                    .as_ref()
                    .map(|t| self.ty(t))
                    .transpose()?
                    .map(|t| format!(" (type {t})"))
                    .unwrap_or_default();
                let value = self.local_token(value)?;
                env.entry(binding.name.to_string())
                    .or_default()
                    .push(binding.symbol.clone());
                let body = self.expression(body, env)?;
                if let Some(values) = env.get_mut(binding.name.as_str()) {
                    values.pop();
                }
                format!(
                    "({head} (local {value}){selector} (binding {} (as {}){ty}) (in {body}))",
                    binding.name, binding.symbol
                )
            }
            E::ReborrowReadView { value, read_scope } => format!(
                "(view-reborrow (local {}) {})",
                self.local_token(value)?,
                self.read_scope(read_scope)?
            ),
            E::MoveReadView { value } => {
                format!("(view-move (local {}))", self.local_token(value)?)
            }
            E::DropReadView { value } => {
                format!("(view-drop (local {}))", self.local_token(value)?)
            }
            E::ReadViewDataField { value, field } => format!(
                "(view-data-field (local {}) {} {})",
                self.local_token(value)?,
                self.declaration(&field.declaration)?,
                field.field
            ),
            E::BorrowCall {
                call,
                binding,
                body,
            } => {
                let ty = self.ty(binding
                    .declared_type
                    .as_ref()
                    .ok_or_else(|| error("borrowed call binding requires a type"))?)?;
                let call = self.expression(call, env)?;
                env.entry(binding.name.to_string())
                    .or_default()
                    .push(binding.symbol.clone());
                let body = self.expression(body, env)?;
                if let Some(values) = env.get_mut(binding.name.as_str()) {
                    values.pop();
                }
                format!(
                    "(borrow-call {call} (binding {} (as {}) (type {ty})) (in {body}))",
                    binding.name, binding.symbol
                )
            }
            E::BorrowOwnedField {
                product_type,
                source,
                field,
                binding,
                body,
            } => {
                let ty = self.ty(binding
                    .declared_type
                    .as_ref()
                    .ok_or_else(|| error("borrowed field binding requires a type"))?)?;
                let source = self.expression(source, env)?;
                env.entry(binding.name.to_string())
                    .or_default()
                    .push(binding.symbol.clone());
                let body = self.expression(body, env)?;
                if let Some(values) = env.get_mut(binding.name.as_str()) {
                    values.pop();
                }
                format!(
                    "(borrow-owned-field (type {}) {source} (field {field} (binding {} (as {}) (type {ty}))) (in {body}))",
                    self.ty(product_type)?,
                    binding.name,
                    binding.symbol,
                )
            }
            E::SequenceEmpty { sequence_type } => {
                format!("(sequence-empty (type {}))", self.ty(sequence_type)?)
            }
            E::SequenceLength {
                sequence_type,
                source,
            } => format!(
                "(sequence-length (type {}) {})",
                self.ty(sequence_type)?,
                self.expression(source, env)?,
            ),
            E::SequencePush {
                sequence_type,
                value,
                source,
            } => format!(
                "(sequence-push (type {}) {} {})",
                self.ty(sequence_type)?,
                self.expression(value, env)?,
                self.expression(source, env)?,
            ),
            E::SequencePop {
                sequence_type,
                source,
            } => format!(
                "(sequence-pop (type {}) {})",
                self.ty(sequence_type)?,
                self.expression(source, env)?,
            ),
            E::SequenceGet {
                sequence_type,
                source,
                index,
            } => format!(
                "(sequence-get (type {}) {} (index {}))",
                self.ty(sequence_type)?,
                self.expression(source, env)?,
                self.expression(index, env)?,
            ),
            E::SequenceReplace {
                sequence_type,
                index,
                value,
                source,
            } => format!(
                "(sequence-replace (type {}) (index {}) {} {})",
                self.ty(sequence_type)?,
                self.expression(index, env)?,
                self.expression(value, env)?,
                self.expression(source, env)?,
            ),
            E::BorrowOwnedItem {
                sequence_type,
                source,
                index,
                binding,
                body,
            } => {
                let ty = self.ty(binding
                    .declared_type
                    .as_ref()
                    .ok_or_else(|| error("borrowed item binding requires a type"))?)?;
                let index = self.expression(index, env)?;
                let source = self.expression(source, env)?;
                env.entry(binding.name.to_string())
                    .or_default()
                    .push(binding.symbol.clone());
                let body = self.expression(body, env)?;
                if let Some(values) = env.get_mut(binding.name.as_str()) {
                    values.pop();
                }
                format!(
                    "(borrow-owned-item (type {}) {source} (index {index}) (binding {} (as {}) (type {ty})) (in {body}))",
                    self.ty(sequence_type)?,
                    binding.name,
                    binding.symbol,
                )
            }
            E::Let { bindings, body } => {
                let mut text = "(let".to_owned();
                for b in bindings {
                    let initial = self.expression(&b.value, env)?;
                    let ty = b
                        .declared_type
                        .as_ref()
                        .map(|t| self.ty(t))
                        .transpose()?
                        .map(|t| format!(" (type {t})"))
                        .unwrap_or_default();
                    append(
                        &mut text,
                        &format!(" (binding {} (as {}){ty} {initial})", b.name, b.symbol),
                        self.maximum,
                    )?;
                    env.entry(b.name.to_string())
                        .or_default()
                        .push(b.symbol.clone());
                }
                append(
                    &mut text,
                    &format!(" (in {}))", self.expression(body, env)?),
                    self.maximum,
                )?;
                for b in bindings.iter().rev() {
                    if let Some(v) = env.get_mut(b.name.as_str()) {
                        v.pop();
                    }
                }
                text
            }
            E::Sequence { items } => self.expressions("sequence", items, env)?,
            E::ImplementationCall {
                function,
                type_arguments,
                effect_arguments,
                requirement_arguments,
                read_scope_arguments,
                implementations,
                arguments,
            } => {
                let mut text = format!(
                    "(implementation-call {}{} (implementations",
                    self.declaration(function)?,
                    self.application(type_arguments, effect_arguments, requirement_arguments)?
                );
                for operand in implementations {
                    append(
                        &mut text,
                        &format!(" {}", self.implementation_operand(operand)?),
                        self.maximum,
                    )?;
                }
                text.push(')');
                append(
                    &mut text,
                    &self.read_scope_arguments(read_scope_arguments)?,
                    self.maximum,
                )?;
                for arg in arguments {
                    append(
                        &mut text,
                        &format!(" {}", self.expression(arg, env)?),
                        self.maximum,
                    )?;
                }
                text.push(')');
                text
            }
            E::MethodCall {
                witness,
                contract,
                method,
                read_scope_arguments,
                arguments,
            } => {
                let mut text = format!(
                    "(method-call {} {} {}{}",
                    self.implementation_operand(witness)?,
                    self.declaration(contract)?,
                    method,
                    self.read_scope_arguments(read_scope_arguments)?
                );
                for arg in arguments {
                    append(
                        &mut text,
                        &format!(" {}", self.expression(arg, env)?),
                        self.maximum,
                    )?;
                }
                text.push(')');
                text
            }
            E::Call {
                function,
                type_arguments,
                effect_arguments,
                requirement_arguments,
                read_scope_arguments,
                arguments,
            } => {
                let mut text = format!(
                    "(call {}{}{}",
                    self.declaration(function)?,
                    self.application(type_arguments, effect_arguments, requirement_arguments)?,
                    self.read_scope_arguments(read_scope_arguments)?
                );
                for arg in arguments {
                    append(
                        &mut text,
                        &format!(" {}", self.expression(arg, env)?),
                        self.maximum,
                    )?;
                }
                text.push(')');
                text
            }
            E::FunctionValue {
                function,
                type_arguments,
                effect_arguments,
                requirement_arguments,
                read_scope_arguments,
            } => format!(
                "(function-value {}{}{})",
                self.declaration(function)?,
                self.application(type_arguments, effect_arguments, requirement_arguments)?,
                self.read_scope_arguments(read_scope_arguments)?
            ),
            E::Invoke { callee, arguments } | E::Bind { callee, arguments } => {
                let mut text = format!(
                    "({} {}",
                    if matches!(expression.operation, E::Invoke { .. }) {
                        "invoke"
                    } else {
                        "bind"
                    },
                    self.expression(callee, env)?
                );
                for arg in arguments {
                    append(
                        &mut text,
                        &format!(" {}", self.expression(arg, env)?),
                        self.maximum,
                    )?;
                }
                text.push(')');
                text
            }
            E::Record {
                nominal_type,
                type_arguments,
                fields,
            } => {
                let nominal = nominal_type
                    .as_ref()
                    .map(|d| self.declaration(d))
                    .transpose()?
                    .unwrap_or_else(|| "structural".into());
                let mut text = format!(
                    "(record {nominal}{}",
                    self.application(type_arguments, &[], &[])?
                );
                for f in fields {
                    append(
                        &mut text,
                        &format!(
                            " (field {} {})",
                            self.field(&f.selector, false)?,
                            self.expression(&f.value, env)?
                        ),
                        self.maximum,
                    )?;
                }
                text.push(')');
                text
            }
            E::Variant {
                case,
                type_arguments,
                payload,
            } => {
                let mut text = format!(
                    "(variant {}{}",
                    self.case(case)?,
                    self.application(type_arguments, &[], &[])?
                );
                if let Some(payload) = payload {
                    append(
                        &mut text,
                        &format!(" {}", self.expression(payload, env)?),
                        self.maximum,
                    )?;
                }
                text.push(')');
                text
            }
            E::Field { value, selector } => format!(
                "(field {} {})",
                self.expression(value, env)?,
                self.field(selector, true)?
            ),
            E::List { item_type, items } => {
                let head = format!("list {}", self.ty(item_type)?);
                self.expressions(&head, items, env)?
            }
            E::Map {
                key_type,
                value_type,
                entries,
            } => {
                let mut text = format!("(map {} {}", self.ty(key_type)?, self.ty(value_type)?);
                for e in entries {
                    append(
                        &mut text,
                        &format!(
                            " (entry {} {})",
                            self.expression(&e.key, env)?,
                            self.expression(&e.value, env)?
                        ),
                        self.maximum,
                    )?;
                }
                text.push(')');
                text
            }
            E::Match { value, arms } => {
                let mut text = format!("(match {}", self.expression(value, env)?);
                for arm in arms {
                    append(
                        &mut text,
                        &format!(" (arm {}", self.case(&arm.case)?),
                        self.maximum,
                    )?;
                    if let Some(b) = &arm.payload_binding {
                        let ty = self.ty(b
                            .declared_type
                            .as_ref()
                            .ok_or_else(|| error("match payload has no declared type"))?)?;
                        append(
                            &mut text,
                            &format!(" (payload {} {ty} (as {}))", b.name, b.symbol),
                            self.maximum,
                        )?;
                        env.entry(b.name.to_string())
                            .or_default()
                            .push(b.symbol.clone());
                    }
                    append(
                        &mut text,
                        &format!(" {})", self.expression(&arm.body, env)?),
                        self.maximum,
                    )?;
                    if let Some(b) = &arm.payload_binding
                        && let Some(values) = env.get_mut(b.name.as_str())
                    {
                        values.pop();
                    }
                }
                text.push(')');
                text
            }
            E::CapabilityCall {
                requirement,
                operation,
                arguments,
            } => {
                let AuthoredOperationReference::Exact { package, operation } = operation else {
                    return Err(error("capability operation is not exact"));
                };
                let mut text = format!(
                    "(capability-call {} {}",
                    self.requirement(requirement)?,
                    self.reference("operation", format!("{package}/{operation}"))?
                );
                for arg in arguments {
                    append(
                        &mut text,
                        &format!(" {}", self.expression(arg, env)?),
                        self.maximum,
                    )?;
                }
                text.push(')');
                text
            }
            E::Transaction {
                requirement,
                binding,
                body,
            }
            | E::TransactionOutcome {
                requirement,
                binding,
                body,
                ..
            } => {
                let mut text = format!(
                    "({} {}",
                    if matches!(expression.operation, E::Transaction { .. }) {
                        "transaction"
                    } else {
                        "transaction-outcome"
                    },
                    self.requirement(requirement)?
                );
                if let E::TransactionOutcome {
                    type_argument,
                    outcome,
                    ..
                } = &expression.operation
                {
                    append(
                        &mut text,
                        &format!(
                            " (types {}) (outcome {} {} {} {} {} {})",
                            self.ty(type_argument)?,
                            self.declaration(&outcome.outcome)?,
                            self.declaration(&outcome.abort_reason)?,
                            self.case(&outcome.committed)?,
                            self.case(&outcome.aborted)?,
                            self.case(&outcome.condition_failed)?,
                            self.case(&outcome.conflict)?
                        ),
                        self.maximum,
                    )?;
                }
                append(
                    &mut text,
                    &format!(" (binding {} (as {}))", binding.name, binding.symbol),
                    self.maximum,
                )?;
                env.entry(binding.name.to_string())
                    .or_default()
                    .push(binding.symbol.clone());
                append(
                    &mut text,
                    &format!(" {})", self.expression(body, env)?),
                    self.maximum,
                )?;
                if let Some(v) = env.get_mut(binding.name.as_str()) {
                    v.pop();
                }
                text
            }
        };
        if value.len() > self.maximum {
            return Err(Diagnostic::new(
                DiagnosticClass::Resource,
                "change_draft_capacity",
                "canonical body exceeds complete draft admission",
            ));
        }
        Ok(value)
    }

    fn expressions(
        &mut self,
        head: &str,
        items: &[AuthoredExpression],
        env: &mut BTreeMap<String, Vec<String>>,
    ) -> Result<String, Diagnostic> {
        let mut text = format!("({head}");
        for item in items {
            append(
                &mut text,
                &format!(" {}", self.expression(item, env)?),
                self.maximum,
            )?;
        }
        text.push(')');
        Ok(text)
    }
}

fn visibility(v: DeclarationVisibility) -> &'static str {
    match v {
        DeclarationVisibility::Public => "public",
        DeclarationVisibility::Package => "package",
        DeclarationVisibility::Private => "private",
    }
}
fn unit_name(v: ResourceUnit) -> &'static str {
    match v {
        ResourceUnit::Bytes => "bytes",
        ResourceUnit::Items => "items",
        ResourceUnit::Calls => "calls",
        ResourceUnit::Milliseconds => "milliseconds",
        ResourceUnit::Tasks => "tasks",
    }
}
fn quote(value: &str, maximum: usize) -> Result<String, Diagnostic> {
    crate::platform::control::compact::render_structural_string(value, maximum)
}
