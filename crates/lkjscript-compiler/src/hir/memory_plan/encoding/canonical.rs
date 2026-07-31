use lkjscript_core::{Error, Result};

use super::super::*;

pub(super) trait Canonical {
    fn encode(&self, output: &mut Encoder) -> Result<()>;
}

pub(super) struct Encoder {
    bytes: Vec<u8>,
}

impl Encoder {
    pub(super) fn new(domain: &[u8]) -> Result<Self> {
        let mut output = Self { bytes: Vec::new() };
        output.bytes(domain)?;
        Ok(output)
    }

    pub(super) fn value<T: Canonical + ?Sized>(&mut self, value: &T) -> Result<()> {
        value.encode(self)
    }

    pub(super) fn tag(&mut self, tag: u8) -> Result<()> {
        self.reserve(1)?;
        self.bytes.push(tag);
        Ok(())
    }

    pub(super) fn bytes(&mut self, value: &[u8]) -> Result<()> {
        let length = u64::try_from(value.len())
            .map_err(|_| Error::msg("canonical memory encoding field exceeds u64"))?;
        self.reserve(
            8_usize
                .checked_add(value.len())
                .ok_or_else(|| Error::msg("canonical memory encoding field size overflow"))?,
        )?;
        self.bytes.extend_from_slice(&length.to_be_bytes());
        self.bytes.extend_from_slice(value);
        Ok(())
    }

    pub(super) fn finish(self) -> Vec<u8> {
        self.bytes
    }

    fn reserve(&mut self, additional: usize) -> Result<()> {
        self.bytes
            .try_reserve(additional)
            .map_err(|_| Error::msg("canonical memory encoding allocation failed"))
    }
}

macro_rules! canonical_struct {
    ($name:ty { $($field:ident),+ $(,)? }) => {
        impl Canonical for $name {
            fn encode(&self, output: &mut Encoder) -> Result<()> {
                let Self { $($field),+ } = self;
                $(output.value($field)?;)+
                Ok(())
            }
        }
    };
}

macro_rules! unit_enum {
    ($name:ty { $($variant:ident = $tag:expr),+ $(,)? }) => {
        impl Canonical for $name {
            fn encode(&self, output: &mut Encoder) -> Result<()> {
                output.tag(match self { $(Self::$variant => $tag,)+ })
            }
        }
    };
}

macro_rules! dense_ids {
    ($($name:ty),+ $(,)?) => {
        $(impl Canonical for $name {
            fn encode(&self, output: &mut Encoder) -> Result<()> {
                output.value(&self.raw())
            }
        })+
    };
}

dense_ids!(
    MemoryFunctionId,
    MemoryExpressionId,
    MemoryEntryId,
    MemoryUseId,
    MemoryConstantId,
    MemoryCallId,
    MemoryObligationId,
    MemoryDropGlueId,
    MemoryTypeFactId,
    MemoryDestinationId,
    MemoryBorrowScopeId,
    MemoryDropPathId,
);

impl Canonical for MemoryPlanId {
    fn encode(&self, output: &mut Encoder) -> Result<()> {
        output.bytes(&self.as_bytes())
    }
}

impl Canonical for MemoryWitnessId {
    fn encode(&self, output: &mut Encoder) -> Result<()> {
        output.bytes(&self.as_bytes())
    }
}

impl Canonical for str {
    fn encode(&self, output: &mut Encoder) -> Result<()> {
        output.bytes(self.as_bytes())
    }
}

impl Canonical for String {
    fn encode(&self, output: &mut Encoder) -> Result<()> {
        output.value(self.as_str())
    }
}

impl<T: Canonical> Canonical for Vec<T> {
    fn encode(&self, output: &mut Encoder) -> Result<()> {
        output.value(
            &u64::try_from(self.len())
                .map_err(|_| Error::msg("canonical memory encoding record count exceeds u64"))?,
        )?;
        for value in self {
            output.value(value)?;
        }
        Ok(())
    }
}

impl<T: Canonical> Canonical for Option<T> {
    fn encode(&self, output: &mut Encoder) -> Result<()> {
        match self {
            Some(value) => {
                output.tag(1)?;
                output.value(value)
            }
            None => output.tag(0),
        }
    }
}

impl<T: Canonical> Canonical for Box<T> {
    fn encode(&self, output: &mut Encoder) -> Result<()> {
        output.value(self.as_ref())
    }
}

impl Canonical for [u8; 32] {
    fn encode(&self, output: &mut Encoder) -> Result<()> {
        output.bytes(self)
    }
}

macro_rules! fixed_integer {
    ($($name:ty),+ $(,)?) => {
        $(impl Canonical for $name {
            fn encode(&self, output: &mut Encoder) -> Result<()> {
                output.reserve(std::mem::size_of::<Self>())?;
                output.bytes.extend_from_slice(&self.to_be_bytes());
                Ok(())
            }
        })+
    };
}

fixed_integer!(u16, u32, u64, i64);

impl Canonical for u8 {
    fn encode(&self, output: &mut Encoder) -> Result<()> {
        output.tag(*self)
    }
}

impl Canonical for bool {
    fn encode(&self, output: &mut Encoder) -> Result<()> {
        output.tag(u8::from(*self))
    }
}

mod authority;
mod modes;
mod obligations;
mod records;
mod types;
mod witness;
