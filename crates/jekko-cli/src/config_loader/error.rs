/// Errors emitted by the TOML loader.
#[derive(Debug)]
pub enum LoadError {
    /// Underlying filesystem failure while reading the config file.
    Io(std::io::Error),
    /// TOML deserialization failure.
    Parse(toml::de::Error),
}

impl std::fmt::Display for LoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(err) => write!(f, "failed to read jekko ui.toml: {err}"),
            Self::Parse(err) => write!(f, "failed to parse jekko ui.toml: {err}"),
        }
    }
}

impl std::error::Error for LoadError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(err) => Some(err),
            Self::Parse(err) => Some(err),
        }
    }
}

impl From<std::io::Error> for LoadError {
    fn from(err: std::io::Error) -> Self {
        Self::Io(err)
    }
}

impl From<toml::de::Error> for LoadError {
    fn from(err: toml::de::Error) -> Self {
        Self::Parse(err)
    }
}
