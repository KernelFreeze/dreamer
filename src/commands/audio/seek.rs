use humantime::parse_duration;
use serenity::client::Context;
use serenity::framework::standard::macros::command;
use serenity::framework::standard::{Args, CommandResult};
use serenity::model::channel::Message;

#[command]
#[only_in(guilds)]
#[description = "Seek a portion of an audio.\n**Example:** `seek 1m 30s`"]
async fn seek(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    let guild = msg.guild(&ctx.cache).await.ok_or("Failed to fetch guild")?;
    let guild_id = guild.id;

    let manager = songbird::get(ctx)
        .await
        .ok_or("Voice client was not initialized")?
        .clone();

    let position = args
        .remains()
        .ok_or("You must provide a position to seek")?;
    let handler_lock = manager
        .get(guild_id)
        .ok_or("Not in a voice channel to play in")?;
    let handler = handler_lock.lock().await;
    let current = handler
        .queue()
        .current()
        .ok_or("Not currently playing a song")?;
    current.seek_time(parse_duration(position)?)?;

    Ok(())
}
