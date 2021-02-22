use log::error;
use serde_json::json;
use serenity::framework::standard::macros::hook;
use serenity::framework::standard::CommandError;
use serenity::model::prelude::*;
use serenity::prelude::*;
use serenity::utils::Colour;

use crate::database::get_language;

const ERROR_MARK: &str = "https://media.discordapp.net/attachments/811977842060951613/813408904750694440/016-caution.png";

#[hook]
pub async fn after_hook(
    ctx: &Context, msg: &Message, cmd_name: &str, error: Result<(), CommandError>,
) {
    if let Err(why) = error {
        error!("Error in command '{}': {:?}", cmd_name, why);

        let lang = get_language(msg.author.id, msg.guild_id).await;
        let title = lang.get("command.error.title").unwrap_or("Error");

        let text = lang
            .translate(
                "command.error.internal",
                json!({ "error": why.to_string() }),
            )
            .unwrap_or_else(|_| "Command failed".into());

        let err = msg
            .channel_id
            .send_message(&ctx.http, |m| {
                m.reference_message(msg);
                m.allowed_mentions(|f| f.replied_user(false));
                m.embed(|e| {
                    e.color(Colour::DARK_RED);
                    e.thumbnail(ERROR_MARK);
                    e.title(title);
                    e.description(text);
                    e
                });
                m
            })
            .await;

        if let Err(why) = err {
            error!("Error sending a message: {:?}", why);
        }
    }
}

#[hook]
pub async fn delay_action(ctx: &Context, msg: &Message) {
    // You may want to handle a Discord rate limit if this fails.
    if let Err(why) = msg.react(ctx, '⏱').await {
        error!("Failed to insert delay reaction: {:?}", why);
    }
}
