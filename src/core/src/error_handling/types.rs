pub enum ControllerError {
    //ex : Config(ConfigError),
}

pub enum ConfigError {}
pub enum SessionError {}

pub enum WebError {}

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

pub enum StorageError {}
