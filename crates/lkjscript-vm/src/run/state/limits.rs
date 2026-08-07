use super::*;

pub(crate) struct Interruption {
    deadline: Option<Instant>,
    cancellation: Option<std::sync::Arc<dyn lkjscript_host::Cancellation>>,
}

impl Interruption {
    pub(crate) fn check(&self) -> Result<()> {
        if self
            .deadline
            .is_some_and(|deadline| Instant::now() >= deadline)
        {
            return Err(Error::deadline("execution wall deadline exceeded"));
        }
        if let Some(cancellation) = &self.cancellation {
            cancellation
                .check()
                .map_err(|error| Error::host(format!("execution cancellation: {error}")))?;
        }
        Ok(())
    }
}

impl<'a, J: RuntimeTier> Vm<'a, J> {
    pub(crate) fn check_runtime_limits(&mut self) -> Result<()> {
        if let Some(policy) = self.config.limited_policy() {
            if self.stack.len() > policy.max_stack_values {
                return Err(Error::resource(
                    ResourceLimitKind::StackValues,
                    "VM stack value limit exceeded",
                ));
            }
            if self.frames.len() > policy.max_frames {
                return Err(Error::resource(
                    ResourceLimitKind::FrameDepth,
                    "VM frame depth limit exceeded",
                ));
            }
            if self
                .list_allocations
                .saturating_add(self.region_product_allocations)
                > policy.max_allocations
            {
                return Err(Error::resource(
                    ResourceLimitKind::Allocations,
                    "VM aggregate allocation limit exceeded",
                ));
            }
            let list_bytes =
                usize::try_from(self.list_reserved_bytes_estimate()).unwrap_or(usize::MAX);
            let region_bytes = self
                .region_products
                .as_ref()
                .map_or(0, |arena| arena.metrics().reserved_bytes_estimate);
            let runtime_bytes =
                list_bytes.saturating_add(usize::try_from(region_bytes).unwrap_or(usize::MAX));
            if runtime_bytes > policy.max_heap_bytes {
                return Err(Error::resource(
                    ResourceLimitKind::HeapBytes,
                    "VM deterministic runtime byte limit exceeded",
                ));
            }
            if self.resources.allocated_handle_slots() > policy.max_handles {
                return Err(Error::resource(
                    ResourceLimitKind::Handles,
                    "VM handle limit exceeded",
                ));
            }
        }
        if self.resources.limit_exceeded() {
            return Err(Error::resource(
                ResourceLimitKind::Handles,
                "VM handle representation exhausted",
            ));
        }
        self.check_interruption()
    }

    pub(crate) fn interruption(&self) -> Result<Interruption> {
        let deadline = self
            .config
            .wall_time()
            .map(|limit| {
                self.started.checked_add(limit).ok_or_else(|| {
                    Error::host("execution wall deadline exceeds monotonic clock range")
                })
            })
            .transpose()?;
        let interruption = Interruption {
            deadline,
            cancellation: self.inputs.host.cancellation.clone(),
        };
        interruption.check()?;
        Ok(interruption)
    }

    pub(crate) fn check_interruption(&self) -> Result<()> {
        self.interruption().map(|_| ())
    }

    pub(crate) fn check_deadline(&self) -> Result<()> {
        if self
            .config
            .wall_time()
            .is_some_and(|limit| self.started.elapsed() >= limit)
        {
            return Err(Error::deadline("execution wall deadline exceeded"));
        }
        Ok(())
    }

    pub(crate) fn remaining_wall_time(&self) -> Result<Option<Duration>> {
        let Some(limit) = self.config.wall_time() else {
            return Ok(None);
        };
        let elapsed = self.started.elapsed();
        limit
            .checked_sub(elapsed)
            .filter(|remaining| !remaining.is_zero())
            .map(Some)
            .ok_or_else(|| Error::deadline("execution wall deadline exceeded"))
    }

    pub(crate) fn ensure_host_deadline_support(
        &self,
        operation: &str,
        hard_deadline_supported: bool,
    ) -> Result<()> {
        if self.config.require_hard_deadline()
            && self.config.wall_time().is_some()
            && !hard_deadline_supported
        {
            return Err(Error::host(format!(
                "{operation}: hard wall deadline is unsupported by the current host wrapper"
            )));
        }
        Ok(())
    }

    pub(crate) fn deadline_timeout_ms(&self) -> Result<Option<i32>> {
        let Some(remaining) = self.remaining_wall_time()? else {
            return Ok(None);
        };
        let milliseconds = remaining.as_millis().max(1);
        Ok(Some(i32::try_from(milliseconds).unwrap_or(i32::MAX)))
    }

    pub(crate) fn remaining_output_capacity(&self) -> Result<Option<usize>> {
        self.config
            .max_output_bytes()
            .map(|maximum| {
                maximum.checked_sub(self.output_bytes).ok_or_else(|| {
                    Error::resource(
                        ResourceLimitKind::OutputBytes,
                        "VM output byte counter exceeds configured policy",
                    )
                })
            })
            .transpose()
    }

    pub(crate) fn record_output(&mut self, bytes: usize) -> Result<()> {
        let Some(maximum) = self.config.max_output_bytes() else {
            return Ok(());
        };
        let total = self.output_bytes.checked_add(bytes).ok_or_else(|| {
            Error::resource(
                ResourceLimitKind::OutputBytes,
                "VM output byte counter overflow",
            )
        })?;
        if total > maximum {
            return Err(Error::resource(
                ResourceLimitKind::OutputBytes,
                "VM output byte limit exceeded",
            ));
        }
        self.output_bytes = total;
        Ok(())
    }
}
