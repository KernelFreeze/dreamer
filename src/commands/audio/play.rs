use std::lazy::SyncLazy;

use regex::Regex;
use serenity::client::Context;
use serenity::framework::standard::macros::command;
use serenity::framework::standard::{Args, CommandResult};
use serenity::model::channel::Message;
use serenity::utils::Colour;

use crate::audio::restartable::Restartable;
use crate::audio::source::{ytdl_metadata, YouTubeRestarter, YouTubeType};
use crate::spotify;

const MUSIC_ICON: &str =
    "https://cdn.discordapp.com/attachments/811977842060951613/813137572766810112/stereo.png";

async fn get_videos(query: &str) -> Vec<YouTubeType> {
    static RE: SyncLazy<Regex> = SyncLazy::new(|| {
        let regex = r"https?://(www\.)?[-a-zA-Z0-9@:%._\+~#=]{1,256}\.[a-zA-Z0-9()]{1,6}\b([-a-zA-Z0-9()@:%_\+.~#?&//=]*)";
        Regex::new(regex).expect("Failed to compile URL regex")
    });

    // Check if is a Spotify uri
    if spotify::is_spotify_url(query) {
        return spotify::get_titles(query)
            .await
            .unwrap_or_else(|_| Vec::new())
            .iter()
            .map(|track| YouTubeType::Search(track.into()))
            .collect();
    }

    // Check if is a normal uri
    if RE.is_match(query) {
        if let Ok(metadatas) = ytdl_metadata(query).await {
            return metadatas
                .iter()
                .map(|metadata| YouTubeType::Uri(metadata.clone()))
                .collect();
        }
        return Vec::new();
    }

    vec![YouTubeType::Search(query.into())]
}

#[command]
#[aliases("p")]
#[only_in(guilds)]
#[bucket = "basic"]
#[description = "Play a sound, song, playlist, or album with the provided URL or YouTube query. \
                 Supported websites are: YouTube, Spotify, Soundcloud, and 200+ more websites."]
async fn play(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    let query = args
        .remains()
        .ok_or("Must provide a URL to a video or audio, or a search query.")?;
    let videos = get_videos(&query).await;
    let videos_length = videos.len();

    let guild = msg.guild(&ctx.cache).await.unwrap();
    let guild_id = guild.id;

    let manager = songbird::get(ctx)
        .await
        .ok_or("Voice client was not initialized")?
        .clone();

    let handler_lock = manager.get(guild_id).ok_or("Not in a voice channel")?;
    let mut handler = handler_lock.lock().await;

    // Early exit if no videos found
    if videos_length <= 0 {
        return Err("No search results found.".into());
    }

    let mut video_list = videos
        .iter()
        .take(10)
        .filter_map(|audio| match audio {
            YouTubeType::Uri(metadata) => metadata.title.clone(),
            YouTubeType::Search(query) => Some(query.clone()),
        })
        .map(|element| format!("\u{279c} {}", element))
        .collect::<Vec<String>>()
        .join("\n");
    if video_list.is_empty() {
        video_list = "\u{279c} (Titles not displayed)".to_string();
    }

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

    // Show embed for every video
    msg.channel_id
        .send_message(&ctx.http, |m| {
            m.reference_message(msg);
            m.allowed_mentions(|f| f.replied_user(false));
            m.embed(|e| {
                e.author(|a| {
                    a.name(&msg.author.name);
                    a.icon_url(
                        msg.author
                            .avatar_url()
                            .unwrap_or_else(|| msg.author.default_avatar_url()),
                    );
                    a
                });
                e.thumbnail(MUSIC_ICON);
                e.color(Colour::DARK_PURPLE);
                e.title("Updated queue");
                e.field(
                    format!("Added {} elements to the queue", videos_length),
                    video_list,
                    true,
                );
                e
            });
            m
        })
        .await?;

    Ok(())
}
