use serenity::client::Context;
use serenity::framework::standard::macros::command;
use serenity::framework::standard::{Args, CommandResult};
use serenity::model::channel::Message;

use crate::audio::queue;

#[command]
#[only_in(guilds)]
#[description = "Pause the current playing sound"]
async fn pause(ctx: &Context, msg: &Message, _args: Args) -> CommandResult {
    let guild = msg.guild(&ctx.cache).await.ok_or("Failed to fetch guild")?;
    let guild_id = guild.id;

    let queues = queue::get_queues().await;
    let mut queue = queue::get(&queues, guild_id)
        .ok_or("No queue found for guild")?
        .write()
        .await;
    queue.pause()?;
    msg.reply(ctx, "Paused sound player.").await?;

    Ok(())
}
