use std::{net::Ipv4Addr, num::ParseIntError, str::FromStr};

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub struct IpAddrPort {
    pub ip: Ipv4Addr,
    pub port: u16,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct Quad {
    pub src: IpAddrPort,
    pub dst: IpAddrPort,
}

impl FromStr for IpAddrPort {
    type Err = ParseIntError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parts = s.split(":").collect::<Vec<_>>();
        Ok(IpAddrPort {
            ip: Ipv4Addr::from_str(parts[0]).expect("Failed to parse ip addr"),
            port: u16::from_str(parts[1]).expect("Failed to parse port"),
        })
    }
}
