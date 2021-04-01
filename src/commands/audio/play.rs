use std::lazy::SyncLazy;

use regex::Regex;
use serenity::client::Context;
use serenity::framework::standard::macros::command;
use serenity::framework::standard::{Args, CommandResult};
use serenity::model::channel::Message;
use serenity::utils::Colour;

use crate::audio::queue;
use crate::audio::source::{ytdl_metadata, MediaResource};
use crate::constants::MUSIC_ICON;
use crate::spotify;

async fn get_videos<S>(query: S) -> Vec<MediaResource>
where
    S: AsRef<str>,
{
    static RE: SyncLazy<Regex> = SyncLazy::new(|| {
        let regex = r"https?://(www\.)?[-a-zA-Z0-9@:%._\+~#=]{1,256}\.[a-zA-Z0-9()]{1,6}\b([-a-zA-Z0-9()@:%_\+.~#?&//=]*)";
        Regex::new(regex).expect("Failed to compile URL regex")
    });

    // Check if is a Spotify uri
    if spotify::is_spotify_url(query.as_ref()) {
        return spotify::get_titles(query)
            .await
            .unwrap_or_else(|_| Vec::new())
            .iter()
            .map(MediaResource::with_query)
            .collect();
    }

    // Check if is a normal uri
    if RE.is_match(query.as_ref()) {
        return ytdl_metadata(query).await.unwrap_or_else(|_| Vec::new());
    }

    vec![MediaResource::with_query(query)]
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
    let audios = get_videos(&query).await;
    let audio_len = audios.len();

    let guild = msg.guild(&ctx.cache).await.ok_or("Failed to fetch guild")?;
    let guild_id = guild.id;

    let manager = songbird::get(ctx)
        .await
        .ok_or("Voice client was not initialized")?
        .clone();
    let handler_lock = manager.get(guild_id).ok_or("Not in a voice channel")?;
    let channel = msg.channel_id;

    // Early exit if no videos found
    if audio_len == 0 {
        return Err("No search results found.".into());
    }

    let mut audio_list = audios
        .iter()
        .take(10)
        .filter_map(|youtube| Some(format!("\u{279c} {}", youtube.title()?)))
        .collect::<Vec<String>>()
        .join("\n");
    if audio_list.is_empty() {
        audio_list = "\u{279c} (Titles not displayed)".to_string();
    }

    let mut queues = queue::get_queues_mut().await;
    let queue = queue::get(&mut queues, guild_id);

    // Add all videos to the queue
    for audio in audios {
        queue.add(audio);
    }

    // Show embed for every video
    channel
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
                    format!("Added {} elements to the queue", audio_len),
                    audio_list,
                    true,
                );
                e
            });
            m
        })
        .await?;

    queue
        .start(handler_lock.clone(), channel, guild_id, ctx.http.clone())
        .await?;

    Ok(())
}
