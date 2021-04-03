use hhmmss::Hhmmss;
use progressing::clamping::Bar;
use progressing::Baring;
use serenity::client::Context;
use serenity::framework::standard::macros::command;
use serenity::framework::standard::{Args, CommandResult};
use serenity::model::channel::Message;
use serenity::utils::Colour;

use crate::audio::queue;

#[command]
#[aliases("np", "now_playing", "nowplaying", "current")]
#[only_in(guilds)]
async fn song(ctx: &Context, msg: &Message, _args: Args) -> CommandResult {
    let guild = msg.guild(&ctx.cache).await.ok_or("Failed to fetch guild")?;
    let guild_id = guild.id;

    let queues = queue::get_queues().await;
    let queue = queue::get(&queues, guild_id)
        .ok_or("No queue found for guild")?
        .read()
        .await;

    let metadata = queue.metadata().await?;

    let position = queue.track_info().await?.position;
    let end = metadata.duration.ok_or("Failed to fetch track length")?;

    let mut progress_bar = Bar::new();
    progress_bar.set_len(35);
    progress_bar.set(position.as_secs() as f64 / end.as_secs() as f64);
    progress_bar.set_style("[\u{25ac}\u{29bf}\u{25ac}]");

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

                e.color(Colour::DARK_PURPLE);
                e.title(metadata.title.as_deref().unwrap_or("Unknown"));
                e.description(format!(
                    "```\n{} {} {}\n```",
                    position.hhmmss(),
                    progress_bar,
                    end.hhmmss(),
                ));
                e
            });
            m
        })
        .await?;
    Ok(())
}
