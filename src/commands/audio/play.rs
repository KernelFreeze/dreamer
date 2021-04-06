use std::error::Error;
use std::lazy::SyncLazy;

use queue::try_play_all;
use regex::Regex;
use serenity::client::Context;
use serenity::framework::standard::macros::command;
use serenity::framework::standard::{Args, CommandResult};
use serenity::model::channel::Message;
use serenity::utils::Colour;

use crate::audio::source::{ytdl_metadata, MediaResource};
use crate::audio::{queue, spotify};
use crate::constants::MUSIC_ICON;

async fn get_videos<S>(query: S) -> Result<Vec<MediaResource>, Box<dyn Error>>
where
    S: AsRef<str>, {
    static RE: SyncLazy<Regex> = SyncLazy::new(|| {
        let regex = r"https?://(www\.)?[-a-zA-Z0-9@:%._\+~#=]{1,256}\.[a-zA-Z0-9()]{1,6}\b([-a-zA-Z0-9()@:%_\+.~#?&//=]*)";
        Regex::new(regex).expect("Failed to compile URL regex")
    });

    // Check if is a Spotify uri
    if spotify::is_spotify_url(query.as_ref()) {
        return Ok(spotify::get_titles(query)
            .await?
            .iter()
            .map(MediaResource::with_query)
            .collect());
    }

    // Check if is a normal uri
    if RE.is_match(query.as_ref()) {
        return Ok(ytdl_metadata(query)
            .await
            .map_err(|err| format!("`youtube-dl` error {:?}", err))?);
    }

    Ok(vec![MediaResource::with_query(query)])
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
    let audios = get_videos(&query)
        .await
        .map_err(|err| format!("Failed to query: {:?}", err))?;
    let audio_len = audios.len();

    let guild = msg.guild(&ctx.cache).await.ok_or("Failed to fetch guild")?;

    let manager = songbird::get(ctx)
        .await
        .ok_or("Voice client was not initialized")?
        .clone();

    let voice_channel = guild
        .voice_states
        .get(&msg.author.id)
        .and_then(|voice_state| voice_state.channel_id)
        .ok_or("Not in a voice channel")?;

    {
        let mut queues = queue::get_queues_mut().await;
        queue::get_or_create(&mut queues, guild.id);
    }

    let queues = queue::get_queues().await;
    let queue = queue::get(&queues, guild.id).ok_or("Failed to create queue")?;

    if let Some(vc) = queue.read().await.voice_channel() {
        if voice_channel != vc {
            return Err("You are not in the current bot voice channel.".into());
        }
    }

    // Join call if not inside one
    let call = if let Some(lock) = manager.get(guild.id) {
        lock
    } else {
        let (call, result) = manager.join(guild.id, voice_channel).await;
        result?;

        queue
            .write()
            .await
            .start(
                call.clone(),
                msg.channel_id,
                guild.id,
                ctx.http.clone(),
                voice_channel,
            )
            .await?;

        call
    };

    // Deafen if not deafened
    {
        let mut handler = call.lock().await;
        if !handler.is_deaf() {
            handler.deafen(true).await?;
        }
    }

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

    // Add all audios to the queue
    for audio in audios {
        queue.write().await.add(audio);
    }

    let text_channel = msg.channel_id;

    // Show embed for every audio
    text_channel
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

    if !queue.read().await.is_playing() {
        try_play_all(guild.id, false).await?;
    }
    Ok(())
}
