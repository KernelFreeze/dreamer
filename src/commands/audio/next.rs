use serenity::client::Context;
use serenity::framework::standard::macros::command;
use serenity::framework::standard::{Args, CommandResult};
use serenity::model::channel::Message;

#[command]
#[only_in(guilds)]
#[aliases("skip")]
async fn next(ctx: &Context, msg: &Message, _args: Args) -> CommandResult {
    let guild = msg.guild(&ctx.cache).await.unwrap();
    let guild_id = guild.id;

    let manager = songbird::get(ctx)
        .await
        .ok_or("Voice client was not initialized")?
        .clone();

    let handler_lock = manager.get(guild_id).ok_or("Not in a voice channel")?;
    let handler = handler_lock.lock().await;
    let queue = handler.queue();
    queue.skip()?;

    msg.reply(ctx, format!("Song skipped: {} in queue.", queue.len() - 1))
        .await?;

    Ok(())
}
