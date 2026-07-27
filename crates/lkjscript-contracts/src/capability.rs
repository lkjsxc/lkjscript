/// Closed provider authorities carried by unforgeable language values.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(u8)]
pub enum CapabilityKind {
    Arguments = 0,
    Clock = 1,
    Entropy = 2,
    FileSystem = 3,
    Network = 4,
    Sqlite = 5,
    Stdio = 6,
    Terminal = 7,
}

impl CapabilityKind {
    pub const ALL: [Self; 8] = [
        Self::Arguments,
        Self::Clock,
        Self::Entropy,
        Self::FileSystem,
        Self::Network,
        Self::Sqlite,
        Self::Stdio,
        Self::Terminal,
    ];

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Arguments => "arguments",
            Self::Clock => "clock",
            Self::Entropy => "entropy",
            Self::FileSystem => "file-system",
            Self::Network => "network",
            Self::Sqlite => "sqlite",
            Self::Stdio => "stdio",
            Self::Terminal => "terminal",
        }
    }

    pub fn parse(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|kind| kind.as_str() == name)
    }

    pub const fn from_tag(tag: u8) -> Option<Self> {
        match tag {
            0 => Some(Self::Arguments),
            1 => Some(Self::Clock),
            2 => Some(Self::Entropy),
            3 => Some(Self::FileSystem),
            4 => Some(Self::Network),
            5 => Some(Self::Sqlite),
            6 => Some(Self::Stdio),
            7 => Some(Self::Terminal),
            _ => None,
        }
    }
}
