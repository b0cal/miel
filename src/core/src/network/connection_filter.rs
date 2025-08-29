use super::protocol_filter::ProtocolFilter;
use crate::configuration::types::{IpFilter, PortFilter};

pub struct ConnectionFilter {
    ip_filter: IpFilter,
    port_filter: PortFilter,
    protocol_filter: ProtocolFilter,
}

impl ConnectionFilter {
    pub fn new() -> Self {
        Self {
            ip_filter: IpFilter::default(),
            port_filter: PortFilter::default(),
            protocol_filter: ProtocolFilter::default(),
        }
    }
}
