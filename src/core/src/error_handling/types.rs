use std::fmt;

#[derive(Debug)]
pub enum ConfigError {
    IoError(std::io::Error),
    TomlError(String),
    ServicesEmpty(String),
    BadIPFormatting(String),
    BadPortsRange(String),
    DirectoryDoesNotExist(String),
    NotInRange(String),
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConfigError::IoError(e) => write!(f, "IO error: {}", e),
            ConfigError::TomlError(e) => write!(f, "TOML parsing error: {}", e),
            ConfigError::ServicesEmpty(e) => write!(f, "Services configuration error: {}", e),
            ConfigError::BadIPFormatting(e) => write!(f, "IP formatting error: {}", e),
            ConfigError::BadPortsRange(e) => write!(f, "Port range error: {}", e),
            ConfigError::DirectoryDoesNotExist(e) => write!(f, "Directory error: {}", e),
            ConfigError::NotInRange(e) => write!(f, "Value out of range: {}", e),
        }
    }
}

impl std::error::Error for ConfigError {}

#[derive(Debug)]
pub enum SessionError {
    CreationFailed,
    ContainerError(ContainerError),
    StorageError(StorageError),
    CaptureError(CaptureError),
    NotFound,
    SessionLimitReached,
}

impl fmt::Display for SessionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SessionError::CreationFailed => write!(f, "Session creation failed"),
            SessionError::ContainerError(e) => write!(f, "Container error: {}", e),
            SessionError::StorageError(e) => write!(f, "Storage error: {}", e),
            SessionError::CaptureError(e) => write!(f, "Capture error: {}", e),
            SessionError::NotFound => write!(f, "Session not found"),
            SessionError::SessionLimitReached => write!(f, "Session limit reached"),
        }
    }
}

impl std::error::Error for SessionError {}

#[derive(Debug)]
pub enum ContainerError {
    RuntimeNotAvailable,
    CreationFailed(String),
    StartFailed(String),
    IoError(std::io::Error),
    ProcessError(String),
    InsufficientPrivileges,
    ConnectionFailed(String),
}

impl fmt::Display for ContainerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ContainerError::RuntimeNotAvailable => write!(f, "Container runtime not available"),
            ContainerError::CreationFailed(e) => write!(f, "Container creation failed: {}", e),
            ContainerError::StartFailed(e) => write!(f, "Container start failed: {}", e),
            ContainerError::IoError(e) => write!(f, "Container IO error: {}", e),
            ContainerError::ProcessError(e) => write!(f, "Container process error: {}", e),
            ContainerError::InsufficientPrivileges => {
                write!(f, "Insufficient privileges for container operations")
            }
            ContainerError::ConnectionFailed(e) => write!(f, "Container connection failed: {}", e),
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
pub enum NetworkError {
    BindError(std::io::Error),
    ChannelFailed,
    SockError(std::io::Error),
    ConnectionFailed,
    ServiceDetectionFailed,
    BindFail(std::io::Error),
}

impl fmt::Display for NetworkError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NetworkError::BindError(e) => write!(f, "Network bind error: {}", e),
            NetworkError::ChannelFailed => write!(f, "Network channel failed"),
            NetworkError::SockError(e) => write!(f, "Socket error: {}", e),
            NetworkError::ConnectionFailed => write!(f, "Connection failed"),
            NetworkError::ServiceDetectionFailed => write!(f, "Service detection failed"),
            NetworkError::BindFail(e) => write!(f, "Bind failed: {}", e),
        }
    }
}

impl std::error::Error for NetworkError {}

#[derive(Debug)]
pub enum StorageError {
    ConnectionFailed,
    WriteFailed,
    ReadFailed,
}

impl fmt::Display for StorageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StorageError::ConnectionFailed => write!(f, "Storage connection failed"),
            StorageError::WriteFailed => write!(f, "Storage write failed"),
            StorageError::ReadFailed => write!(f, "Storage read failed"),
        }
    }
}

impl std::error::Error for StorageError {}

#[derive(Debug)]
pub enum CaptureError {
    TcpStreamError(std::io::Error),
    StdioError(std::io::Error),
    StorageError(StorageError),
}

impl fmt::Display for CaptureError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CaptureError::TcpStreamError(e) => write!(f, "TCP stream capture error: {}", e),
            CaptureError::StdioError(e) => write!(f, "Stdio capture error: {}", e),
            CaptureError::StorageError(e) => write!(f, "Capture storage error: {}", e),
        }
    }
}

impl std::error::Error for CaptureError {}

#[derive(Debug)]
pub enum ControllerError {
    ConfigurationError(ConfigError),
    NetworkError(NetworkError),
    SessionError(SessionError),
    ContainerError(ContainerError),
    StorageError(StorageError),
    InitializationFailed(String),
}

impl fmt::Display for ControllerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ControllerError::ConfigurationError(e) => write!(f, "Configuration error: {}", e),
            ControllerError::NetworkError(e) => write!(f, "Network error: {}", e),
            ControllerError::SessionError(e) => write!(f, "Session error: {}", e),
            ControllerError::ContainerError(e) => write!(f, "Container error: {}", e),
            ControllerError::StorageError(e) => write!(f, "Storage error: {}", e),
            ControllerError::InitializationFailed(e) => write!(f, "Initialization failed: {}", e),
        }
    }
}

impl std::error::Error for ControllerError {}
