use std::{
    env,
    net::{IpAddr, SocketAddr},
    path::PathBuf,
    str::FromStr,
};

use ipnet::IpNet;

#[derive(Debug, Clone)]
pub struct Config {
    pub bind_address: SocketAddr,
    pub redirects_file: PathBuf,
    pub events_file: PathBuf,
    pub trusted_proxies: Vec<IpNet>,
    pub record_client_addresses: bool,
    pub max_captured_header_value_bytes: usize,
    pub max_captured_header_total_bytes: usize,
}

impl Config {
    pub fn from_env() -> Result<Self, String> {
        let bind_address = value("COMPACTOR_BIND_ADDRESS", "0.0.0.0:8080")
            .parse()
            .map_err(|error| format!("invalid COMPACTOR_BIND_ADDRESS: {error}"))?;
        let redirects_file = PathBuf::from(value("COMPACTOR_REDIRECTS_FILE", "./redirects.json"));
        let events_file = PathBuf::from(value("COMPACTOR_EVENTS_FILE", "./events.jsonl"));
        let trusted_proxies =
            parse_trusted_proxies(&env::var("COMPACTOR_TRUSTED_PROXIES").unwrap_or_default())?;
        let record_client_addresses = parse_bool("COMPACTOR_RECORD_CLIENT_ADDRESSES", "true")?;
        let max_captured_header_value_bytes =
            parse_nonzero_usize("COMPACTOR_MAX_CAPTURED_HEADER_VALUE_BYTES", "1024")?;
        let max_captured_header_total_bytes =
            parse_nonzero_usize("COMPACTOR_MAX_CAPTURED_HEADER_TOTAL_BYTES", "4096")?;

        Ok(Self {
            bind_address,
            redirects_file,
            events_file,
            trusted_proxies,
            record_client_addresses,
            max_captured_header_value_bytes,
            max_captured_header_total_bytes,
        })
    }
}

fn value(name: &str, default: &str) -> String {
    env::var(name).unwrap_or_else(|_| default.to_owned())
}

fn parse_bool(name: &str, default: &str) -> Result<bool, String> {
    value(name, default)
        .parse()
        .map_err(|_| format!("{name} must be true or false"))
}

fn parse_nonzero_usize(name: &str, default: &str) -> Result<usize, String> {
    let parsed = value(name, default)
        .parse::<usize>()
        .map_err(|_| format!("{name} must be a positive integer"))?;
    if parsed == 0 {
        return Err(format!("{name} must be greater than zero"));
    }
    Ok(parsed)
}

fn parse_trusted_proxies(value: &str) -> Result<Vec<IpNet>, String> {
    value
        .split(',')
        .map(str::trim)
        .filter(|entry| !entry.is_empty())
        .map(|entry| {
            IpNet::from_str(entry)
                .or_else(|_| entry.parse::<IpAddr>().map(IpNet::from))
                .map_err(|_| format!("invalid trusted proxy IP or CIDR {entry:?}"))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trusted_proxies_accept_ips_and_cidrs() {
        let parsed = parse_trusted_proxies("127.0.0.1, 10.0.0.0/8").unwrap();
        assert_eq!(parsed.len(), 2);
        assert!(parsed[0].contains(&"127.0.0.1".parse::<IpAddr>().unwrap()));
        assert!(parsed[1].contains(&"10.4.3.2".parse::<IpAddr>().unwrap()));
    }
}
