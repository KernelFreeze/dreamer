use serde_json::json;
use serenity::framework::standard::macros::hook;
use serenity::framework::standard::CommandError;
use serenity::model::prelude::*;
use serenity::prelude::*;
use serenity::utils::Colour;
use tracing::error;

use crate::constants::ERROR_MARK;
use crate::database::get_language;

#[hook]
pub async fn after_hook(
    ctx: &Context, msg: &Message, cmd_name: &str, error: Result<(), CommandError>,
) {
    if let Err(why) = error {
        error!("Error in command '{}': {:?}", cmd_name, why);

        let lang = get_language(msg.author.id, msg.guild_id).await;
        let title = lang.get("command.error.title");

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
                    e.author(|a| {
                        a.name(&msg.author.name);
                        a.icon_url(
                            msg.author
                                .avatar_url()
                                .unwrap_or_else(|| msg.author.default_avatar_url()),
                        );
                        a
                    });
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
