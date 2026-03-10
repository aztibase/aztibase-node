use std::collections::HashMap;
use std::net::IpAddr;
use std::time::Instant;

pub const MAX_CONNS_PER_IP: u32 = 3;
pub const MIN_CONNECT_INTERVAL_MS: u64 = 1000;
pub const MAX_PEERS_PER_SUBNET: u32 = 5;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FilterReason {
    IpLimit,
    RateLimit,
    SubnetLimit,
}

impl std::fmt::Display for FilterReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::IpLimit => write!(f, "ip-limit"),
            Self::RateLimit => write!(f, "rate-limit"),
            Self::SubnetLimit => write!(f, "subnet-limit"),
        }
    }
}

struct IpState {
    active: u32,
    last_connect: Instant,
}

pub struct ConnectionFilter {
    ips: HashMap<IpAddr, IpState>,
    subnets: HashMap<u16, u32>,
    max_per_ip: u32,
    min_interval_ms: u64,
    max_per_subnet: u32,
}

impl Default for ConnectionFilter {
    fn default() -> Self {
        Self::new()
    }
}

impl ConnectionFilter {
    pub fn new() -> Self {
        Self {
            ips: HashMap::new(),
            subnets: HashMap::new(),
            max_per_ip: MAX_CONNS_PER_IP,
            min_interval_ms: MIN_CONNECT_INTERVAL_MS,
            max_per_subnet: MAX_PEERS_PER_SUBNET,
        }
    }

    pub fn with_limits(max_per_ip: u32, min_interval_ms: u64, max_per_subnet: u32) -> Self {
        Self {
            ips: HashMap::new(),
            subnets: HashMap::new(),
            max_per_ip,
            min_interval_ms,
            max_per_subnet,
        }
    }

    pub fn try_accept(&mut self, ip: IpAddr) -> Result<(), FilterReason> {
        let now = Instant::now();
        let is_loopback = ip.is_loopback();

        if let Some(state) = self.ips.get(&ip) {
            if state.active >= self.max_per_ip {
                return Err(FilterReason::IpLimit);
            }
            if !is_loopback
                && now.duration_since(state.last_connect).as_millis() < self.min_interval_ms as u128
            {
                return Err(FilterReason::RateLimit);
            }
        }

        let subnet = subnet_key(ip);
        let subnet_count = self.subnets.get(&subnet).copied().unwrap_or(0);
        if subnet_count >= self.max_per_subnet {
            return Err(FilterReason::SubnetLimit);
        }

        let state = self.ips.entry(ip).or_insert(IpState {
            active: 0,
            last_connect: now,
        });
        state.active += 1;
        state.last_connect = now;

        *self.subnets.entry(subnet).or_insert(0) += 1;

        Ok(())
    }

    pub fn release(&mut self, ip: IpAddr) {
        if let Some(state) = self.ips.get_mut(&ip) {
            state.active = state.active.saturating_sub(1);
            if state.active == 0 {
                self.ips.remove(&ip);
            }
        }

        let subnet = subnet_key(ip);
        if let Some(count) = self.subnets.get_mut(&subnet) {
            *count = count.saturating_sub(1);
            if *count == 0 {
                self.subnets.remove(&subnet);
            }
        }
    }

    pub fn active_count(&self, ip: IpAddr) -> u32 {
        self.ips.get(&ip).map_or(0, |s| s.active)
    }

    pub fn subnet_count(&self, ip: IpAddr) -> u32 {
        let subnet = subnet_key(ip);
        self.subnets.get(&subnet).copied().unwrap_or(0)
    }
}

fn subnet_key(ip: IpAddr) -> u16 {
    match ip {
        IpAddr::V4(v4) => {
            let octets = v4.octets();
            u16::from_be_bytes([octets[0], octets[1]])
        }
        IpAddr::V6(v6) => {
            let segments = v6.segments();
            segments[0]
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{Ipv4Addr, Ipv6Addr};

    #[test]
    fn per_ip_limit_enforced() {
        let mut filter = ConnectionFilter::with_limits(2, 0, 100);
        let ip = IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1));

        assert!(filter.try_accept(ip).is_ok());
        assert!(filter.try_accept(ip).is_ok());
        assert_eq!(filter.try_accept(ip), Err(FilterReason::IpLimit));

        filter.release(ip);
        assert!(filter.try_accept(ip).is_ok());
    }

    #[test]
    fn rate_limit_rejection() {
        let mut filter = ConnectionFilter::with_limits(10, 5000, 100);
        let ip = IpAddr::V4(Ipv4Addr::new(10, 0, 0, 2));

        assert!(filter.try_accept(ip).is_ok());
        assert_eq!(filter.try_accept(ip), Err(FilterReason::RateLimit));
    }

    #[test]
    fn subnet_cap_enforced() {
        let mut filter = ConnectionFilter::with_limits(10, 0, 2);

        let ip1 = IpAddr::V4(Ipv4Addr::new(192, 168, 0, 1));
        let ip2 = IpAddr::V4(Ipv4Addr::new(192, 168, 0, 2));
        let ip3 = IpAddr::V4(Ipv4Addr::new(192, 168, 0, 3));

        assert!(filter.try_accept(ip1).is_ok());
        assert!(filter.try_accept(ip2).is_ok());
        assert_eq!(filter.try_accept(ip3), Err(FilterReason::SubnetLimit));

        let other_subnet = IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1));
        assert!(filter.try_accept(other_subnet).is_ok());
    }

    #[test]
    fn legitimate_peer_after_release() {
        let mut filter = ConnectionFilter::with_limits(1, 0, 100);
        let ip = IpAddr::V4(Ipv4Addr::new(172, 16, 0, 1));

        assert!(filter.try_accept(ip).is_ok());
        assert_eq!(filter.try_accept(ip), Err(FilterReason::IpLimit));

        filter.release(ip);
        assert_eq!(filter.active_count(ip), 0);
        assert!(filter.try_accept(ip).is_ok());
    }

    #[test]
    fn ipv6_subnet_tracking() {
        let mut filter = ConnectionFilter::with_limits(10, 0, 2);

        let ip1 = IpAddr::V6(Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, 1));
        let ip2 = IpAddr::V6(Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, 2));
        let ip3 = IpAddr::V6(Ipv6Addr::new(0x2001, 0xdb8, 1, 0, 0, 0, 0, 1));

        assert!(filter.try_accept(ip1).is_ok());
        assert!(filter.try_accept(ip2).is_ok());
        assert_eq!(filter.try_accept(ip3), Err(FilterReason::SubnetLimit));
    }

    #[test]
    fn subnet_key_groups_correctly() {
        assert_eq!(
            subnet_key(IpAddr::V4(Ipv4Addr::new(192, 168, 0, 1))),
            subnet_key(IpAddr::V4(Ipv4Addr::new(192, 168, 255, 255)))
        );
        assert_ne!(
            subnet_key(IpAddr::V4(Ipv4Addr::new(192, 168, 0, 1))),
            subnet_key(IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1)))
        );
    }

    #[test]
    fn release_cleans_up_state() {
        let mut filter = ConnectionFilter::with_limits(10, 0, 100);
        let ip = IpAddr::V4(Ipv4Addr::new(10, 0, 0, 5));

        assert!(filter.try_accept(ip).is_ok());
        assert_eq!(filter.active_count(ip), 1);
        assert_eq!(filter.subnet_count(ip), 1);

        filter.release(ip);
        assert_eq!(filter.active_count(ip), 0);
        assert_eq!(filter.subnet_count(ip), 0);
    }
}
