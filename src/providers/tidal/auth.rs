use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::time::{Duration, SystemTime};

const TOKEN_URL: &str = "https://auth.tidal.com/v1/oauth2/token";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenResponse {
    pub access_token: String,
    pub token_type: String,
    pub expires_in: u64,
}

#[derive(Debug, Clone)]
pub struct TidalAuth {
    client: reqwest::blocking::Client,
    client_id: String,
    client_secret: String,
    token: Option<CachedToken>,
}

#[derive(Debug, Clone)]
struct CachedToken {
    access_token: String,
    expires_at: SystemTime,
}

impl TidalAuth {
    pub fn new(client_id: String, client_secret: String) -> Self {
        Self {
            client: reqwest::blocking::Client::new(),
            client_id,
            client_secret,
            token: None,
        }
    }

    pub fn get_access_token(&mut self) -> Result<String> {
        // Check if we have a valid cached token
        if let Some(ref cached) = self.token {
            if SystemTime::now() < cached.expires_at {
                return Ok(cached.access_token.clone());
            }
        }

        // Need to fetch a new token
        let token_response = self.request_new_token()?;

        // Cache the token with expiration
        let expires_at = SystemTime::now() + Duration::from_secs(token_response.expires_in - 60); // Subtract 60s for safety
        self.token = Some(CachedToken {
            access_token: token_response.access_token.clone(),
            expires_at,
        });

        Ok(token_response.access_token)
    }

    fn request_new_token(&self) -> Result<TokenResponse> {
        let params = [
            ("grant_type", "client_credentials"),
            ("client_id", &self.client_id),
            ("client_secret", &self.client_secret),
        ];

        let response = self.client.post(TOKEN_URL).form(&params).send()?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response
                .text()
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(anyhow!(
                "Failed to get access token: {} - {}",
                status,
                error_text
            ));
        }

        let token_response: TokenResponse = response.json()?;
        Ok(token_response)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_caching() {
        // This is a unit test example - in real tests, you'd mock the HTTP client
        let auth = TidalAuth::new("test_id".to_string(), "test_secret".to_string());

        // Initially, no token should be cached
        assert!(auth.token.is_none());
    }
}
