use serenity::client::Context;
use serenity::framework::standard::macros::command;
use serenity::framework::standard::{Args, CommandResult};
use serenity::model::channel::Message;

use crate::audio::queue;
use crate::utils::send_info;

#[command]
#[only_in(guilds)]
#[description = "Shuffle the queue"]
async fn shuffle(ctx: &Context, msg: &Message, _args: Args) -> CommandResult {
    let guild = msg.guild(&ctx.cache).await.ok_or("Failed to fetch guild")?;
    let guild_id = guild.id;

    let mut queues = queue::get_queues_mut().await;
    let queue = queue::get(&mut queues, guild_id);
    queue.shuffle();

    send_info("voice.update", "queue.shuffled", msg, ctx).await?;

    Ok(())
}
