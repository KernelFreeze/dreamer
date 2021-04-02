use serenity::client::Context;
use serenity::framework::standard::macros::command;
use serenity::framework::standard::{Args, CommandError, CommandResult};
use serenity::model::channel::Message;

use crate::audio::queue;
use crate::constants::{self};
use crate::paginator::send_pages;

async fn get_current_song(ctx: &Context, msg: &Message) -> Result<String, CommandError> {
    let guild = msg.guild(&ctx.cache).await.ok_or("Failed to fetch guild")?;
    let guild_id = guild.id;

    let queues = queue::get_queues().await;
    let current = queue::get_option(&queues, guild_id)
        .ok_or("Current queue is empty")?
        .current()
        .ok_or("Failed to fetch current song")?;
    let title = current.title().ok_or("Failed to fetch current song")?;
    Ok(title)
}

#[command]
#[only_in(guilds)]
#[aliases("letra")]
async fn lyrics(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    let name = match args.remains() {
        Some(t) => Some(t.to_string()),
        None => get_current_song(ctx, msg).await.ok(),
    };
    let name = name.ok_or("You must provide a song name to search!")?;
    let lyrics = lyrics::search(name).await.map_err(|_| "Song not found!")?;
    let lines: Vec<&str> = lyrics.lyrics.lines().collect();
    let text = lines.chunks(30).map(|lines| lines.join("\n")).collect();

    send_pages(lyrics.title, text, constants::LYRICS_ICON.into(), ctx, msg).await
}
