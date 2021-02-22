use serenity::client::Context;
use serenity::framework::standard::macros::command;
use serenity::framework::standard::CommandResult;
use serenity::model::channel::Message;

#[command]
#[aliases("l", "quit", "exit", "part")]
#[only_in(guilds)]
#[bucket = "basic"]
#[description = "Leave a voice channel."]
async fn leave(ctx: &Context, msg: &Message) -> CommandResult {
    let guild = msg.guild(&ctx.cache).await.ok_or("Failed to fetch guild")?;
    let guild_id = guild.id;

    let manager = songbird::get(ctx)
        .await
        .ok_or("Voice client was not initialized")?
        .clone();

    manager.get(guild_id).ok_or("Not in a voice channel")?;
    manager.remove(guild_id).await?;

    Ok(())
}
