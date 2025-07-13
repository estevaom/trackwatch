mod api;
mod auth;

use self::api::TidalApi;
use crate::models::AlbumMetadata;
use crate::providers::MusicProvider;
use anyhow::Result;

pub struct TidalProvider {
    api: TidalApi,
}

impl TidalProvider {
    pub fn new(client_id: String, client_secret: String) -> Self {
        Self {
            api: TidalApi::new(client_id, client_secret),
        }
    }
}

impl MusicProvider for TidalProvider {
    fn get_album_metadata(&mut self, artist: &str, album: &str) -> Result<AlbumMetadata> {
        self.api.search_album(artist, album)
    }
}
