use serenity::client::bridge::gateway::ShardId;
use serenity::client::Context;
use serenity::framework::standard::macros::command;
use serenity::framework::standard::CommandResult;
use serenity::model::channel::Message;

use crate::ShardManagerContainer;

#[command]
#[aliases("lag")]
#[bucket = "basic"]
#[description = "Check bot shard latency with the Discord Gateway."]
async fn latency(ctx: &Context, msg: &Message) -> CommandResult {
    // The shard manager is an interface for mutating, stopping, restarting, and
    // retrieving information about shards.
    let data = ctx.data.read().await;

    let shard_manager = if let Some(v) = data.get::<ShardManagerContainer>() {
        v
    } else {
        msg.reply(ctx, "There was a problem getting the shard manager")
            .await?;
        return Ok(());
    };

    let manager = shard_manager.lock().await;
    let runners = manager.runners.lock().await;

    // Shards are backed by a "shard runner" responsible for processing events
    // over the shard, so we'll get the information about the shard runner for
    // the shard this command was sent over.
    let runner = if let Some(runner) = runners.get(&ShardId(ctx.shard_id)) {
        runner
    } else {
        msg.reply(ctx, "No shard found").await?;
        return Ok(());
    };

    if let Some(latency) = runner.latency {
        msg.reply(ctx, &format!("The shard latency is {:?}", latency))
            .await?;
    } else {
        msg.reply(ctx, "Latency data is not available.").await?;
    }

    Ok(())
}
