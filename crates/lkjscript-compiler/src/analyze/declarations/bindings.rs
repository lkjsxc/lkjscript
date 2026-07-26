use crate::analyze::*;

impl Analyzer {
    pub(in crate::analyze) fn add_global(
        &mut self,
        origin: SourceId,
        name: String,
        kind: BindingKind,
        ty: Type,
    ) -> Result<BindingId> {
        if Operation::from_name(&name).is_some()
            || is_contextual_name(&name)
            || is_control_name(&name)
        {
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

    pub(in crate::analyze) fn add_binding(
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

    pub(in crate::analyze) fn binding(&self, id: BindingId) -> Result<&Binding> {
        let Some(index) = id.index() else {
            return Err(Error::msg("HIR BindingId cannot index this platform"));
        };
        self.bindings
            .get(index)
            .ok_or_else(|| Error::msg(format!("unknown HIR BindingId {}", id.raw())))
    }

    pub(in crate::analyze) fn diagnostic(
        &self,
        origin: SourceId,
        diagnostic: AnalysisDiagnostic,
    ) -> Error {
        self.error(origin, diagnostic.render_human())
    }

    pub(in crate::analyze) fn error(&self, origin: SourceId, message: impl Into<String>) -> Error {
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

pub(in crate::analyze) fn record_global(
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
