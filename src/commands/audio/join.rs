use serenity::client::Context;
use serenity::framework::standard::macros::command;
use serenity::framework::standard::CommandResult;
use serenity::model::channel::Message;

use crate::audio::queue;
use crate::utils::send_info;

#[command]
#[aliases("j")]
#[only_in(guilds)]
#[bucket = "basic"]
#[description = "Join a voice channel, to be able to play sounds and videos."]
async fn join(ctx: &Context, msg: &Message) -> CommandResult {
    let guild = msg.guild(&ctx.cache).await.ok_or("Failed to fetch guild")?;

    let voice_channel = guild
        .voice_states
        .get(&msg.author.id)
        .and_then(|voice_state| voice_state.channel_id)
        .ok_or("Not in a voice channel")?;

    let manager = songbird::get(ctx)
        .await
        .ok_or("Voice client was not initialized")?
        .clone();

    let (call, result) = manager.join(guild.id, voice_channel).await;
    result?;

    let mut handler = call.lock().await;
    if !handler.is_deaf() {
        handler.deafen(true).await?;
    }

    {
        let mut queues = queue::get_queues_mut().await;
        queue::get_or_create(&mut queues, guild.id);
    }

    let queues = queue::get_queues().await;
    let queue = queue::get(&queues, guild.id).ok_or("Failed to create queue")?;

    queue
        .write()
        .await
        .start(call.clone(), msg.channel_id, guild.id, ctx.http.clone(), voice_channel)
        .await?;

    send_info("voice.update", "voice.joined", msg, ctx).await
}
