
use std::time::Duration;

use hhmmss::Hhmmss;
use serde_json::json;
use serenity::client::Context;
use serenity::framework::standard::macros::command;
use serenity::framework::standard::{Args, CommandResult};
use serenity::model::channel::Message;

use crate::audio::queue;
use crate::utils::send_translated_info;

#[command]
#[only_in(guilds)]
#[description = "Fast forward a portion of an audio.\nUses 5 seconds by default\n**Example:** `ff 30`"]
#[aliases("ff", "fastforward", "advance")]
async fn fast_forward(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let guild = msg.guild(&ctx.cache).await.ok_or("Failed to fetch guild")?;
    let guild_id = guild.id;

    let queues = queue::get_queues().await;
    let mut queue = queue::get(&queues, guild_id)
        .ok_or("No queue found for guild")?
        .write()
        .await;

    let mut position = queue.track_info().await?.position;
    position += Duration::new(args.single().unwrap_or(5), 0);

    queue.seek(position)?;

    send_translated_info(
        "voice.update",
        "audio.seek",
        json!({ "position": position.hhmmss() }),
        msg,
        ctx,
    )
    .await
}
