use serenity::client::Context;
use serenity::framework::standard::CommandResult;
use serenity::model::channel::Message;
use serenity::utils::Colour;

use crate::database::get_language;

pub async fn send_info<S>(title: S, description: S, msg: &Message, ctx: &Context) -> CommandResult
where
    S: AsRef<str>,
{
    let lang = get_language(msg.author.id, msg.guild_id).await;
    let title = lang.get(title.as_ref());
    let description = lang.get(description.as_ref());

    msg.channel_id
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
                e.color(Colour::DARK_BLUE);
                e.title(title);
                e.description(description);
                e
            });
            m
        })
        .await?;
    Ok(())
}
