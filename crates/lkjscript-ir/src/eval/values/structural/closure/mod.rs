use crate::{EnumId, ProductId, Program, SsaType};

use super::{AggregateMode, ClosureClass};

pub(crate) struct ClosureReconstructor<'a> {
    program: &'a Program,
    active: Vec<SsaType>,
}

impl<'a> ClosureReconstructor<'a> {
    pub(crate) const fn new(program: &'a Program) -> Self {
        Self {
            program,
            active: Vec::new(),
        }
    }

    pub(crate) fn aggregate_mode(mut self, ty: &SsaType) -> Result<AggregateMode, String> {
        match self.classify(ty)? {
            ClosureClass::Dynamic => Ok(AggregateMode::Structural),
            ClosureClass::Legacy { .. } => Ok(AggregateMode::Legacy),
            ClosureClass::Resource => Ok(AggregateMode::ResourceAdapter),
            _ => Err("evaluator aggregate classification did not produce an aggregate".into()),
        }
    }

    pub(crate) fn classify(&mut self, ty: &SsaType) -> Result<ClosureClass, String> {
        match ty {
            SsaType::Unit | SsaType::Bool | SsaType::I64 | SsaType::F64 => Ok(ClosureClass::Inline),
            SsaType::Symbol => Ok(ClosureClass::Static),
            SsaType::Str | SsaType::Path | SsaType::Bytes | SsaType::ByteVector => {
                Ok(ClosureClass::Dynamic)
            }
            SsaType::Capability(_) => {
                Err("capability aggregate awaits an exact evaluator structural leaf adapter".into())
            }
            SsaType::Resource(_) => Ok(ClosureClass::Resource),
            SsaType::StructuralDestination(_) => {
                Err("private destination cannot enter evaluator structural closure".into())
            }
            SsaType::List(inner) => {
                let inner = self.classify(inner)?;
                Ok(ClosureClass::Legacy {
                    dynamic_reachable: inner.dynamic_reachable(),
                })
            }
            SsaType::Product(id) => self.product(*id),
            SsaType::Enum { id, arguments } => self.enumeration(*id, arguments),
            SsaType::ByteSlice | SsaType::ByteSliceMut => {
                Err("borrowed byte views cannot enter evaluator aggregates".into())
            }
            SsaType::Function(_) => {
                Err("function capture cannot enter evaluator structural closure".into())
            }
            SsaType::TypeParameter(_) => {
                Err("unknown type parameter in evaluator structural closure".into())
            }
        }
    }

    fn product(&mut self, id: ProductId) -> Result<ClosureClass, String> {
        let concrete = SsaType::Product(id);
        if self.active.contains(&concrete) {
            return Ok(ClosureClass::Dynamic);
        }
        self.enter(concrete);
        let fields = self
            .program
            .products
            .iter()
            .find(|item| item.id == id)
            .ok_or_else(|| "evaluator product metadata is missing".to_owned())?
            .fields
            .iter()
            .map(|field| field.ty.clone())
            .collect::<Vec<_>>();
        let result = self.fields(&fields);
        self.active.pop();
        result
    }

    fn enumeration(&mut self, id: EnumId, arguments: &[SsaType]) -> Result<ClosureClass, String> {
        let concrete = SsaType::Enum {
            id,
            arguments: arguments.to_vec(),
        };
        if self.active.contains(&concrete) {
            return Ok(ClosureClass::Dynamic);
        }
        self.enter(concrete);
        let definition = self
            .program
            .enums
            .iter()
            .find(|item| item.id == id)
            .ok_or_else(|| "evaluator enum metadata is missing".to_owned())?;
        if definition.type_parameters.len() != arguments.len() {
            self.active.pop();
            return Err("evaluator enum substitution arity mismatch".into());
        }
        let mut fields = Vec::new();
        for variant in &definition.variants {
            for field in &variant.fields {
                fields.push(substitute(
                    &field.ty,
                    &definition.type_parameters,
                    arguments,
                )?);
            }
        }
        let result = self.fields(&fields);
        self.active.pop();
        result
    }

    fn fields(&mut self, fields: &[SsaType]) -> Result<ClosureClass, String> {
        let mut dynamic = false;
        let mut legacy = false;
        let mut resource = false;
        for field in fields {
            match self.classify(field)? {
                ClosureClass::Dynamic => dynamic = true,
                ClosureClass::Legacy { dynamic_reachable } => {
                    legacy = true;
                    dynamic |= dynamic_reachable;
                }
                ClosureClass::Resource => resource = true,
                ClosureClass::Inline | ClosureClass::Static => {}
            }
        }
        if legacy && dynamic {
            return Err("mixed legacy and structural aggregate closure".into());
        }
        if resource && legacy {
            return Err("resource aggregate cannot contain a legacy closure".into());
        }
        Ok(if resource {
            ClosureClass::Resource
        } else if legacy {
            ClosureClass::Legacy {
                dynamic_reachable: false,
            }
        } else {
            ClosureClass::Dynamic
        })
    }

    fn enter(&mut self, concrete_type: SsaType) {
        self.active.push(concrete_type);
    }
}

mod substitution;
pub(crate) use substitution::substitute;
