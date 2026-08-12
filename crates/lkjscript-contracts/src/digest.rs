use std::fmt;

#[cfg(test)]
use crate::{canonical_bytes, sha256, ContractDescriptor, ContractError};

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ContractDigest([u8; 32]);

impl ContractDigest {
    #[cfg(test)]
    pub(crate) fn of(descriptor: &ContractDescriptor) -> Result<Self, ContractError> {
        Ok(Self(sha256(&canonical_bytes(descriptor)?)))
    }

    pub const fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    pub const fn as_bytes(self) -> [u8; 32] {
        self.0
    }

    pub fn to_hex(self) -> String {
        const HEX: &[u8; 16] = b"0123456789abcdef";
        let mut output = String::with_capacity(64);
        for byte in self.0 {
            output.push(char::from(HEX[usize::from(byte >> 4)]));
            output.push(char::from(HEX[usize::from(byte & 0x0f)]));
        }
        output
    }

    pub fn from_hex(value: &str) -> Option<Self> {
        let lowercase_hex = |byte: u8| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte);
        if value.len() != 64 || !value.bytes().all(lowercase_hex) {
            return None;
        }
        let mut bytes = [0_u8; 32];
        for (index, slot) in bytes.iter_mut().enumerate() {
            let start = index * 2;
            *slot = u8::from_str_radix(&value[start..start + 2], 16).ok()?;
        }
        Some(Self(bytes))
    }
}

impl fmt::Display for ContractDigest {
    fn fmt(&self, output: &mut fmt::Formatter<'_>) -> fmt::Result {
        output.write_str(&self.to_hex())
    }
}
