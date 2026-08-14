use std::fmt;
use std::num::NonZeroU64;
use std::str::FromStr;

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct WorkspaceId([u8; 16]);

impl WorkspaceId {
    pub const BYTE_LEN: usize = 16;

    pub const fn from_bytes(bytes: [u8; Self::BYTE_LEN]) -> Self {
        Self(bytes)
    }

    pub const fn as_bytes(self) -> [u8; Self::BYTE_LEN] {
        self.0
    }

    pub fn generate() -> Result<Self, IdentityError> {
        let mut bytes = [0_u8; Self::BYTE_LEN];
        getrandom::fill(&mut bytes).map_err(|_| IdentityError::EntropyUnavailable)?;
        if bytes == [0; Self::BYTE_LEN] {
            bytes[Self::BYTE_LEN - 1] = 1;
        }
        Ok(Self(bytes))
    }

    pub fn to_hex(self) -> String {
        encode_hex(&self.0)
    }
}

impl fmt::Display for WorkspaceId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&encode_hex(&self.0))
    }
}

impl FromStr for WorkspaceId {
    type Err = IdentityError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Ok(Self(decode_hex::<16>(value)?))
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Revision(u64);

impl Revision {
    pub const INITIAL: Self = Self(0);

    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    pub const fn get(self) -> u64 {
        self.0
    }

    pub fn next(self) -> Option<Self> {
        self.0.checked_add(1).map(Self)
    }
}

impl fmt::Display for Revision {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct NodeId {
    workspace: WorkspaceId,
    serial: NonZeroU64,
}

impl NodeId {
    pub fn new(workspace: WorkspaceId, serial: u64) -> Result<Self, IdentityError> {
        let serial = NonZeroU64::new(serial).ok_or(IdentityError::ZeroNodeSerial)?;
        Ok(Self { workspace, serial })
    }

    pub const fn workspace(self) -> WorkspaceId {
        self.workspace
    }

    pub const fn serial(self) -> u64 {
        self.serial.get()
    }
}

impl fmt::Display for NodeId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}:{}", self.workspace, self.serial)
    }
}

impl FromStr for NodeId {
    type Err = IdentityError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let (workspace, serial) = value.split_once(':').ok_or(IdentityError::InvalidNodeId)?;
        let workspace = workspace.parse()?;
        let serial = serial
            .parse::<u64>()
            .map_err(|_| IdentityError::InvalidNodeId)?;
        Self::new(workspace, serial)
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct LocalHandle(u32);

impl LocalHandle {
    pub const fn new(value: u32) -> Self {
        Self(value)
    }

    pub const fn get(self) -> u32 {
        self.0
    }
}

impl fmt::Display for LocalHandle {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "@{}", self.0)
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct RequestId(u64);

impl RequestId {
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    pub const fn get(self) -> u64 {
        self.0
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct IdempotencyKey([u8; 16]);

impl IdempotencyKey {
    pub const fn from_bytes(bytes: [u8; 16]) -> Self {
        Self(bytes)
    }

    pub const fn as_bytes(self) -> [u8; 16] {
        self.0
    }
}

impl fmt::Display for IdempotencyKey {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&encode_hex(&self.0))
    }
}

impl FromStr for IdempotencyKey {
    type Err = IdentityError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Ok(Self(decode_hex::<16>(value)?))
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct SnapshotHash([u8; 32]);

impl SnapshotHash {
    pub const BYTE_LEN: usize = 32;

    pub const fn from_bytes(bytes: [u8; Self::BYTE_LEN]) -> Self {
        Self(bytes)
    }

    pub const fn as_bytes(self) -> [u8; Self::BYTE_LEN] {
        self.0
    }
}

impl fmt::Display for SnapshotHash {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&encode_hex(&self.0))
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ArtifactVersion(pub u16);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SchemaId(pub [u8; 16]);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum IdentityError {
    EntropyUnavailable,
    InvalidHex,
    InvalidLength,
    InvalidNodeId,
    ZeroNodeSerial,
}

impl fmt::Display for IdentityError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::EntropyUnavailable => "operating-system entropy is unavailable",
            Self::InvalidHex => "identity contains invalid hexadecimal digits",
            Self::InvalidLength => "identity has the wrong encoded length",
            Self::InvalidNodeId => "node identity must be WORKSPACE:SERIAL",
            Self::ZeroNodeSerial => "node serial zero is reserved",
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for IdentityError {}

fn encode_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut result = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        result.push(char::from(HEX[usize::from(byte >> 4)]));
        result.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
    result
}

fn decode_hex<const N: usize>(value: &str) -> Result<[u8; N], IdentityError> {
    if value.len() != N * 2 {
        return Err(IdentityError::InvalidLength);
    }
    let source = value.as_bytes();
    let mut result = [0_u8; N];
    for (index, slot) in result.iter_mut().enumerate() {
        let high = decode_nibble(source[index * 2])?;
        let low = decode_nibble(source[index * 2 + 1])?;
        *slot = (high << 4) | low;
    }
    Ok(result)
}

fn decode_nibble(value: u8) -> Result<u8, IdentityError> {
    match value {
        b'0'..=b'9' => Ok(value - b'0'),
        b'a'..=b'f' => Ok(value - b'a' + 10),
        b'A'..=b'F' => Ok(value - b'A' + 10),
        _ => Err(IdentityError::InvalidHex),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identities_round_trip_without_positional_aliases() {
        let workspace = WorkspaceId::from_bytes([0xabu8; 16]);
        let encoded = workspace.to_string();
        assert_eq!(encoded.parse::<WorkspaceId>(), Ok(workspace));
        let node = NodeId::new(workspace, 42).expect("nonzero serial");
        assert_eq!(node.to_string().parse::<NodeId>(), Ok(node));
    }
}
