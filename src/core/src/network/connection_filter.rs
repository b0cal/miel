use crate::configuration::types::{IpFilter, PortFilter};

use std::net::IpAddr;

#[derive(Clone, Default)]
pub struct ConnectionFilter {
    ip_filter: IpFilter,
    port_filter: PortFilter,
}

impl ConnectionFilter {
    pub fn new(ip_filter: IpFilter, port_filter: PortFilter) -> Self {
        Self {
            ip_filter,
            port_filter,
        }
    }
    pub fn should_accept_connection(&self, client_addr: &IpAddr, port: u16) -> bool {
        if !(self.is_ip_allowed(client_addr) && self.is_port_allowed(port)) {
            return false;
        }
        true
    }

    fn is_ip_allowed(&self, ip: &IpAddr) -> bool {
        let mut result = true;
        if self.ip_filter.whitelist_mode {
            // White list mode
            for ip_range in self.ip_filter.allowed_ranges.iter() {
                if !(ip.min(&ip_range.start).eq(&ip_range.start)
                    && ip.max(&ip_range.end).eq(&ip_range.end))
                {
                    result = false;
                }
            }
            result
        } else {
            // Blacklist mode
            for ip_range in self.ip_filter.blocked_ranges.iter() {
                if ip.min(&ip_range.start).eq(&ip_range.start)
                    && ip.max(&ip_range.end).eq(&ip_range.end)
                {
                    result = false;
                }
            }
            result
        }
    }

    // Blacklists by default
    fn is_port_allowed(&self, port: u16) -> bool {
        let mut result = true;
        for port_range in self.port_filter.allowed_ports.iter() {
            if !(port.min(port_range.start) == port_range.start
                && port.max(port_range.end) == port_range.end)
            {
                result = false;
            }
        }

        for port_range in self.port_filter.blocked_ports.iter() {
            if port.min(port_range.start) == port_range.start
                && port.max(port_range.end) == port_range.end
            {
                result = false;
            }
        }
        result
    }
}
