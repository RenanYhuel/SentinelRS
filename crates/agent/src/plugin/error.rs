use std::fmt;

#[derive(Debug)]
pub enum PluginError {
    Compile(String),
    Instantiation(String),
    Execution(String),
    Timeout,
    MemoryLimit,
    InvalidOutput(String),
    Io(std::io::Error),
}

impl fmt::Display for PluginError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Compile(msg) => write!(f, "compile: {msg}"),
            Self::Instantiation(msg) => write!(f, "instantiation: {msg}"),
            Self::Execution(msg) => write!(f, "execution: {msg}"),
            Self::Timeout => write!(f, "plugin execution timed out"),
            Self::MemoryLimit => write!(f, "plugin exceeded memory limit"),
            Self::InvalidOutput(msg) => write!(f, "invalid output: {msg}"),
            Self::Io(e) => write!(f, "io: {e}"),
        }
    }
}

impl std::error::Error for PluginError {}

impl From<std::io::Error> for PluginError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}
