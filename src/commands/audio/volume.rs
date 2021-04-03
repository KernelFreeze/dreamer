use serde_json::json;
use serenity::client::Context;
use serenity::framework::standard::macros::command;
use serenity::framework::standard::{Args, CommandResult};
use serenity::model::channel::Message;

use crate::audio::queue;
use crate::utils::send_translated_info;

#[command]
#[only_in(guilds)]
async fn volume(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let guild = msg.guild(&ctx.cache).await.ok_or("Failed to fetch guild")?;
    let guild_id = guild.id;

    let volume = args.single::<u8>()?;

    if !(0..=200).contains(&volume) {
        return Err("Volume must be in range from 0 to 200".into());
    }

    let queues = queue::get_queues().await;
    let mut queue = queue::get(&queues, guild_id)
        .ok_or("No queue found for guild")?
        .write()
        .await;
    queue.volume(f32::from(volume) / 100.0)?;
    msg.reply(ctx, format!("Set volume to {}%", volume)).await?;

    send_translated_info(
        "voice.update",
        "queue.volume",
        json!({"volume": volume}),
        msg,
        ctx,
    )
    .await
}
