use serde_json::json;
use serenity::client::Context;
use serenity::framework::standard::macros::command;
use serenity::framework::standard::{Args, CommandResult};
use serenity::model::channel::Message;

use crate::audio::queue;
use crate::utils::send_translated_info;

#[command]
#[only_in(guilds)]
async fn back(ctx: &Context, msg: &Message, _args: Args) -> CommandResult {
    let guild = msg.guild(&ctx.cache).await.ok_or("Failed to fetch guild")?;
    let guild_id = guild.id;

    let queues = queue::get_queues().await;
    let mut queue = queue::get(&queues, guild_id)
        .ok_or("No queue found for guild")?
        .write()
        .await;
    queue.back().await?;

    send_translated_info(
        "voice.update",
        "queue.back",
        json!({"remaining": queue.remaining().len()}),
        msg,
        ctx,
    )
    .await
}
