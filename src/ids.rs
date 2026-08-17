use serde::de::{self, Visitor};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
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

impl Serialize for WorkspaceId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for WorkspaceId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserialize_canonical_string(deserializer, "workspace ID")
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
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

const LOCAL_NODE_BIT: u64 = 1_u64 << 63;
const LOCAL_FUNCTION_MASK: u64 = (1_u64 << 31) - 1;
const LOCAL_ORDINAL_MASK: u64 = u32::MAX as u64;
pub const MAX_DURABLE_NODE_SERIAL: u64 = LOCAL_NODE_BIT - 1;
pub const MAX_LOCAL_FUNCTION_SERIAL: u64 = LOCAL_FUNCTION_MASK;
pub const MAX_FUNCTION_LOCAL_ORDINAL: u32 = u32::MAX;

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum NodeIdentityClass {
    Durable,
    FunctionLocal,
}

impl NodeIdentityClass {
    pub const fn machine_name(self) -> &'static str {
        match self {
            Self::Durable => "durable",
            Self::FunctionLocal => "function_local",
        }
    }
}

impl NodeId {
    /// Creates a durable workspace identity.
    pub fn new(workspace: WorkspaceId, serial: u64) -> Result<Self, IdentityError> {
        let serial = NonZeroU64::new(serial).ok_or(IdentityError::ZeroNodeSerial)?;
        if serial.get() & LOCAL_NODE_BIT != 0 {
            return Err(IdentityError::InvalidNodeDomain);
        }
        Ok(Self { workspace, serial })
    }

    /// Creates a revision-bound local reference in one durable function's body.
    pub fn new_function_local(
        workspace: WorkspaceId,
        function: Self,
        ordinal: u32,
    ) -> Result<Self, IdentityError> {
        if function.workspace != workspace || function.is_function_local() {
            return Err(IdentityError::InvalidLocalOwner);
        }
        let function_serial = function.serial();
        if function_serial > LOCAL_FUNCTION_MASK || ordinal == 0 {
            return Err(IdentityError::InvalidFunctionLocalId);
        }
        let encoded = LOCAL_NODE_BIT | (function_serial << 32) | u64::from(ordinal);
        let serial = NonZeroU64::new(encoded).ok_or(IdentityError::InvalidFunctionLocalId)?;
        Ok(Self { workspace, serial })
    }

    pub(crate) fn from_encoded(
        workspace: WorkspaceId,
        encoded: u64,
    ) -> Result<Self, IdentityError> {
        if encoded & LOCAL_NODE_BIT == 0 {
            return Self::new(workspace, encoded);
        }
        let serial = NonZeroU64::new(encoded).ok_or(IdentityError::ZeroNodeSerial)?;
        let id = Self { workspace, serial };
        if id.local_function_serial().is_none() || id.local_ordinal().is_none() {
            return Err(IdentityError::InvalidFunctionLocalId);
        }
        Ok(id)
    }

    pub const fn workspace(self) -> WorkspaceId {
        self.workspace
    }

    pub const fn serial(self) -> u64 {
        self.serial.get()
    }

    pub const fn identity_class(self) -> NodeIdentityClass {
        if self.is_function_local() {
            NodeIdentityClass::FunctionLocal
        } else {
            NodeIdentityClass::Durable
        }
    }

    pub const fn is_durable(self) -> bool {
        !self.is_function_local()
    }

    pub const fn is_function_local(self) -> bool {
        self.serial.get() & LOCAL_NODE_BIT != 0
    }

    pub const fn local_function_serial(self) -> Option<u64> {
        if !self.is_function_local() {
            return None;
        }
        let serial = (self.serial.get() >> 32) & LOCAL_FUNCTION_MASK;
        if serial == 0 { None } else { Some(serial) }
    }

    pub const fn local_ordinal(self) -> Option<u32> {
        if !self.is_function_local() {
            return None;
        }
        let ordinal = (self.serial.get() & LOCAL_ORDINAL_MASK) as u32;
        if ordinal == 0 { None } else { Some(ordinal) }
    }
}

