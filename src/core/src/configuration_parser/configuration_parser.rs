use super::runtime_configuration::RuntimeConfiguration;
use super::configuration;

pub struct ConfigurationParser {
    pub foo: i32,
}

impl ConfigurationParser {
    pub fn new() -> RuntimeConfiguration {
        RuntimeConfiguration { foo: 0 }
    }
}