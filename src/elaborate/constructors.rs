//! Constructors use the resolved nominal declaration, including transparent aliases.
use super::{Builder, Result, a, err};
use std::collections::{BTreeMap, BTreeSet};

pub(super) struct ConstructorTarget {
    pub(super) declaration: a::DeclarationId,
    pub(super) substitutions: BTreeMap<String, usize>,
    expected: Option<usize>,
    alias: Option<(usize, BTreeMap<String, usize>)>,
}

fn parameters(source: &a::Program, declaration: a::DeclarationId) -> Vec<a::GenericParam> {
    match source.declarations[declaration].item {
        a::Item::Struct(index) => source.structs[index].generics.clone(),
        a::Item::Enum(index) => source.enums[index].generics.clone(),
        a::Item::Alias(index) => source.aliases[index].generics.clone(),
        _ => Vec::new(),
    }
}

impl Builder<'_, '_> {
    pub(super) fn nominal_declaration(&self, ty: usize, span: a::Span) -> Result<a::DeclarationId> {
        let (name, _) = self.compiler.applications.get(&ty).ok_or_else(|| {
            err(
                span,
                "E_TYPE",
                "constructor requires a nominal struct or enum",
            )
        })?;
        self.compiler
            .declaration_names
            .get(name)
            .copied()
            .ok_or_else(|| err(span, "E_TYPE", "nominal declaration identity is missing"))
    }

    fn constructor_arguments(
        &mut self,
        declaration: a::DeclarationId,
        arguments: &[a::TypeId],
        environment: &BTreeMap<String, usize>,
        infer_omitted: bool,
        span: a::Span,
    ) -> Result<BTreeMap<String, usize>> {
        let parameters: Vec<_> = parameters(self.compiler.source, declaration)
            .into_iter()
            .filter(|parameter| !parameter.lifetime)
            .collect();
        let arguments: Vec<_> = arguments.iter().copied().filter(|id| {
            !matches!(&self.compiler.source.types[*id].kind, a::Type::Path(path) if path.name.starts_with('\''))
        }).collect();
        if arguments.len() != parameters.len() && !(infer_omitted && arguments.is_empty()) {
            return Err(err(
                span,
                "E_TYPE",
                "wrong number of constructor type arguments",
            ));
        }
        let mut bindings = BTreeMap::new();
        for (parameter, argument) in parameters.iter().zip(arguments) {
            if let Some(ty) = self.compiler.resolve_if_known(argument, environment)? {
                bindings.insert(parameter.name.clone(), ty);
            }
        }
        Ok(bindings)
    }

    fn constrain_constructor(
        &self,
        declaration: a::DeclarationId,
        ty: usize,
        substitutions: &mut BTreeMap<String, usize>,
        span: a::Span,
    ) -> Result<()> {
        if self.nominal_declaration(ty, span)? != declaration {
            return Err(err(
                span,
                "E_TYPE",
                "constructor names a different declaration",
            ));
        }
        let (_, arguments) = &self.compiler.applications[&ty];
        for (parameter, argument) in parameters(self.compiler.source, declaration)
            .iter()
            .filter(|parameter| !parameter.lifetime)
            .zip(arguments)
        {
            if substitutions
                .get(&parameter.name)
                .is_some_and(|previous| previous != argument)
            {
                return Err(err(
                    span,
                    "E_TYPE",
                    "constructor type arguments differ from expected type",
                ));
            }
            substitutions.insert(parameter.name.clone(), *argument);
        }
        Ok(())
    }

