use std::{
    collections::BTreeMap,
    env,
    net::{IpAddr, SocketAddr},
    num::NonZeroUsize,
    path::{Path, PathBuf},
    str::FromStr,
    time::Duration,
};

use ipnet::IpNet;

use crate::{HttpEndpointConfig, HttpTransportConfig};

#[derive(Debug, Clone)]
pub enum RedirectSourceConfig {
    Json {
        path: PathBuf,
    },
    Http {
        endpoint: HttpEndpointConfig,
        max_response_bytes: usize,
    },
}

#[derive(Debug, Clone)]
pub enum RedirectEventSinkConfig {
    Jsonl { path: PathBuf },
    Http { endpoint: HttpEndpointConfig },
}

#[derive(Debug, Clone)]
pub struct Config {
    pub bind_address: SocketAddr,
    pub source: RedirectSourceConfig,
    pub event_sink: RedirectEventSinkConfig,
    pub http_transport: Option<HttpTransportConfig>,
    pub trusted_proxies: Vec<IpNet>,
    pub record_client_addresses: bool,
    pub max_captured_header_value_bytes: usize,
    pub max_captured_header_total_bytes: usize,
    pub redirect_cache_ttl: Duration,
    pub redirect_cache_max_entries: NonZeroUsize,
}

impl Config {
    pub fn from_env() -> Result<Self, String> {
        Self::from_lookup(|name| env::var(name).ok())
    }

    fn from_lookup(mut lookup: impl FnMut(&str) -> Option<String>) -> Result<Self, String> {
        let mut value =
            |name: &str, default: &str| lookup(name).unwrap_or_else(|| default.to_owned());
        let bind_address = value("COMPACTOR_BIND_ADDRESS", "0.0.0.0:8080")
            .parse()
            .map_err(|error| format!("invalid COMPACTOR_BIND_ADDRESS: {error}"))?;
        let source_type = value("COMPACTOR_SOURCE_TYPE", "json");
        let source = match source_type.as_str() {
            "json" => RedirectSourceConfig::Json {
                path: PathBuf::from(value("COMPACTOR_REDIRECTS_FILE", "./redirects.json")),
            },
            "http" => {
                let endpoint = http_endpoint_from_lookup(
                    "SOURCE",
                    &mut value,
                    "COMPACTOR_HTTP_SOURCE_URL",
                    "COMPACTOR_HTTP_SOURCE_REQUEST_TIMEOUT_MS",
                )?;
                let max_response_bytes = parse_nonzero_usize_value(
                    "COMPACTOR_HTTP_SOURCE_MAX_RESPONSE_BYTES",
                    &value("COMPACTOR_HTTP_SOURCE_MAX_RESPONSE_BYTES", "65536"),
                )?;
                RedirectSourceConfig::Http {
                    endpoint,
                    max_response_bytes,
                }
            }
            _ => return Err("COMPACTOR_SOURCE_TYPE must be json or http".into()),
        };
        let sink_type = value("COMPACTOR_EVENT_SINK_TYPE", "jsonl");
        let event_sink = match sink_type.as_str() {
            "jsonl" => RedirectEventSinkConfig::Jsonl {
                path: PathBuf::from(value("COMPACTOR_EVENTS_FILE", "./events.jsonl")),
            },
            "http" => RedirectEventSinkConfig::Http {
                endpoint: http_endpoint_from_lookup(
                    "EVENT_SINK",
                    &mut value,
                    "COMPACTOR_HTTP_EVENT_SINK_URL",
                    "COMPACTOR_HTTP_EVENT_SINK_REQUEST_TIMEOUT_MS",
                )?,
            },
            _ => return Err("COMPACTOR_EVENT_SINK_TYPE must be jsonl or http".into()),
        };
        let http_transport = if matches!(source, RedirectSourceConfig::Http { .. })
            || matches!(event_sink, RedirectEventSinkConfig::Http { .. })
        {
            Some(
                HttpTransportConfig::new(Duration::from_millis(parse_nonzero_u64_value(
                    "COMPACTOR_HTTP_CONNECT_TIMEOUT_MS",
                    &value("COMPACTOR_HTTP_CONNECT_TIMEOUT_MS", "500"),
                )?))
                .map_err(|error| error.to_string())?,
            )
        } else {
            None
        };
        let trusted_proxies = parse_trusted_proxies(&value("COMPACTOR_TRUSTED_PROXIES", ""))?;
        let record_client_addresses = parse_bool_value(
            "COMPACTOR_RECORD_CLIENT_ADDRESSES",
            &value("COMPACTOR_RECORD_CLIENT_ADDRESSES", "true"),
        )?;
        let max_captured_header_value_bytes = parse_nonzero_usize_value(
            "COMPACTOR_MAX_CAPTURED_HEADER_VALUE_BYTES",
            &value("COMPACTOR_MAX_CAPTURED_HEADER_VALUE_BYTES", "1024"),
        )?;
        let max_captured_header_total_bytes = parse_nonzero_usize_value(
            "COMPACTOR_MAX_CAPTURED_HEADER_TOTAL_BYTES",
            &value("COMPACTOR_MAX_CAPTURED_HEADER_TOTAL_BYTES", "4096"),
        )?;
        let redirect_cache_ttl = Duration::from_secs(parse_nonzero_u64_value(
            "COMPACTOR_REDIRECT_CACHE_TTL_SECONDS",
            &value("COMPACTOR_REDIRECT_CACHE_TTL_SECONDS", "300"),
        )?);
        let redirect_cache_max_entries = NonZeroUsize::new(parse_nonzero_usize_value(
            "COMPACTOR_REDIRECT_CACHE_MAX_ENTRIES",
            &value("COMPACTOR_REDIRECT_CACHE_MAX_ENTRIES", "10000"),
        )?)
        .expect("positive cache entry count is nonzero");

        Ok(Self {
            bind_address,
            source,
            event_sink,
            http_transport,
            trusted_proxies,
            record_client_addresses,
            max_captured_header_value_bytes,
            max_captured_header_total_bytes,
            redirect_cache_ttl,
            redirect_cache_max_entries,
        })
    }
}

