use serenity::client::Context;
use serenity::framework::standard::macros::command;
use serenity::framework::standard::{Args, CommandResult};
use serenity::model::channel::Message;

#[command]
#[only_in(guilds)]
#[description = "Pause the current playing sound"]
async fn pause(ctx: &Context, msg: &Message, _args: Args) -> CommandResult {
    let guild = msg.guild(&ctx.cache).await.unwrap();
    let guild_id = guild.id;

    let manager = songbird::get(ctx)
        .await
        .ok_or("Voice client was not initialized")?
        .clone();
    let handler_lock = manager.get(guild_id).ok_or("Not in a voice channel")?;
    let handler = handler_lock.lock().await;
    let queue = handler.queue();
    queue.pause()?;
    msg.reply(ctx, "Paused sound player.").await?;

    Ok(())
}
