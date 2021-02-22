use serenity::async_trait;
use serenity::client::{Context, EventHandler};
use serenity::model::gateway::Ready;
use serenity::model::prelude::*;
use tracing::info;

pub struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, ctx: Context, ready: Ready) {
        ctx.set_presence(Some(Activity::playing(".help")), OnlineStatus::DoNotDisturb)
            .await;
        info!("{} connected!", ready.user.name);
    }
}
