#[derive(Debug)]
#[derive(Debug)]
pub enum ControllerError {
    //ex : Config(ConfigError),
}

#[derive(Debug)]
pub enum ConfigError {
    InvalidFormat,
    MissingField(String),
    IoError(std::io::Error),
    TomlError(String),
    ServicesEmpty(String),
    NotInRange(String),
    BadIPFormatting(String),
    BadPortsRange(String),
    DirectoryDoesNotExist(String),
}

#[derive(Debug)]
pub enum SessionError {}

#[derive(Debug)]
#[derive(Debug)]
pub enum WebError {}

#[derive(Debug)]
#[derive(Debug)]
pub enum NetworkError {}

#[derive(Debug)]
pub enum ContainerError {
    RuntimeNotAvailable,
    CreationFailed(String),
    StartFailed(String),
    IoError(std::io::Error),
    ProcessError(String),
}

impl std::fmt::Display for ContainerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ContainerError::RuntimeNotAvailable => write!(f, "Container runtime is not available"),
            ContainerError::CreationFailed(msg) => write!(f, "Container creation failed: {}", msg),
            ContainerError::StartFailed(msg) => write!(f, "Container start failed: {}", msg),
            ContainerError::IoError(err) => write!(f, "IO error: {}", err),
            ContainerError::ProcessError(msg) => write!(f, "Process error: {}", msg),
        }
    }
}

impl std::error::Error for ContainerError {}

impl From<std::io::Error> for ContainerError {
    fn from(err: std::io::Error) -> Self {
        ContainerError::IoError(err)
    }
}

#[derive(Debug)]
pub enum StorageError {}