    pub(super) fn constructor_target(
        &mut self,
        path: &a::Path,
        expected: Option<usize>,
        span: a::Span,
    ) -> Result<Option<ConstructorTarget>> {
        let mut resolved = None;
        let (mut declaration, mut substitutions) = if let Some(declaration) = path.declaration {
            if !matches!(
                self.compiler.source.declarations[declaration].item,
                a::Item::Struct(_) | a::Item::Enum(_) | a::Item::Alias(_)
            ) {
                return Ok(None);
            }
            let caller = self.sub.clone();
            let bindings =
                self.constructor_arguments(declaration, &path.args, &caller, true, span)?;
            (declaration, bindings)
        } else if path.name == "Self" || path.name.starts_with("Self::") {
            let ty = self
                .sub
                .get("Self")
                .copied()
                .ok_or_else(|| err(span, "E_TYPE", "Self has no concrete constructor type"))?;
            resolved = Some(ty);
            (self.nominal_declaration(ty, span)?, BTreeMap::new())
        } else {
            return Ok(None);
        };
        let alias =
            if let a::Item::Alias(index) = self.compiler.source.declarations[declaration].item {
                Some((index, substitutions.clone()))
            } else {
                None
            };
        let mut aliases = BTreeSet::new();
        while let a::Item::Alias(index) = self.compiler.source.declarations[declaration].item {
            if !aliases.insert(declaration) {
                return Err(err(span, "E_RECURSION", "cyclic type alias constructor"));
            }
            let definition = self.compiler.source.aliases[index].clone();
            if matches!(&self.compiler.source.types[definition.ty].kind,
                a::Type::Path(path) if self.compiler.names.is_type_parameter(path, self.compiler.source.types[definition.ty].span))
            {
                if let Some(ty) = expected {
                    if self
                        .compiler
                        .resolve_if_known(definition.ty, &substitutions)?
                        .is_none()
                    {
                        self.infer_type(definition.ty, ty, &mut substitutions)?;
                    }
                }
            }
            if let Some(ty) = self
                .compiler
                .resolve_if_known(definition.ty, &substitutions)?
            {
                declaration = self.nominal_declaration(ty, span)?;
                substitutions = BTreeMap::new();
                resolved = Some(ty);
                break;
            }
            let node = self.compiler.source.types[definition.ty].clone();
            let a::Type::Path(target) = node.kind else {
                return Err(err(
                    span,
                    "E_TYPE",
                    "type alias does not name a constructible nominal type",
                ));
            };
            let Some(next) = target.declaration else {
                return Err(err(
                    span,
                    "E_TYPE",
                    "type alias constructor target needs type arguments or an expected type",
                ));
            };
            if !matches!(
                self.compiler.source.declarations[next].item,
                a::Item::Struct(_) | a::Item::Enum(_) | a::Item::Alias(_)
            ) {
                return Err(err(
                    span,
                    "E_TYPE",
                    "type alias does not name a constructible nominal type",
                ));
            }
            substitutions =
                self.constructor_arguments(next, &target.args, &substitutions, false, node.span)?;
            declaration = next;
        }
        if !self
            .compiler
            .names
            .declaration_visible(self.scope, declaration)
        {
            return Err(err(
                span,
                "E_PRIVACY",
                "type alias constructor targets a private declaration",
            ));
        }
        if let Some(ty) = resolved {
            self.constrain_constructor(declaration, ty, &mut substitutions, span)?;
        }
        Ok(Some(ConstructorTarget {
            declaration,
            substitutions,
            expected,
            alias,
        }))
    }

    pub(super) fn constrain_constructor_target(
        &self,
        target: &mut ConstructorTarget,
        span: a::Span,
    ) -> Result<()> {
        if let Some(ty) = target.expected {
            self.constrain_constructor(target.declaration, ty, &mut target.substitutions, span)?;
        }
        Ok(())
    }

    pub(super) fn finish_constructor(
        &mut self,
        target: ConstructorTarget,
        span: a::Span,
    ) -> Result<usize> {
        let definition = &self.compiler.source.declarations[target.declaration];
        let ty = self.compiler.named_type(
            &a::Path {
                declaration: Some(target.declaration),
                name: definition.name.clone(),
                args: Vec::new(),
            },
            &target.substitutions,
            span,
        )?;
        if target.expected.is_some_and(|expected| expected != ty) {
            return Err(err(
                span,
                "E_TYPE",
                "constructor result differs from expected type",
            ));
        }
        if let Some((index, mut substitutions)) = target.alias {
            let alias = self.compiler.source.aliases[index].clone();
            self.infer_type(alias.ty, ty, &mut substitutions)?;
            if alias.generics.iter().any(|parameter| {
                !parameter.lifetime && !substitutions.contains_key(&parameter.name)
            }) {
                return Err(err(
                    span,
                    "E_TYPE",
                    "type alias argument needs an annotation",
                ));
            }
            self.compiler.bounds(&alias.generics, &substitutions)?;
            if self.compiler.resolve(alias.ty, &substitutions)? != ty {
                return Err(err(
                    span,
                    "E_TYPE",
                    "inferred constructor fields differ from the type alias target; specify alias type arguments or annotate the field values",
                ));
            }
        }
        Ok(ty)
    }
}
