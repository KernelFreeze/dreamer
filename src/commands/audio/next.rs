use queue::try_play_all;
use serde_json::json;
use serenity::client::Context;
use serenity::framework::standard::macros::command;
use serenity::framework::standard::{Args, CommandResult};
use serenity::model::channel::Message;

use crate::audio::queue;
use crate::utils::send_translated_info;

#[command]
#[only_in(guilds)]
#[aliases("skip")]
async fn next(ctx: &Context, msg: &Message, _args: Args) -> CommandResult {
    let guild = msg.guild(&ctx.cache).await.ok_or("Failed to fetch guild")?;
    let guild_id = guild.id;

    try_play_all(guild_id, true).await?;

    let queues = queue::get_queues().await;
    let queue = queue::get(&queues, guild_id).ok_or("There are no active queue for this server")?;

    send_translated_info(
        "voice.update",
        "queue.next",
        json!({"remaining": queue.read().await.remaining().len()}),
        msg,
        ctx,
    )
    .await
}
