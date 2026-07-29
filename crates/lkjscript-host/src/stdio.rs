use std::collections::VecDeque;
use std::io::{Read, Write};
use std::sync::{Arc, Mutex};

use crate::{HostError, HostResult, StdioProvider};

#[derive(Clone, Copy, Debug, Default)]
pub struct PortableStdio;

impl StdioProvider for PortableStdio {
    fn write(&self, bytes: &[u8]) -> HostResult<()> {
        std::io::stdout()
            .write_all(bytes)
            .map_err(|error| HostError::from_io("write standard output", error))
    }

    fn flush(&self) -> HostResult<()> {
        std::io::stdout()
            .flush()
            .map_err(|error| HostError::from_io("flush standard output", error))
    }

    fn read_byte(&self) -> HostResult<Option<u8>> {
        let mut byte = [0_u8; 1];
        match std::io::stdin().read(&mut byte) {
            Ok(0) => Ok(None),
            Ok(1) => Ok(Some(byte[0])),
            Ok(_) => Err(HostError::Io {
                operation: "read standard input".to_string(),
                message: "one-byte read returned oversized count".to_string(),
            }),
            Err(error) => Err(HostError::from_io("read standard input", error)),
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct BufferedStdio {
    state: Arc<Mutex<BufferedState>>,
}

#[derive(Debug, Default)]
struct BufferedState {
    input: VecDeque<u8>,
    output: Vec<u8>,
    flushes: u64,
}

impl BufferedStdio {
    pub fn with_input(input: impl Into<Vec<u8>>) -> Self {
        Self {
            state: Arc::new(Mutex::new(BufferedState {
                input: input.into().into(),
                output: Vec::new(),
                flushes: 0,
            })),
        }
    }

    pub fn output(&self) -> HostResult<Vec<u8>> {
        Ok(self.lock()?.output.clone())
    }

    pub fn flushes(&self) -> HostResult<u64> {
        Ok(self.lock()?.flushes)
    }

    pub fn drain_output(&self) -> HostResult<(Vec<u8>, u64)> {
        let mut state = self.lock()?;
        let output = std::mem::take(&mut state.output);
        let flushes = std::mem::take(&mut state.flushes);
        Ok((output, flushes))
    }

    fn lock(&self) -> HostResult<std::sync::MutexGuard<'_, BufferedState>> {
        self.state.lock().map_err(|_| HostError::Io {
            operation: "lock buffered stdio".to_string(),
            message: "provider state poisoned".to_string(),
        })
    }
}

impl StdioProvider for BufferedStdio {
    fn write(&self, bytes: &[u8]) -> HostResult<()> {
        self.lock()?.output.extend_from_slice(bytes);
        Ok(())
    }

    fn flush(&self) -> HostResult<()> {
        let mut state = self.lock()?;
        state.flushes = state.flushes.saturating_add(1);
        Ok(())
    }

    fn read_byte(&self) -> HostResult<Option<u8>> {
        Ok(self.lock()?.input.pop_front())
    }
}
