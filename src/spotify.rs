use std::error::Error;
use std::lazy::SyncLazy;

use regex::Regex;
use rspotify::client::{Spotify, SpotifyBuilder};
use rspotify::oauth2::CredentialsBuilder;
use serenity::prelude::RwLock;

static SPOTIFY_URL: SyncLazy<Regex> = SyncLazy::new(|| {
    let regex = r"^(?:spotify:|(?:https?://(?:open|play)\.spotify\.com/))(?:embed)?/?(album|track|playlist)(?::|/)((?:[0-9a-zA-Z]){22})";
    Regex::new(regex).expect("Failed to compile Spotify URL regex")
});

const PARSE_ERR: &str = "Failed to parse Spotify URI";

pub fn is_spotify_url<S>(url: S) -> bool
where
    S: AsRef<str>, {
    SPOTIFY_URL.is_match(url.as_ref())
}

pub async fn get_titles<S>(query: S) -> Result<Vec<String>, Box<dyn Error>>
where
    S: AsRef<str>, {
    static SPOTIFY: SyncLazy<RwLock<Spotify>> = SyncLazy::new(|| {
        let creds = CredentialsBuilder::from_env()
            .build()
            .expect("Failed to load Spotify client");
        RwLock::new(
            SpotifyBuilder::default()
                .credentials(creds)
                .build()
                .expect("Failed to load Spotify client"),
        )
    });
    SPOTIFY
        .write()
        .await
        .request_client_token()
        .await
        .expect("Failed to fetch Spotify client token");

    let client = SPOTIFY.read().await;

    let captures = SPOTIFY_URL.captures(query.as_ref()).ok_or(PARSE_ERR)?;
    let id = captures.get(2).ok_or(PARSE_ERR)?.as_str();
    let result = match captures.get(1).ok_or(PARSE_ERR)?.as_str() {
        "album" => {
            let tracks = client.album_track(id, None, None).await?;

            tracks
                .items
                .iter()
                .map(|track| {
                    let artists: Vec<String> = track
                        .artists
                        .iter()
                        .map(|artist| artist.name.clone())
                        .collect();
                    format!("{} - {}", artists.join(", "), track.name)
                })
                .collect()
        },
        "track" => {
            let track = client.track(id).await?;
            let artists: Vec<String> = track
                .artists
                .iter()
                .map(|artist| artist.name.clone())
                .collect();
            vec![format!("{} - {}", artists.join(", "), track.name)]
        },
        "playlist" => {
            let playlist = client.playlist(id, None, None).await?;

            playlist
                .tracks
                .items
                .iter()
                .filter_map(|playlist_item| {
                    let track = playlist_item.track.as_ref()?;
                    let artists: Vec<String> = track
                        .artists
                        .iter()
                        .map(|artist| artist.name.clone())
                        .collect();
                    Some(format!("{} - {}", artists.join(", "), track.name))
                })
                .collect()
        },
        _ => vec![],
    };
    Ok(result)
}
