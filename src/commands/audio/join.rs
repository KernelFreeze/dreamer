use serenity::client::Context;
use serenity::framework::standard::macros::command;
use serenity::framework::standard::CommandResult;
use serenity::model::channel::Message;

#[command]
#[only_in(guilds)]
#[bucket = "basic"]
#[description = "Join a voice channel, to be able to play sounds and videos."]
async fn join(ctx: &Context, msg: &Message) -> CommandResult {
    let guild = msg.guild(&ctx.cache).await.unwrap();
    let guild_id = guild.id;

    let channel_id = guild
        .voice_states
        .get(&msg.author.id)
        .and_then(|voice_state| voice_state.channel_id);

    let connect_to = if let Some(channel) = channel_id {
        channel
    } else {
        msg.reply(ctx, "Not in a voice channel").await?;
        return Ok(());
    };

    let manager = songbird::get(ctx)
        .await
        .ok_or("Voice client was not initialized")?
        .clone();

    let (call, result) = manager.join(guild_id, connect_to).await;
    result?;

    let mut handler = call.lock().await;
    if !handler.is_deaf() {
        handler.deafen(true).await?;
    }

    Ok(())
}
