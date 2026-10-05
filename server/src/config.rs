use std::{env, net::SocketAddr, path::PathBuf};

#[derive(Clone, Debug)]
pub struct Config {
    pub bind_addr: SocketAddr,
    /// Directory of built web assets. `None` in the default deployment
    /// topology, where Nginx (the `web` service) serves the SPA and the
    /// Rust server only answers `/api/*` and `/health/*`.
    pub static_dir: Option<PathBuf>,
}

impl Config {
    pub fn from_env() -> Result<Self, ConfigError> {
        let bind_addr = env::var("LT_BIND_ADDR")
            .unwrap_or_else(|_| "0.0.0.0:8080".to_owned())
            .parse()
            .map_err(|_| ConfigError::InvalidBindAddress)?;
        let static_dir = env::var("LT_STATIC_DIR").ok().map(PathBuf::from);

        Ok(Self {
            bind_addr,
            static_dir,
        })
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("LT_BIND_ADDR must be a socket address")]
    InvalidBindAddress,
}
