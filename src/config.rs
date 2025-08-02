#[derive(Debug, Clone)]
pub struct Config {
    pub tidal_client_id: Option<String>,
    pub tidal_client_secret: Option<String>,
}

impl Config {
    pub fn from_env() -> Self {
        let tidal_client_id = std::env::var("TIDAL_CLIENT_ID").ok();
        let tidal_client_secret = std::env::var("TIDAL_CLIENT_SECRET").ok();

        Self {
            tidal_client_id,
            tidal_client_secret,
        }
    }

    pub fn has_tidal_credentials(&self) -> bool {
        self.tidal_client_id.is_some() && self.tidal_client_secret.is_some()
    }
}