impl fmt::Display for NodeId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let (Some(function), Some(ordinal)) =
            (self.local_function_serial(), self.local_ordinal())
        {
            write!(formatter, "{}:l{function}.{ordinal}", self.workspace)
        } else {
            write!(formatter, "{}:{}", self.workspace, self.serial)
        }
    }
}

impl FromStr for NodeId {
    type Err = IdentityError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let (workspace, serial) = value.split_once(':').ok_or(IdentityError::InvalidNodeId)?;
        let workspace = workspace.parse()?;
        if let Some(local) = serial.strip_prefix('l') {
            let (function, ordinal) = local.split_once('.').ok_or(IdentityError::InvalidNodeId)?;
            let function = function
                .parse::<u64>()
                .map_err(|_| IdentityError::InvalidNodeId)?;
            let ordinal = ordinal
                .parse::<u32>()
                .map_err(|_| IdentityError::InvalidNodeId)?;
            let function = Self::new(workspace, function)?;
            Self::new_function_local(workspace, function, ordinal)
        } else {
            let serial = serial
                .parse::<u64>()
                .map_err(|_| IdentityError::InvalidNodeId)?;
            Self::new(workspace, serial)
        }
    }
}

impl Serialize for NodeId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for NodeId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserialize_canonical_string(deserializer, "node ID")
    }
}

pub const MAX_DRAFT_SYMBOL_BYTES: usize = 64;
const MAX_UNVALIDATED_DRAFT_SYMBOL_BYTES: usize = MAX_DRAFT_SYMBOL_BYTES + 1;

/// A transaction-local proposal label. It is never persisted as semantic identity.
#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct DraftSymbol {
    bytes: [u8; MAX_UNVALIDATED_DRAFT_SYMBOL_BYTES],
    len: u8,
}

impl DraftSymbol {
    #[allow(clippy::expect_used)]
    pub fn new(value: &str) -> Self {
        Self::parse(value).expect("invalid draft symbol")
    }

    pub fn parse(value: &str) -> Result<Self, &'static str> {
        let symbol = Self::unvalidated(value)?;
        symbol.validate()?;
        Ok(symbol)
    }

    pub fn validate(self) -> Result<(), &'static str> {
        let bytes = self.public_bytes().ok_or("private draft symbol")?;
        if bytes.is_empty() {
            return Err("draft symbol must not be empty");
        }
        if bytes.len() > MAX_DRAFT_SYMBOL_BYTES {
            return Err("draft symbol exceeds byte policy");
        }
        if !bytes[0].is_ascii_lowercase()
            || !bytes[1..]
                .iter()
                .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || *byte == b'_')
        {
            return Err("draft symbol must match [a-z][a-z0-9_]*");
        }
        Ok(())
    }

    #[cfg(test)]
    pub(crate) fn generated(value: u32) -> Self {
        Self::new(&format!("s{value}"))
    }

    #[cfg(test)]
    pub(crate) fn generated_number(self) -> u32 {
        self.to_string()
            .strip_prefix('s')
            .and_then(|value| value.parse().ok())
            .expect("generated test draft symbol")
    }

    pub(crate) fn synthetic(value: u32) -> Self {
        let mut bytes = [0_u8; MAX_UNVALIDATED_DRAFT_SYMBOL_BYTES];
        bytes[0] = 0xff;
        bytes[1..5].copy_from_slice(&value.to_le_bytes());
        Self { bytes, len: 5 }
    }

    pub(crate) fn is_synthetic(self) -> bool {
        self.bytes[0] == 0xff
    }

    fn public_bytes(&self) -> Option<&[u8]> {
        (!self.is_synthetic()).then_some(&self.bytes[..usize::from(self.len)])
    }

    fn unvalidated(value: &str) -> Result<Self, &'static str> {
        if value.len() > MAX_UNVALIDATED_DRAFT_SYMBOL_BYTES {
            return Err("draft symbol exceeds boundary representation");
        }
        let mut bytes = [0_u8; MAX_UNVALIDATED_DRAFT_SYMBOL_BYTES];
        bytes[..value.len()].copy_from_slice(value.as_bytes());
        Ok(Self {
            bytes,
            len: u8::try_from(value.len()).map_err(|_| "draft symbol length overflow")?,
        })
    }
}

