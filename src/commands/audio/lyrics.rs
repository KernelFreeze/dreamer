use serenity::client::Context;
use serenity::framework::standard::macros::command;
use serenity::framework::standard::{Args, CommandError, CommandResult};
use serenity::model::channel::Message;
use serenity::utils::Colour;

use crate::{audio::queue, constants::LYRICS_ICON};

async fn get_current_song(ctx: &Context, msg: &Message) -> Result<String, CommandError> {
    let guild = msg.guild(&ctx.cache).await.ok_or("Failed to fetch guild")?;
    let guild_id = guild.id;

    let mut queues = queue::get_queues_mut().await;
    let current = queue::get(&mut queues, guild_id)
        .current_mut()
        .ok_or("Failed to fetch current song")?;
    let title = current.title().ok_or("Failed to fetch current song")?;
    Ok(title.into())
}

#[command]
#[only_in(guilds)]
#[aliases("letra")]
async fn lyrics(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    let name = match args.remains() {
        Some(t) => Some(t.to_string()),
        None => get_current_song(ctx, msg).await.ok(),
    };
    let name = name.ok_or("You must provide a song name to search!")?;
    let lyrics = lyrics::search(name).await.map_err(|_| "Song not found!")?;

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

                e.thumbnail(LYRICS_ICON);
                e.color(Colour::DARK_PURPLE);
                e.title(lyrics.title);
                e.description(lyrics.lyrics);
                e
            });
            m
        })
        .await?;
    Ok(())
}
