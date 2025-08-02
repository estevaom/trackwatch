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

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    // Use a mutex to ensure tests don't interfere with each other
    static ENV_MUTEX: Mutex<()> = Mutex::new(());

    #[test]
    fn test_from_env_no_credentials() {
        let _guard = ENV_MUTEX.lock().unwrap();

        // Clear any existing env vars
        std::env::remove_var("TIDAL_CLIENT_ID");
        std::env::remove_var("TIDAL_CLIENT_SECRET");

        let config = Config::from_env();
        assert!(config.tidal_client_id.is_none());
        assert!(config.tidal_client_secret.is_none());
    }

    #[test]
    fn test_from_env_with_credentials() {
        let _guard = ENV_MUTEX.lock().unwrap();

        // Clear first to ensure clean state
        std::env::remove_var("TIDAL_CLIENT_ID");
        std::env::remove_var("TIDAL_CLIENT_SECRET");

        // Set env vars
        std::env::set_var("TIDAL_CLIENT_ID", "test_id");
        std::env::set_var("TIDAL_CLIENT_SECRET", "test_secret");

        let config = Config::from_env();
        assert_eq!(config.tidal_client_id, Some("test_id".to_string()));
        assert_eq!(config.tidal_client_secret, Some("test_secret".to_string()));

        // Clean up
        std::env::remove_var("TIDAL_CLIENT_ID");
        std::env::remove_var("TIDAL_CLIENT_SECRET");
    }

    #[test]
    fn test_from_env_partial_credentials() {
        let _guard = ENV_MUTEX.lock().unwrap();

        // Clear first
        std::env::remove_var("TIDAL_CLIENT_ID");
        std::env::remove_var("TIDAL_CLIENT_SECRET");

        // Only set one env var
        std::env::set_var("TIDAL_CLIENT_ID", "test_id");

        let config = Config::from_env();
        assert_eq!(config.tidal_client_id, Some("test_id".to_string()));
        assert!(config.tidal_client_secret.is_none());

        // Clean up
        std::env::remove_var("TIDAL_CLIENT_ID");
    }

    #[test]
    fn test_has_tidal_credentials_both_present() {
        let config = Config {
            tidal_client_id: Some("id".to_string()),
            tidal_client_secret: Some("secret".to_string()),
        };
        assert!(config.has_tidal_credentials());
    }

    #[test]
    fn test_has_tidal_credentials_none_present() {
        let config = Config {
            tidal_client_id: None,
            tidal_client_secret: None,
        };
        assert!(!config.has_tidal_credentials());
    }

    #[test]
    fn test_has_tidal_credentials_only_id() {
        let config = Config {
            tidal_client_id: Some("id".to_string()),
            tidal_client_secret: None,
        };
        assert!(!config.has_tidal_credentials());
    }

    #[test]
    fn test_has_tidal_credentials_only_secret() {
        let config = Config {
            tidal_client_id: None,
            tidal_client_secret: Some("secret".to_string()),
        };
        assert!(!config.has_tidal_credentials());
    }
}
