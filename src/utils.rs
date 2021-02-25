use serenity::client::Context;
use serenity::framework::standard::CommandResult;
use serenity::model::channel::Message;
use serenity::utils::Colour;

pub async fn send_info<S>(title: S, description: S, msg: &Message, ctx: &Context) -> CommandResult
where
    S: AsRef<str>, {
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
                e.title(title.as_ref());
                e.description(description.as_ref());
                e
            });
            m
        })
        .await?;
    Ok(())
}
