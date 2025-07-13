use crate::models::AlbumMetadata;
use anyhow::Result;

pub mod tidal;

// For watch mode, we only need album metadata
pub trait MusicProvider {
    fn get_album_metadata(&mut self, artist: &str, album: &str) -> Result<AlbumMetadata>;
}
