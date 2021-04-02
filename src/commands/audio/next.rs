use serenity::client::Context;
use serenity::framework::standard::macros::command;
use serenity::framework::standard::{Args, CommandResult};
use serenity::model::channel::Message;

use crate::audio::queue;

#[command]
#[only_in(guilds)]
#[aliases("skip")]
async fn next(ctx: &Context, msg: &Message, _args: Args) -> CommandResult {
    let guild = msg.guild(&ctx.cache).await.ok_or("Failed to fetch guild")?;
    let guild_id = guild.id;

    let mut queues = queue::get_queues_mut().await;
    let queue = queue::get(&mut queues, guild_id);
    queue.next().await?;

    msg.reply(
        ctx,
        format!("Song skipped. {} in queue.", queue.remaining().len()),
    )
    .await?;

    Ok(())
}
