/// Closed source-visible affine resource kinds.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(u8)]
pub enum ResourceKind {
    InputStream = 0,
    OutputStream = 1,
    FileReader = 2,
    FileWriter = 3,
    FileAppender = 4,
    Directory = 5,
    TcpListener = 6,
    TcpStream = 7,
    SqliteConnection = 8,
    SqliteStatement = 9,
    TerminalSession = 10,
}

impl ResourceKind {
    pub const ALL: [Self; 11] = [
        Self::InputStream,
        Self::OutputStream,
        Self::FileReader,
        Self::FileWriter,
        Self::FileAppender,
        Self::Directory,
        Self::TcpListener,
        Self::TcpStream,
        Self::SqliteConnection,
        Self::SqliteStatement,
        Self::TerminalSession,
    ];

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::InputStream => "input-stream",
            Self::OutputStream => "output-stream",
            Self::FileReader => "file-reader",
            Self::FileWriter => "file-writer",
            Self::FileAppender => "file-appender",
            Self::Directory => "directory",
            Self::TcpListener => "tcp-listener",
            Self::TcpStream => "tcp-stream",
            Self::SqliteConnection => "sqlite-connection",
            Self::SqliteStatement => "sqlite-statement",
            Self::TerminalSession => "terminal-session",
        }
    }

    pub fn parse(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|kind| kind.as_str() == name)
    }

    pub const fn from_tag(tag: u8) -> Option<Self> {
        match tag {
            0 => Some(Self::InputStream),
            1 => Some(Self::OutputStream),
            2 => Some(Self::FileReader),
            3 => Some(Self::FileWriter),
            4 => Some(Self::FileAppender),
            5 => Some(Self::Directory),
            6 => Some(Self::TcpListener),
            7 => Some(Self::TcpStream),
            8 => Some(Self::SqliteConnection),
            9 => Some(Self::SqliteStatement),
            10 => Some(Self::TerminalSession),
            _ => None,
        }
    }
}