fn http_endpoint_from_lookup(
    adapter: &str,
    value: &mut impl FnMut(&str, &str) -> String,
    url_name: &str,
    timeout_name: &str,
) -> Result<HttpEndpointConfig, String> {
    let url = value(url_name, "");
    if url.is_empty() {
        return Err(format!("{url_name} is required for the selected adapter"));
    }
    let timeout = Duration::from_millis(parse_nonzero_u64_value(
        timeout_name,
        &value(timeout_name, "1500"),
    )?);
    let token_name = format!("COMPACTOR_HTTP_{adapter}_BEARER_TOKEN");
    let token_file_name = format!("COMPACTOR_HTTP_{adapter}_BEARER_TOKEN_FILE");
    let direct_token = configured_setting(value(&token_name, "\0"));
    let token_file = configured_setting(value(&token_file_name, "\0"));
    if direct_token.is_some() && token_file.is_some() {
        return Err(format!(
            "{token_name} and {token_file_name} are mutually exclusive"
        ));
    }
    let bearer_token = match (direct_token, token_file) {
        (Some(token), None) => Some(token),
        (None, Some(path)) => Some(read_token_file(&path, &token_file_name)?),
        (None, None) => None,
        (Some(_), Some(_)) => unreachable!(),
    };
    let headers_name = format!("COMPACTOR_HTTP_{adapter}_STATIC_HEADERS_JSON");
    let static_headers = parse_static_headers(&headers_name, &value(&headers_name, "{}"))?;
    HttpEndpointConfig::new(url, timeout, static_headers, bearer_token)
        .map_err(|error| format!("invalid {adapter} HTTP configuration: {error}"))
}

fn configured_setting(value: String) -> Option<String> {
    if value == "\0" { None } else { Some(value) }
}

fn read_token_file(path: &str, name: &str) -> Result<String, String> {
    let contents = std::fs::read_to_string(Path::new(path))
        .map_err(|error| format!("could not read {name}: {error}"))?;
    let token = contents
        .strip_suffix("\r\n")
        .or_else(|| contents.strip_suffix('\n'))
        .unwrap_or(&contents)
        .to_owned();
    if token.is_empty() {
        return Err(format!("{name} must not contain an empty token"));
    }
    Ok(token)
}

fn parse_static_headers(name: &str, input: &str) -> Result<BTreeMap<String, String>, String> {
    serde_json::from_str(input)
        .map_err(|_| format!("{name} must be a JSON object with string values"))
}

fn parse_bool_value(name: &str, value: &str) -> Result<bool, String> {
    value
        .parse()
        .map_err(|_| format!("{name} must be true or false"))
}

fn parse_nonzero_usize_value(name: &str, value: &str) -> Result<usize, String> {
    let parsed = value
        .parse::<usize>()
        .map_err(|_| format!("{name} must be a positive integer"))?;
    if parsed == 0 {
        return Err(format!("{name} must be greater than zero"));
    }
    Ok(parsed)
}

