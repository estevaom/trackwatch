use crate::config::Config;
use crate::providers::tidal::TidalProvider;

pub fn create_tidal_provider(config: &Config) -> Option<TidalProvider> {
    if config.has_tidal_credentials() {
        Some(TidalProvider::new(
            config.tidal_client_id.clone().unwrap(),
            config.tidal_client_secret.clone().unwrap(),
        ))
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_tidal_provider_with_credentials() {
        let config = Config {
            tidal_client_id: Some("test_id".to_string()),
            tidal_client_secret: Some("test_secret".to_string()),
        };

        let provider = create_tidal_provider(&config);
        assert!(provider.is_some());
    }

    #[test]
    fn test_create_tidal_provider_without_credentials() {
        let config = Config {
            tidal_client_id: None,
            tidal_client_secret: None,
        };

        let provider = create_tidal_provider(&config);
        assert!(provider.is_none());
    }

    #[test]
    fn test_create_tidal_provider_partial_credentials_id_only() {
        let config = Config {
            tidal_client_id: Some("test_id".to_string()),
            tidal_client_secret: None,
        };

        let provider = create_tidal_provider(&config);
        assert!(provider.is_none());
    }

    #[test]
    fn test_create_tidal_provider_partial_credentials_secret_only() {
        let config = Config {
            tidal_client_id: None,
            tidal_client_secret: Some("test_secret".to_string()),
        };

        let provider = create_tidal_provider(&config);
        assert!(provider.is_none());
    }
}
