use super::*;

impl<'a, J: RuntimeTier> Vm<'a, J> {
    pub(crate) fn collect(&mut self) {
        let mut roots = self.globals.clone();
        roots.extend_from_slice(&self.stack);
        self.arena.collect(&roots);
    }

    pub(crate) fn check_runtime_limits(&mut self) -> Result<()> {
        if self.stack.len() > self.config.max_stack_values {
            return Err(Error::resource(
                ResourceLimitKind::StackValues,
                "VM stack value limit exceeded",
            ));
        }
        if self.frames.len() > self.config.max_frames {
            return Err(Error::resource(
                ResourceLimitKind::FrameDepth,
                "VM frame depth limit exceeded",
            ));
        }
        if self.arena.total_allocations() > self.config.max_allocations {
            return Err(Error::resource(
                ResourceLimitKind::Allocations,
                "VM aggregate allocation limit exceeded",
            ));
        }
        if self.arena.heap_bytes() > self.config.max_heap_bytes {
            self.collect();
            if self.arena.heap_bytes() > self.config.max_heap_bytes {
                return Err(Error::resource(
                    ResourceLimitKind::HeapBytes,
                    "VM live heap byte limit exceeded",
                ));
            }
        }
        if self.resources.limit_exceeded()
            || self.resources.allocated_handle_slots() > self.config.max_handles
        {
            return Err(Error::resource(
                ResourceLimitKind::Handles,
                "VM handle limit exceeded",
            ));
        }
        self.check_deadline()
    }

    pub(crate) fn check_deadline(&self) -> Result<()> {
        if self
            .config
            .wall_time
            .is_some_and(|limit| self.started.elapsed() >= limit)
        {
            return Err(Error::deadline("execution wall deadline exceeded"));
        }
        Ok(())
    }

    pub(crate) fn remaining_wall_time(&self) -> Result<Option<Duration>> {
        let Some(limit) = self.config.wall_time else {
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
        if self.config.require_hard_deadline
            && self.config.wall_time.is_some()
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

    pub(crate) fn wait_for_stdin(&self) -> Result<()> {
        let Some(timeout) = self.deadline_timeout_ms()? else {
            return Ok(());
        };
        let ready = lkjscript_sys::poll_fd(lkjscript_sys::STDIN_FD, timeout)
            .map_err(|error| Error::host(format!("read-byte poll: {error}")))?;
        if ready {
            Ok(())
        } else {
            Err(Error::deadline(
                "execution wall deadline exceeded during read-byte",
            ))
        }
    }

    pub(crate) fn record_output(&mut self, bytes: usize) -> Result<()> {
        let total = self.output_bytes.checked_add(bytes).ok_or_else(|| {
            Error::resource(
                ResourceLimitKind::OutputBytes,
                "VM output byte counter overflow",
            )
        })?;
        if total > self.config.max_output_bytes {
            return Err(Error::resource(
                ResourceLimitKind::OutputBytes,
                "VM output byte limit exceeded",
            ));
        }
        self.output_bytes = total;
        Ok(())
    }
}
