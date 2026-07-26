use crate::*;

impl JitSession {
    pub(super) fn prepare_native_group(
        &mut self,
        root: FunctionId,
    ) -> Result<(lower::LoweredGroup, Duration, super::cache::CacheAttempt), EngineError> {
        let compile_started = Instant::now();
        let (cached, mut cache) = self.cached_lowering(root);
        let (lowered, lowering_and_encoding, publish) = if let Some(lowered) = cached {
            (lowered, Duration::ZERO, false)
        } else {
            let lowering_started = Instant::now();
            let lowered = match &self.program {
                ProgramAuthority::Baseline(program) => {
                    lower::lower_baseline_group(program, root, self.config.backend_limits)?
                }
                ProgramAuthority::Optimizing(program) => {
                    lower::lower_optimizing_group(program, root, self.config.backend_limits)?
                }
            };
            (lowered, lowering_started.elapsed(), true)
        };
        self.check_cache_compile_time(root, compile_started, "native compilation")?;
        if publish {
            self.publish_cached_image(&mut cache, &lowered.image);
        }
        self.check_cache_compile_time(root, compile_started, "native cache publication")?;
        Ok((lowered, lowering_and_encoding, cache))
    }

    fn check_cache_compile_time(
        &self,
        root: FunctionId,
        started: Instant,
        phase: &str,
    ) -> Result<(), EngineError> {
        let elapsed = started.elapsed();
        if self.optimization_time.saturating_add(elapsed) > self.config.max_object_compile_time
            || self.total_compile_time.saturating_add(elapsed) > self.config.max_total_compile_time
        {
            return Err(EngineError::new(
                FailureCode::CompileWallTime,
                Some(root),
                format!("{phase} wall-time budget exceeded"),
            ));
        }
        Ok(())
    }
}
