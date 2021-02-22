use std::error::Error;
use std::lazy::SyncLazy;

use regex::Regex;
use rspotify::client::{Spotify, SpotifyBuilder};
use rspotify::oauth2::CredentialsBuilder;
use serenity::client::Context;
use serenity::framework::standard::macros::command;
use serenity::framework::standard::{Args, CommandResult};
use serenity::model::channel::Message;
use serenity::prelude::RwLock;
use serenity::utils::Colour;

use crate::audio::restartable::Restartable;
use crate::audio::source::{ytdl_metadata, YouTubeRestarter, YouTubeType};

static SPOTIFY_URL: SyncLazy<Regex> = SyncLazy::new(|| {
    let regex = r"^(?:spotify:|(?:https?://(?:open|play)\.spotify\.com/))(?:embed)?/?(album|track|playlist)(?::|/)((?:[0-9a-zA-Z]){22})";
    Regex::new(regex).expect("Failed to compile Spotify URL regex")
});

async fn get_spotify(query: &str) -> Result<Vec<String>, Box<dyn Error>> {
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

    const PARSE_ERR: &'static str = "Failed to parse Spotify URI";

    let captures = SPOTIFY_URL.captures(query).ok_or(PARSE_ERR)?;
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
                .map(|track| track.track.as_ref())
                .filter_map(|x| x)
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
        _ => vec![],
    };
    Ok(result)
}

async fn get_videos(query: &str) -> Vec<YouTubeType> {
    static RE: SyncLazy<Regex> = SyncLazy::new(|| {
        let regex = r"https?://(www\.)?[-a-zA-Z0-9@:%._\+~#=]{1,256}\.[a-zA-Z0-9()]{1,6}\b([-a-zA-Z0-9()@:%_\+.~#?&//=]*)";
        Regex::new(regex).expect("Failed to compile URL regex")
    });

    // Check if is a Spotify uri
    if SPOTIFY_URL.is_match(query) {
        return get_spotify(query)
            .await
            .unwrap_or_else(|_| Vec::new())
            .iter()
            .map(|track| YouTubeType::Search(track.into()))
            .collect();
    }

    if RE.is_match(query) {
        if let Ok(metadatas) = ytdl_metadata(query).await {
            return metadatas
                .iter()
                .map(|metadata| YouTubeType::Uri(metadata.clone()))
                .collect();
        } else {
            return Vec::new();
        }
    }

    vec![YouTubeType::Search(query.into())]
}

#[command]
#[aliases("p")]
#[only_in(guilds)]
#[bucket = "basic"]
#[description = "Play a sound with the provided URL or YouTube query. Supported websites are: \
                 YouTube, Spotify, Soundcloud. Also playlists are supported."]
async fn play(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    if args.is_empty() {
        msg.reply(
            ctx,
            "Must provide a URL to a video or audio, or a search query.",
        )
        .await?;
        return Ok(());
    }

    let videos = get_videos(&args.rest()).await;
    let videos_length = videos.len();

    let guild = msg.guild(&ctx.cache).await.unwrap();
    let guild_id = guild.id;

    let manager = songbird::get(ctx)
        .await
        .ok_or("Voice client was not initialized")?
        .clone();

    if let Some(handler_lock) = manager.get(guild_id) {
        let mut handler = handler_lock.lock().await;

        // Early exit if no videos found
        if videos_length <= 0 {
            msg.channel_id
                .send_message(&ctx.http, |m| {
                    m.reference_message(msg);
                    m.allowed_mentions(|f| f.replied_user(false));
                    m.embed(|e| {
                        e.color(Colour::DARK_RED);
                        e.title("I didn't find anything!");
                        e.description("No search results found.");
                        e
                    });
                    m
                })
                .await?;
            return Ok(());
        }

        // Show embed for every video
        msg.channel_id
            .send_message(&ctx.http, |m| {
                m.reference_message(msg);
                m.allowed_mentions(|f| f.replied_user(false));
                m.embed(|e| {
                    e.author(|a| {
                        a.name(&msg.author.name);
                        a.icon_url(msg.author.avatar_url().unwrap_or(msg.author.default_avatar_url()));
                        a
                    });
                    e.thumbnail("https://cdn.discordapp.com/attachments/811977842060951613/813137572766810112/stereo.png");
                    e.color(Colour::DARK_PURPLE);
                    e.title("Updated queue");
                    e.field(
                        format!("Added {} elements to the queue", videos_length),
                        videos
                            .iter()
                            .take(15)
                            .map(|audio| match audio {
                                YouTubeType::Uri(metadata) => metadata.title.as_ref().unwrap_or(&metadata.url).clone(),
                                YouTubeType::Search(query) => query.clone(),
                            })
                            .map(|element| format!("➜ {}", element))
                            .collect::<Vec<String>>()
                            .join("\n"),
                        true,
                    );
                    e
                });
                m
            })
            .await?;

        // Add all videos to the queue
        for video in videos {
            let source = Restartable::new(YouTubeRestarter::new(video))
                .await
                .map_err(|err| format!("Failed to process a video. Error: {:?}", err))?;

            handler.enqueue_source(source.into());

            if !handler.is_deaf() {
                handler.deafen(true).await?;
            }
        }
    } else {
        msg.channel_id
            .say(&ctx.http, "Not in a voice channel to play in")
            .await?;
    }

    Ok(())
}
