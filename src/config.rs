use std::net::SocketAddr;

#[derive(Debug, Clone)]
pub struct Config {
    pub bind_addr: SocketAddr,
    pub log_level: String,
    pub environment: String,
    /// Base URL of the srvcs-greaterthanorequalto dependency.
    pub greaterthanorequalto_url: String,
    /// Base URL of the srvcs-lessthanorequalto dependency.
    pub lessthanorequalto_url: String,
    /// Base URL of the srvcs-and dependency.
    pub and_url: String,
}

impl Config {
    #[allow(clippy::too_many_arguments)]
    pub fn from_vars(
        bind: Option<String>,
        log: Option<String>,
        env: Option<String>,
        greaterthanorequalto_url: Option<String>,
        lessthanorequalto_url: Option<String>,
        and_url: Option<String>,
    ) -> Self {
        let bind_addr = bind
            .unwrap_or_else(|| "0.0.0.0:8080".to_string())
            .parse()
            .expect("SRVCS_BIND_ADDR must be host:port");
        Config {
            bind_addr,
            log_level: log.unwrap_or_else(|| "info,tower_http=info".to_string()),
            environment: env.unwrap_or_else(|| "development".to_string()),
            greaterthanorequalto_url: greaterthanorequalto_url
                .unwrap_or_else(|| "http://127.0.0.1:8084".to_string()),
            lessthanorequalto_url: lessthanorequalto_url
                .unwrap_or_else(|| "http://127.0.0.1:8085".to_string()),
            and_url: and_url.unwrap_or_else(|| "http://127.0.0.1:8086".to_string()),
        }
    }

    pub fn from_env() -> Self {
        Self::from_vars(
            std::env::var("SRVCS_BIND_ADDR").ok(),
            std::env::var("RUST_LOG").ok(),
            std::env::var("SRVCS_ENV").ok(),
            std::env::var("SRVCS_GREATERTHANOREQUALTO_URL").ok(),
            std::env::var("SRVCS_LESSTHANOREQUALTO_URL").ok(),
            std::env::var("SRVCS_AND_URL").ok(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_sane() {
        let c = Config::from_vars(None, None, None, None, None, None);
        assert_eq!(c.bind_addr.port(), 8080);
        assert_eq!(c.greaterthanorequalto_url, "http://127.0.0.1:8084");
        assert_eq!(c.lessthanorequalto_url, "http://127.0.0.1:8085");
        assert_eq!(c.and_url, "http://127.0.0.1:8086");
    }

    #[test]
    fn parses_explicit_bind_addr() {
        let c = Config::from_vars(Some("127.0.0.1:9000".into()), None, None, None, None, None);
        assert_eq!(c.bind_addr.port(), 9000);
    }
}
