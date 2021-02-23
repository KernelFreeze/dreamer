use humantime::parse_duration;
use serenity::client::Context;
use serenity::framework::standard::macros::command;
use serenity::framework::standard::{Args, CommandResult};
use serenity::model::channel::Message;

use crate::audio::queue;

#[command]
#[only_in(guilds)]
#[description = "Seek a portion of an audio.\n**Example:** `seek 1min 30s`"]
async fn seek(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    let guild = msg.guild(&ctx.cache).await.ok_or("Failed to fetch guild")?;
    let guild_id = guild.id;

    let mut queues = queue::get_queues_mut().await;
    let queue = queue::get(&mut queues, guild_id);
    let position = args
        .remains()
        .ok_or("You must provide a position to seek")?;
    queue.seek(parse_duration(position)?)?;

    Ok(())
}
