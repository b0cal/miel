#[derive(Debug)]
pub enum ControllerError {
    //ex : Config(ConfigError),
}

#[derive(Debug)]
pub enum ConfigError {
    ServicesEmpty(String),
    NotInRange(String),
    BadIPFormatting(String),
    BadPortsRange(String),
    DirectoryDoesNotExist(String),

}
#[derive(Debug)]
pub enum SessionError {}

#[derive(Debug)]
pub enum WebError {}

#[derive(Debug)]
pub enum NetworkError {}

#[derive(Debug)]
pub enum ContainerError {}

#[derive(Debug)]
pub enum StorageError {}

