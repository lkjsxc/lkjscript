use crate::analyze::*;

impl Analyzer {
    pub(in crate::analyze) fn resolve_enum_type(
        &self,
        ty: &Type,
        parameters: &[String],
    ) -> std::result::Result<Type, String> {
        Ok(match ty {
            Type::Enum {
                name, arguments, ..
            } => {
                let (id, declared) = self
                    .enum_headers
                    .get(name)
                    .ok_or_else(|| format!("unknown enum type {name}"))?;
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
                        "enum type {name} cannot be instantiated with an ownership-bearing type"
                    ));
                }
                Type::Enum {
                    id: *id,
                    name: name.clone(),
                    arguments,
                }
            }
            Type::Param(name) if !parameters.iter().any(|parameter| parameter == name) => {
                return Err(format!("unbound type parameter {name}"));
            }
            Type::Owned(inner) => Type::Owned(Box::new(self.resolve_enum_type(inner, parameters)?)),
            Type::Ref(inner) => Type::Ref(Box::new(self.resolve_enum_type(inner, parameters)?)),
            Type::RefMut(inner) => {
                Type::RefMut(Box::new(self.resolve_enum_type(inner, parameters)?))
            }
            Type::List(inner) => Type::List(Box::new(self.resolve_enum_type(inner, parameters)?)),
            Type::Option(inner) => {
                Type::Option(Box::new(self.resolve_enum_type(inner, parameters)?))
            }
            Type::Result(ok, error) => Type::Result(
                Box::new(self.resolve_enum_type(ok, parameters)?),
                Box::new(self.resolve_enum_type(error, parameters)?),
            ),
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
        let mut work = 0_usize;
        for definition in &self.enums {
            let mut path = Vec::new();
            self.walk_enum(definition.id, 0, &mut path, &mut work)
                .map_err(|message| self.error(definition.origin, message))?;
        }
        Ok(())
    }

    fn walk_enum(
        &self,
        id: EnumId,
        depth: usize,
        path: &mut Vec<EnumId>,
        work: &mut usize,
    ) -> std::result::Result<(), String> {
        if path.contains(&id) {
            return Ok(());
        }
        if depth > ENUM_RECURSION_MAX_DEPTH {
            return Err(format!(
                "enum recursion depth exceeds {ENUM_RECURSION_MAX_DEPTH}"
            ));
        }
        *work = work
            .checked_add(1)
            .ok_or_else(|| "enum recursion work overflow".to_string())?;
        if *work > ENUM_RECURSION_MAX_WORK {
            return Err(format!(
                "enum recursion work exceeds {ENUM_RECURSION_MAX_WORK}"
            ));
        }
        let definition = self
            .enums
            .iter()
            .find(|definition| definition.id == id)
            .ok_or_else(|| "enum recursion references unknown EnumId".to_string())?;
        path.push(id);
        for field in definition
            .variants
            .iter()
            .flat_map(|variant| &variant.fields)
        {
            self.walk_type(&field.ty, depth, path, work)?;
        }
        path.pop();
        Ok(())
    }

    fn walk_type(
        &self,
        ty: &Type,
        depth: usize,
        path: &mut Vec<EnumId>,
        work: &mut usize,
    ) -> std::result::Result<(), String> {
        match ty {
            Type::Enum { id, arguments, .. } => {
                self.walk_enum(*id, depth.saturating_add(1), path, work)?;
                for argument in arguments {
                    self.walk_type(argument, depth, path, work)?;
                }
            }
            Type::Owned(inner)
            | Type::Ref(inner)
            | Type::RefMut(inner)
            | Type::List(inner)
            | Type::Option(inner) => {
                self.walk_type(inner, depth, path, work)?;
            }
            Type::Result(ok, error) => {
                self.walk_type(ok, depth, path, work)?;
                self.walk_type(error, depth, path, work)?;
            }
            Type::Fn { params, ret } => {
                for parameter in params {
                    self.walk_type(parameter, depth, path, work)?;
                }
                self.walk_type(ret, depth, path, work)?;
            }
            Type::Forall { body, .. } => self.walk_type(body, depth, path, work)?,
            _ => {}
        }
        Ok(())
    }
}
