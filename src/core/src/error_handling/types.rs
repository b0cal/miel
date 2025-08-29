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
    StartFailed,
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
