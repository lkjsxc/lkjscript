use super::*;

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct StaticBytesIdentity(u32);

impl StaticBytesIdentity {
    #[must_use]
    pub const fn new(index: u32) -> Self {
        Self(index)
    }

    #[must_use]
    pub const fn index(self) -> u32 {
        self.0
    }

    #[must_use]
    pub const fn opaque_word(self) -> u64 {
        self.0 as u64 + 1
    }
}

#[derive(Debug)]
pub struct MachinePlanBuilder {
    pub(super) plan: u64,
    pub(super) functions: Vec<FunctionDeclaration>,
    pub(super) static_bytes: Vec<Box<[u8]>>,
}

impl MachinePlanBuilder {
    #[must_use]
    pub fn new() -> Self {
        Self {
            plan: NEXT_PLAN_ID.fetch_add(1, Ordering::Relaxed),
            functions: Vec::new(),
            static_bytes: Vec::new(),
        }
    }

    pub fn intern_static_bytes(&mut self, bytes: &[u8]) -> Result<StaticBytesIdentity, PlanError> {
        if let Some(index) = self
            .static_bytes
            .iter()
            .position(|candidate| candidate.as_ref() == bytes)
        {
            return u32::try_from(index)
                .map(StaticBytesIdentity::new)
                .map_err(|_| PlanError::TooManyItems);
        }
        let index = u32::try_from(self.static_bytes.len()).map_err(|_| PlanError::TooManyItems)?;
        self.static_bytes.push(bytes.to_vec().into_boxed_slice());
        Ok(StaticBytesIdentity::new(index))
    }

    pub fn declare_function(
        &mut self,
        source_function: SourceFunctionId,
        signature: Signature,
    ) -> Result<FunctionId, PlanError> {
        let index = u32::try_from(self.functions.len()).map_err(|_| PlanError::TooManyItems)?;
        let id = FunctionId {
            plan: self.plan,
            index,
        };
        self.functions.push(FunctionDeclaration {
            id,
            signature,
            source_function,
            body: None,
        });
        Ok(id)
    }

    pub fn function_builder(&self, function: FunctionId) -> Result<FunctionBuilder, PlanError> {
        let declaration = self.declaration(function)?;
        if declaration.body.is_some() {
            return Err(PlanError::FunctionAlreadyDefined);
        }
        Ok(FunctionBuilder::new(
            declaration.id,
            declaration.signature.clone(),
            declaration.source_function,
            self.functions
                .iter()
                .map(|item| (item.id, item.signature.clone()))
                .collect(),
        ))
    }

    pub fn define_function(&mut self, function: FunctionPlan) -> Result<(), PlanError> {
        let declaration = self.declaration_mut(function.id)?;
        if declaration.body.is_some() {
            return Err(PlanError::FunctionAlreadyDefined);
        }
        if declaration.signature != function.signature
            || declaration.source_function != function.source_function
        {
            return Err(PlanError::ForeignId("function definition"));
        }
        declaration.body = Some(function);
        Ok(())
    }

    pub fn verify(self, limits: BackendLimits) -> Result<VerifiedMachinePlan, NativeError> {
        verify_plan(self.plan, self.functions, self.static_bytes, limits)
    }

    fn declaration(&self, function: FunctionId) -> Result<&FunctionDeclaration, PlanError> {
        if function.plan != self.plan {
            return Err(PlanError::ForeignId("function ID"));
        }
        self.functions
            .get(function.index as usize)
            .filter(|item| item.id == function)
            .ok_or(PlanError::UnknownFunction)
    }

    fn declaration_mut(
        &mut self,
        function: FunctionId,
    ) -> Result<&mut FunctionDeclaration, PlanError> {
        if function.plan != self.plan {
            return Err(PlanError::ForeignId("function ID"));
        }
        self.functions
            .get_mut(function.index as usize)
            .filter(|item| item.id == function)
            .ok_or(PlanError::UnknownFunction)
    }
}

impl Default for MachinePlanBuilder {
    fn default() -> Self {
        Self::new()
    }
}
