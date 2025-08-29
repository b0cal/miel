use super::service_pattern::ServicePattern;
use crate::configuration::types::ServiceConfig;
use std::collections::HashMap;

pub struct ServiceDetector {
    pub service_patterns: HashMap<String, Vec<ServicePattern>>,
}

impl ServiceDetector {
    pub fn new(services: &[ServiceConfig]) -> Self {
        services.len();
        Self {
            service_patterns: HashMap::new(),
        }
    }
}
