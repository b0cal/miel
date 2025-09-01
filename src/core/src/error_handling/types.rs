#[derive(Debug)]
pub enum ControllerError {
    Config(ConfigError),
    Network(NetworkError),
    Session(SessionError),
    Container(ContainerError),
    Storage(StorageError),
    Web(WebError),
}

// InvalidFormat and MissingField are not currently used in Config because the crate toml does
// only differentiate between the two in the message returned with the Error, so better to use TomlError(String)
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
pub enum SessionError {
    CreationFailed,
    ContainerError(ContainerError),
    StorageError(StorageError),
    CaptureError(CaptureError),
}

#[derive(Debug)]
pub enum WebError {
    RequestFailed,
    StartFailed(String),
}

#[derive(Debug)]
pub enum NetworkError {
    ConnectionFailed,
    ServiceDetectionFailed,
    BindFail(std::io::Error),
}

#[derive(Debug)]
pub enum ContainerError {
    RuntimeNotAvailable,
    CreationFailed(String),
    StartFailed(String),
    IoError(std::io::Error),
    ProcessError(String),
    ContainerNotFound(String),
    ConnectionFailed(String),
}

impl std::fmt::Display for ContainerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ContainerError::RuntimeNotAvailable => write!(f, "Container runtime is not available"),
            ContainerError::CreationFailed(msg) => write!(f, "Container creation failed: {}", msg),
            ContainerError::StartFailed(msg) => write!(f, "Container start failed: {}", msg),
            ContainerError::IoError(err) => write!(f, "IO error: {}", err),
            ContainerError::ProcessError(msg) => write!(f, "Process error: {}", msg),
            ContainerError::ContainerNotFound(msg) => write!(f, "Container not found: {}", msg),
            ContainerError::ConnectionFailed(msg) => write!(f, "Connection failed: {}", msg),
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
pub enum StorageError {
    ConnectionFailed,
    WriteFailed,
    ReadFailed,
}

#[derive(Debug)]
pub enum CaptureError {
    TcpStreamError(std::io::Error),
    StdioError(std::io::Error),
    StorageError(StorageError),
}