fn parse_nonzero_u64_value(name: &str, value: &str) -> Result<u64, String> {
    let parsed = value
        .parse::<u64>()
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
    use std::collections::HashMap;

    use super::*;

    fn config(values: &[(&str, &str)]) -> Result<Config, String> {
        let values: HashMap<_, _> = values
            .iter()
            .map(|(name, value)| ((*name).to_owned(), (*value).to_owned()))
            .collect();
        Config::from_lookup(|name| values.get(name).cloned())
    }

    #[test]
    fn defaults_to_file_adapters_and_ignores_inactive_http_settings() {
        let parsed = config(&[
            ("COMPACTOR_HTTP_SOURCE_URL", "not a URL"),
            ("COMPACTOR_HTTP_EVENT_SINK_URL", "also not a URL"),
            ("COMPACTOR_HTTP_CONNECT_TIMEOUT_MS", "0"),
            ("COMPACTOR_HTTP_SOURCE_MAX_RESPONSE_BYTES", "0"),
        ])
        .unwrap();
        assert!(matches!(parsed.source, RedirectSourceConfig::Json { .. }));
        assert!(matches!(
            parsed.event_sink,
            RedirectEventSinkConfig::Jsonl { .. }
        ));
        assert!(parsed.http_transport.is_none());
    }

    #[test]
    fn supports_every_adapter_combination() {
        for (source, sink) in [
            ("json", "jsonl"),
            ("http", "jsonl"),
            ("json", "http"),
            ("http", "http"),
        ] {
            let parsed = config(&[
                ("COMPACTOR_SOURCE_TYPE", source),
                ("COMPACTOR_EVENT_SINK_TYPE", sink),
                (
                    "COMPACTOR_HTTP_SOURCE_URL",
                    "https://source.example/resolve",
                ),
                (
                    "COMPACTOR_HTTP_EVENT_SINK_URL",
                    "https://sink.example/events",
                ),
            ])
            .unwrap();
            assert_eq!(
                parsed.http_transport.is_some(),
                source == "http" || sink == "http"
            );
        }
    }

    #[test]
    fn rejects_selected_adapter_errors_without_exposing_token() {
        let secret = "top-secret";
        let error = config(&[
            ("COMPACTOR_SOURCE_TYPE", "http"),
            (
                "COMPACTOR_HTTP_SOURCE_URL",
                "https://source.example/resolve",
            ),
            ("COMPACTOR_HTTP_SOURCE_BEARER_TOKEN", secret),
            ("COMPACTOR_HTTP_SOURCE_BEARER_TOKEN_FILE", "/unused"),
        ])
        .unwrap_err();
        assert!(error.contains("mutually exclusive"));
        assert!(!error.contains(secret));

        let empty = config(&[
            ("COMPACTOR_SOURCE_TYPE", "http"),
            (
                "COMPACTOR_HTTP_SOURCE_URL",
                "https://source.example/resolve",
            ),
            ("COMPACTOR_HTTP_SOURCE_BEARER_TOKEN", ""),
        ])
        .unwrap_err();
        assert!(empty.contains("must not be empty"));
    }

    #[test]
    fn token_files_trim_exactly_one_line_ending() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("token");
        std::fs::write(&path, "secret\r\n").unwrap();
        let path = path.to_str().unwrap();
        let parsed = config(&[
            ("COMPACTOR_EVENT_SINK_TYPE", "http"),
            (
                "COMPACTOR_HTTP_EVENT_SINK_URL",
                "https://sink.example/events",
            ),
            ("COMPACTOR_HTTP_EVENT_SINK_BEARER_TOKEN_FILE", path),
        ])
        .unwrap();
        let RedirectEventSinkConfig::Http { endpoint } = parsed.event_sink else {
            panic!("expected HTTP sink")
        };
        assert!(format!("{endpoint:?}").contains("[redacted]"));
        assert!(!format!("{endpoint:?}").contains("secret"));
    }

    #[test]
    fn validates_proxy_and_positive_limits() {
        assert!(config(&[("COMPACTOR_TRUSTED_PROXIES", "10.0.0.0/33")]).is_err());
        assert!(config(&[("COMPACTOR_REDIRECT_CACHE_MAX_ENTRIES", "0")]).is_err());
        assert!(config(&[("COMPACTOR_REDIRECT_CACHE_TTL_SECONDS", "0")]).is_err());
    }
}
