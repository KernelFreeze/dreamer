use std::error::Error;
use std::lazy::SyncLazy;

use regex::Regex;
use serenity::client::Context;
use serenity::framework::standard::macros::command;
use serenity::framework::standard::{Args, CommandResult};
use serenity::model::channel::Message;
use songbird::input::Restartable;

async fn get_youtube(query: &str) -> Result<String, Box<dyn Error>> {
    static RE: SyncLazy<Regex> = SyncLazy::new(|| {
        let regex = r"https?://(www\.)?[-a-zA-Z0-9@:%._\+~#=]{1,256}\.[a-zA-Z0-9()]{1,6}\b([-a-zA-Z0-9()@:%_\+.~#?&//=]*)";
        Regex::new(regex).expect("Failed to compile URL regex")
    });

    if RE.is_match(query) {
        return Ok(query.into());
    }

    let search = youtube_music::search(query).await?;
    let result = search.get(0).ok_or("No search results found")?;
    Ok(format!("https://www.youtube.com/watch?v={}", result.video_id))
}

#[command]
#[aliases("p")]
#[only_in(guilds)]
#[bucket = "basic"]
#[description = "Play a sound with the provided URL or YouTube query. Supported websites are: YouTube, \
                 Spotify, Soundcloud. Also playlists are supported."]
async fn play(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    if args.is_empty() {
        msg.reply(ctx, "Must provide a URL to a video or audio, or a search query.")
            .await?;
        return Ok(());
    }

    let url = get_youtube(&args.rest())
        .await
        .map_err(|error| format!("{:?}", error))?;

    let guild = msg.guild(&ctx.cache).await.unwrap();
    let guild_id = guild.id;

    let manager = songbird::get(ctx)
        .await
        .ok_or("Voice client was not initialized")?
        .clone();

    if let Some(handler_lock) = manager.get(guild_id) {
        let mut handler = handler_lock.lock().await;

        let source = match Restartable::ytdl(url, true).await {
            Ok(source) => source,
            Err(why) => {
                println!("Err starting source: {:?}", why);

                msg.reply(ctx, "Error sourcing ffmpeg").await?;

                return Ok(());
            },
        };

        handler.enqueue_source(source.into());

        if !handler.is_deaf() {
            handler.deafen(true).await?;
        }

        msg.reply(ctx, "Playing song").await?;
    } else {
        msg.channel_id
            .say(&ctx.http, "Not in a voice channel to play in")
            .await?;
    }

    Ok(())
}
