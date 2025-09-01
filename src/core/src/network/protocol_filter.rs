use crate::configuration::types::Protocol;

#[derive(Clone)]
pub struct ProtocolFilter {
    allowed_protocols: Vec<Protocol>,
    blocked_protocols: Vec<Protocol>,
}

impl ProtocolFilter {
    pub fn default() -> Self {
        Self {
            allowed_protocols: vec![],
            blocked_protocols: vec![],
        }
    }
}