impl fmt::Debug for DraftSymbol {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, formatter)
    }
}

impl fmt::Display for DraftSymbol {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self
            .public_bytes()
            .and_then(|bytes| std::str::from_utf8(bytes).ok())
        {
            Some(value) => formatter.write_str(value),
            None => formatter.write_str("<private-draft>"),
        }
    }
}

impl Serialize for DraftSymbol {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let bytes = self
            .public_bytes()
            .ok_or_else(|| serde::ser::Error::custom("private draft symbol cannot serialize"))?;
        let value = std::str::from_utf8(bytes).map_err(serde::ser::Error::custom)?;
        serializer.serialize_str(value)
    }
}

impl<'de> Deserialize<'de> for DraftSymbol {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct DraftSymbolVisitor;
        impl Visitor<'_> for DraftSymbolVisitor {
            type Value = DraftSymbol;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("a transaction-local draft symbol string")
            }

            fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                DraftSymbol::unvalidated(value).map_err(E::custom)
            }
        }
        deserializer.deserialize_str(DraftSymbolVisitor)
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

impl Serialize for RequestId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_u64(self.0)
    }
}

impl<'de> Deserialize<'de> for RequestId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = u64::deserialize(deserializer)?;
        if value == 0 {
            return Err(de::Error::custom("request ID zero is reserved"));
        }
        Ok(Self(value))
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
pub struct QueryId(u64);

impl QueryId {
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

impl Serialize for IdempotencyKey {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for IdempotencyKey {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserialize_canonical_string(deserializer, "idempotency key")
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

impl FromStr for SnapshotHash {
    type Err = IdentityError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Ok(Self(decode_hex::<32>(value)?))
    }
}

impl Serialize for SnapshotHash {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for SnapshotHash {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserialize_canonical_string(deserializer, "snapshot hash")
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ChangeDigest([u8; 32]);

impl ChangeDigest {
    pub const BYTE_LEN: usize = 32;

    pub const fn from_bytes(bytes: [u8; Self::BYTE_LEN]) -> Self {
        Self(bytes)
    }

