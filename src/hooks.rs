use log::error;
use serde_json::json;
use serenity::framework::standard::macros::hook;
use serenity::framework::standard::CommandError;
use serenity::model::prelude::*;
use serenity::prelude::*;

use crate::database::get_language;

#[hook]
pub async fn after_hook(ctx: &Context, msg: &Message, cmd_name: &str, error: Result<(), CommandError>) {
    if let Err(why) = error {
        error!("Error in command '{}': {:?}", cmd_name, why);

        let text = get_language(msg.author.id, msg.guild_id)
            .await
            .translate("command.error.internal", json!({ "error": why.to_string() }))
            .unwrap_or("Command failed".into());

        if let Err(why) = msg.reply(ctx, text).await {
            error!("Error sending a message: {:?}", why);
        }
    }
}

#[hook]
pub async fn delay_action(ctx: &Context, msg: &Message) {
    // You may want to handle a Discord rate limit if this fails.
    let _ = msg.react(ctx, '⏱').await;
}
