use serenity::client::Context;
use serenity::framework::standard::macros::command;
use serenity::framework::standard::{Args, CommandResult};
use serenity::model::channel::Message;

use crate::audio::queue;

#[command]
#[only_in(guilds)]
async fn volume(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let guild = msg.guild(&ctx.cache).await.ok_or("Failed to fetch guild")?;
    let guild_id = guild.id;

    let volume = args.single::<usize>()?;

    if !(0..=200).contains(&volume) {
        Err("Volume must be in range from 0 to 100")?;
    }

    let mut queues = queue::get_queues_mut().await;
    let queue = queue::get(&mut queues, guild_id);
    queue.volume(volume as f32 / 100.0)?;
    msg.reply(ctx, format!("Set volume to {}%", volume)).await?;

    Ok(())
}
