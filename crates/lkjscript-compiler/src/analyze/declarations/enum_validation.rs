use crate::analyze::*;

impl Analyzer {
    pub(in crate::analyze) fn resolve_enum_type(
        &self,
        ty: &Type,
        parameters: &[String],
    ) -> std::result::Result<Type, String> {
        crate::stack::grow(|| self.resolve_enum_type_inner(ty, parameters))
    }

    fn resolve_enum_type_inner(
        &self,
        ty: &Type,
        parameters: &[String],
    ) -> std::result::Result<Type, String> {
        Ok(match ty {
            Type::Enum { id, arguments } => {
                let (name, (_, declared)) = self
                    .enum_headers
                    .iter()
                    .find(|(_, (expected, _))| expected == id)
                    .ok_or_else(|| "unknown enum type identity".to_owned())?;
                if arguments.len() != declared.len() {
                    return Err(format!(
                        "enum type {name} requires {} explicit invariant arguments, got {}",
                        declared.len(),
                        arguments.len()
                    ));
                }
                let arguments = arguments
                    .iter()
                    .map(|argument| self.resolve_enum_type(argument, parameters))
                    .collect::<std::result::Result<Vec<_>, _>>()?;
                if arguments.iter().any(contains_ownership_type) {
                    return Err(format!(
                        "enum type {name} cannot be instantiated with an ownership/reference-bearing type"
                    ));
                }
                Type::Enum { id: *id, arguments }
            }
            Type::Param(name) if !parameters.iter().any(|parameter| parameter == name) => {
                return Err(format!("unbound type parameter {name}"));
            }
            Type::List(inner) => Type::List(Box::new(self.resolve_enum_type(inner, parameters)?)),
            Type::Fn { params, ret } => Type::Fn {
                params: params
                    .iter()
                    .map(|parameter| self.resolve_enum_type(parameter, parameters))
                    .collect::<std::result::Result<Vec<_>, _>>()?,
                ret: Box::new(self.resolve_enum_type(ret, parameters)?),
            },
            Type::Forall { vars, body } => {
                let mut nested = parameters.to_vec();
                nested.extend(vars.iter().cloned());
                Type::Forall {
                    vars: vars.clone(),
                    body: Box::new(self.resolve_enum_type(body, &nested)?),
                }
            }
            other => other.clone(),
        })
    }

    pub(in crate::analyze) fn resolve_function_enum_types(
        &self,
        parsed: &mut ParsedFunction<'_>,
    ) -> std::result::Result<(), String> {
        parsed.signature_params = parsed
            .signature_params
            .iter()
            .map(|ty| self.resolve_enum_type(ty, &parsed.forall_vars))
            .collect::<std::result::Result<Vec<_>, _>>()?;
        parsed.param_types = parsed
            .param_types
            .iter()
            .map(|ty| self.resolve_enum_type(ty, &parsed.forall_vars))
            .collect::<std::result::Result<Vec<_>, _>>()?;
        parsed.signature_return =
            self.resolve_enum_type(&parsed.signature_return, &parsed.forall_vars)?;
        Ok(())
    }

    pub(in crate::analyze) fn validate_enum_recursion(&self) -> Result<()> {
        enum Work<'a> {
            Enum(EnumId),
            Type(&'a Type),
        }

        let definitions: HashMap<_, _> = self
            .enums
            .iter()
            .map(|definition| (definition.id, definition))
            .collect();
        let mut visited = HashSet::new();
        let mut work = Vec::new();
        work.try_reserve(self.enums.len())
            .map_err(|_| Error::msg("enum recursion work allocation failed"))?;
        work.extend(
            self.enums
                .iter()
                .rev()
                .map(|definition| Work::Enum(definition.id)),
        );
        let mut observed_work = 0_u64;
        while let Some(item) = work.pop() {
            observed_work = observed_work
                .checked_add(1)
                .ok_or_else(|| Error::msg("enum recursion work overflow"))?;
            match item {
                Work::Enum(id) => {
                    if !visited.insert(id) {
                        continue;
                    }
                    let definition = definitions
                        .get(&id)
                        .copied()
                        .ok_or_else(|| Error::msg("enum recursion references unknown EnumId"))?;
                    let field_count = definition
                        .variants
                        .iter()
                        .map(|variant| variant.fields.len())
                        .try_fold(0_usize, usize::checked_add)
                        .ok_or_else(|| Error::msg("enum recursion field count overflow"))?;
                    work.try_reserve(field_count)
                        .map_err(|_| Error::msg("enum recursion work allocation failed"))?;
                    for field in definition
                        .variants
                        .iter()
                        .rev()
                        .flat_map(|variant| variant.fields.iter().rev())
                    {
                        work.push(Work::Type(&field.ty));
                    }
                }
                Work::Type(ty) => match ty {
                    Type::Enum { id, arguments, .. } => {
                        let additional = arguments
                            .len()
                            .checked_add(1)
                            .ok_or_else(|| Error::msg("enum recursion work size overflow"))?;
                        work.try_reserve(additional)
                            .map_err(|_| Error::msg("enum recursion work allocation failed"))?;
                        work.extend(arguments.iter().rev().map(Work::Type));
                        work.push(Work::Enum(*id));
                    }
                    Type::List(inner) => work.push(Work::Type(inner)),
                    Type::Fn { params, ret } => {
                        let additional = params
                            .len()
                            .checked_add(1)
                            .ok_or_else(|| Error::msg("enum recursion work size overflow"))?;
                        work.try_reserve(additional)
                            .map_err(|_| Error::msg("enum recursion work allocation failed"))?;
                        work.push(Work::Type(ret));
                        work.extend(params.iter().rev().map(Work::Type));
                    }
                    Type::Forall { body, .. } => work.push(Work::Type(body)),
                    _ => {}
                },
            }
        }
        let _ = observed_work;
        Ok(())
    }
}
