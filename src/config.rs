use anyhow::{anyhow, Result};

#[derive(Debug, Clone)]
pub struct Config {
    pub tidal_client_id: String,
    pub tidal_client_secret: String,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        let tidal_client_id = std::env::var("TIDAL_CLIENT_ID")
            .map_err(|_| anyhow!("TIDAL_CLIENT_ID environment variable not set"))?;

        let tidal_client_secret = std::env::var("TIDAL_CLIENT_SECRET")
            .map_err(|_| anyhow!("TIDAL_CLIENT_SECRET environment variable not set"))?;

        Ok(Self {
            tidal_client_id,
            tidal_client_secret,
        })
    }
}
