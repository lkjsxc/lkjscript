impl<'a> Producer<'a> {
    fn new(program: &'a hir::Program) -> Result<Self> {
        let mut function_ids = HashMap::with_capacity(program.functions.len());
        for (index, function) in program.functions.iter().enumerate() {
            let raw = u32::try_from(index)
                .map_err(|_| Error::msg("HIR memory-plan function count exceeds u32"))?;
            if function_ids
                .insert(function.binding, MemoryFunctionId::new(raw))
                .is_some()
            {
                return Err(Error::msg(
                    "HIR memory-plan producer found duplicate function binding",
                ));
            }
        }
        let mut producer = Self {
            program,
            function_ids,
            signatures: Vec::new(),
            functions: Vec::new(),
            entries: Vec::new(),
            uses: Vec::new(),
            loans: Vec::new(),
            constants: Vec::new(),
            calls: Vec::new(),
            obligations: Vec::new(),
            current_function: MemoryFunctionId::new(0),
            next_expression: 0,
            next_place: 0,
            expression_parents: BTreeMap::new(),
            work: MemoryPlanWork::default(),
        };
        producer.build_signatures()?;
        Ok(producer)
    }
    fn run(mut self) -> Result<HirMemoryPlan> {
        self.charge_functions(self.program.functions.len().saturating_add(1))?;
        for (index, function) in self.program.functions.iter().enumerate() {
            let id = MemoryFunctionId::new(
                u32::try_from(index)
                    .map_err(|_| Error::msg("HIR memory-plan function identity exceeds u32"))?,
            );
            self.build_function(id, function)?;
        }
        let main_id = MemoryFunctionId::new(
            u32::try_from(self.program.functions.len())
                .map_err(|_| Error::msg("HIR memory-plan main identity exceeds u32"))?,
        );
        self.build_main(main_id)?;
        self.finish_loans()?;
        let mut plan = HirMemoryPlan {
            schema: HIR_MEMORY_PLAN_SCHEMA,
            id: MemoryPlanId::from_bytes([0; 32]),
            functions: self.functions,
            entries: self.entries,
            uses: self.uses,
            loans: self.loans,
            constants: self.constants,
            calls: self.calls,
            obligations: self.obligations,
            drop_glues: drop_glues(),
            work: self.work,
        };
        plan.id = compute_plan_id(&plan)?;
        Ok(plan)
    }
    fn build_signatures(&mut self) -> Result<()> {
        for (index, function) in self.program.functions.iter().enumerate() {
            let function_id = MemoryFunctionId::new(
                u32::try_from(index)
                    .map_err(|_| Error::msg("HIR memory signature identity exceeds u32"))?,
            );
            let binding = self.program.binding(function.binding).ok_or_else(|| {
                Error::msg("HIR memory signature references unknown function binding")
            })?;
            let result_ty = function_result_type(&binding.ty)?;
            let mut parameters = Vec::with_capacity(function.params.len());
            for parameter in &function.params {
                let parameter_ty = &self
                    .program
                    .binding(*parameter)
                    .ok_or_else(|| {
                        Error::msg("HIR memory signature references unknown parameter binding")
                    })?
                    .ty;
                parameters.push(parameter_mode(
                    parameter_ty,
                    resource_parameter_consumed(&function.body, *parameter),
                ));
            }
            self.signatures.push(FunctionMemorySignature {
                function: function_id,
                parameters,
                result: result_mode(result_ty),
            });
        }
        let main_id = MemoryFunctionId::new(
            u32::try_from(self.program.functions.len())
                .map_err(|_| Error::msg("HIR main memory signature identity exceeds u32"))?,
        );
        self.signatures.push(FunctionMemorySignature {
            function: main_id,
            parameters: self
                .program
                .main
                .param_types
                .iter()
                .map(|ty| parameter_mode(ty, false))
                .collect(),
            result: result_mode(&self.program.main.return_type),
        });
        Ok(())
    }
    fn build_function(&mut self, id: MemoryFunctionId, function: &hir::Function) -> Result<()> {
        self.current_function = id;
        self.next_place = 0;
        let signature = self.signature(id)?.clone();
        let mut parameter_entries = Vec::with_capacity(function.params.len());
        for (index, ((binding, place), mode)) in function
            .params
            .iter()
            .copied()
            .zip(function.param_places.iter().copied())
            .zip(signature.parameters.iter().copied())
            .enumerate()
        {
            let ty = self.binding_type(binding)?.clone();
            let parameter_entry = self.add_entry(
                MemorySubject::Parameter {
                    function: id,
                    index: u32::try_from(index)
                        .map_err(|_| Error::msg("HIR memory parameter index exceeds u32"))?,
                    binding: binding.raw(),
                    place: place.raw(),
                },
                &ty,
                0,
                MemoryEscape::Caller,
                MemoryOrigin {
                    source: function.origin.raw(),
                    expression: None,
                },
            )?;
            parameter_entries.push(parameter_entry);
            self.add_place(
                id,
                binding,
                place.raw(),
                &ty,
                function.origin.raw(),
                mode == MemoryParameterMode::Consume,
            )?;
        }
        let result_ty = function_result_type(self.binding_type(function.binding)?)?.clone();
        let result_entry = self.add_entry(
            MemorySubject::Result { function: id },
            &result_ty,
            function.summary.bits(),
            MemoryEscape::Returned,
            MemoryOrigin {
                source: function.origin.raw(),
                expression: None,
            },
        )?;
        let body = self.walk_expr(&function.body, None, 0, MemoryEscape::Returned, None)?;
        let name = self
            .program
            .binding(function.binding)
            .ok_or_else(|| Error::msg("HIR memory plan lost function binding"))?
            .name
            .clone();
        self.functions.push(FunctionMemoryPlan {
            id,
            name,
            binding: Some(function.binding.raw()),
            source: function.origin.raw(),
            signature,
            parameter_entries,
            result_entry,
            body,
        });
        Ok(())
    }
}