    pub const fn as_bytes(self) -> [u8; Self::BYTE_LEN] {
        self.0
    }
}

impl fmt::Display for ChangeDigest {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&encode_hex(&self.0))
    }
}

impl FromStr for ChangeDigest {
    type Err = IdentityError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Ok(Self(decode_hex::<32>(value)?))
    }
}

impl Serialize for ChangeDigest {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for ChangeDigest {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserialize_canonical_string(deserializer, "change digest")
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
    InvalidNodeDomain,
    InvalidFunctionLocalId,
    InvalidLocalOwner,
    ZeroNodeSerial,
}

impl fmt::Display for IdentityError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::EntropyUnavailable => "operating-system entropy is unavailable",
            Self::InvalidHex => "identity contains invalid hexadecimal digits",
            Self::InvalidLength => "identity has the wrong encoded length",
            Self::InvalidNodeId => {
                "node identity must be WORKSPACE:SERIAL or WORKSPACE:lFUNCTION.ORDINAL"
            }
            Self::InvalidNodeDomain => "durable node serial enters the function-local domain",
            Self::InvalidFunctionLocalId => "function-local node identity is outside its domain",
            Self::InvalidLocalOwner => {
                "function-local node identity requires a durable function in the same workspace"
            }
            Self::ZeroNodeSerial => "node serial zero is reserved",
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for IdentityError {}

fn deserialize_canonical_string<'de, D, T>(
    deserializer: D,
    description: &'static str,
) -> Result<T, D::Error>
where
    D: Deserializer<'de>,
    T: FromStr + fmt::Display,
    T::Err: fmt::Display,
{
    struct CanonicalVisitor<T> {
        description: &'static str,
        marker: std::marker::PhantomData<T>,
    }
    impl<'de, T> Visitor<'de> for CanonicalVisitor<T>
    where
        T: FromStr + fmt::Display,
        T::Err: fmt::Display,
    {
        type Value = T;
        fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(formatter, "a canonical {} string", self.description)
        }
        fn visit_str<E>(self, value: &str) -> Result<T, E>
        where
            E: de::Error,
        {
            let parsed = value.parse::<T>().map_err(E::custom)?;
            if parsed.to_string() != value {
                return Err(E::custom(format_args!(
                    "{} is not canonically encoded",
                    self.description
                )));
            }
            Ok(parsed)
        }
    }
    deserializer.deserialize_str(CanonicalVisitor {
        description,
        marker: std::marker::PhantomData,
    })
}

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

    #[test]
    fn durable_and_function_local_identity_domains_are_disjoint_and_canonical() {
        let workspace = WorkspaceId::from_bytes([0x42; 16]);
        let function = NodeId::new(workspace, 17).expect("durable function");
        let local = NodeId::new_function_local(workspace, function, 23).expect("local term");
        assert_eq!(function.identity_class(), NodeIdentityClass::Durable);
        assert_eq!(local.identity_class(), NodeIdentityClass::FunctionLocal);
        assert_eq!(local.local_function_serial(), Some(17));
        assert_eq!(local.local_ordinal(), Some(23));
        assert_eq!(local.to_string(), format!("{workspace}:l17.23"));
        assert_eq!(local.to_string().parse::<NodeId>(), Ok(local));
        assert_eq!(
            serde_json::to_string(&local).expect("encode"),
            format!("\"{local}\"")
        );
        assert_eq!(
            serde_json::from_str::<NodeId>(&format!("\"{local}\""))
                .expect("decode canonical local ID"),
            local
        );
        assert!(serde_json::from_str::<NodeId>(&format!("\"{workspace}:l017.23\"")).is_err());
        assert!(serde_json::from_str::<NodeId>(&format!("\"{workspace}:l17.023\"")).is_err());
    }

    #[test]
    fn function_local_identity_rejects_foreign_local_and_exhausted_owners() {
        let workspace = WorkspaceId::from_bytes([0x42; 16]);
        let foreign = WorkspaceId::from_bytes([0x43; 16]);
        let function = NodeId::new(workspace, 1).expect("function");
        let local = NodeId::new_function_local(workspace, function, 1).expect("local");
        assert_eq!(
            NodeId::new_function_local(foreign, function, 1),
            Err(IdentityError::InvalidLocalOwner)
        );
        assert_eq!(
            NodeId::new_function_local(workspace, local, 1),
            Err(IdentityError::InvalidLocalOwner)
        );
        assert_eq!(
            NodeId::new_function_local(workspace, function, 0),
            Err(IdentityError::InvalidFunctionLocalId)
        );
        assert_eq!(
            NodeId::new(workspace, MAX_DURABLE_NODE_SERIAL + 1),
            Err(IdentityError::InvalidNodeDomain)
        );
        let largest_function =
            NodeId::new(workspace, MAX_LOCAL_FUNCTION_SERIAL).expect("largest local owner");
        assert!(
            NodeId::new_function_local(workspace, largest_function, MAX_FUNCTION_LOCAL_ORDINAL)
                .is_ok()
        );
        let too_large = NodeId::new(workspace, MAX_LOCAL_FUNCTION_SERIAL + 1)
            .expect("durable identity outside local owner encoding");
        assert_eq!(
            NodeId::new_function_local(workspace, too_large, 1),
            Err(IdentityError::InvalidFunctionLocalId)
        );
    }
}
