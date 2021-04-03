use hhmmss::Hhmmss;
use humantime::parse_duration;
use serde_json::json;
use serenity::client::Context;
use serenity::framework::standard::macros::command;
use serenity::framework::standard::{Args, CommandResult};
use serenity::model::channel::Message;

use crate::audio::queue;
use crate::utils::send_translated_info;

#[command]
#[only_in(guilds)]
#[description = "Seek a portion of an audio.\n**Example:** `seek 1min 30s`"]
async fn seek(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    let guild = msg.guild(&ctx.cache).await.ok_or("Failed to fetch guild")?;
    let guild_id = guild.id;

    let queues = queue::get_queues().await;
    let mut queue = queue::get(&queues, guild_id)
        .ok_or("No queue found for guild")?
        .write()
        .await;
    let position = args
        .remains()
        .ok_or("You must provide a position to seek")?;
    queue.seek(parse_duration(position)?)?;

    let position = queue.track_info().await?.position;

    send_translated_info(
        "voice.update",
        "audio.seek",
        json!({ "position": position.hhmmss() }),
        msg,
        ctx,
    )
    .await
}
