use serenity::client::Context;
use serenity::framework::standard::macros::command;
use serenity::framework::standard::{Args, CommandResult};
use serenity::model::channel::Message;

use crate::errors::check_msg;

#[command]
#[only_in(guilds)]
#[bucket = "basic"]
async fn play(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let url = if let Ok(url) = args.single::<String>() {
        url
    } else {
        check_msg(
            msg.channel_id
                .say(&ctx.http, "Must provide a URL to a video or audio")
                .await,
        );

        return Ok(());
    };

    if !url.starts_with("http") {
        check_msg(msg.channel_id.say(&ctx.http, "Must provide a valid URL").await);

        return Ok(());
    }

    let guild = msg.guild(&ctx.cache).await.unwrap();
    let guild_id = guild.id;

    let manager = songbird::get(ctx)
        .await
        .ok_or("Voice client was not initialized")?
        .clone();

    if let Some(handler_lock) = manager.get(guild_id) {
        let mut handler = handler_lock.lock().await;

        let source = match songbird::ytdl(&url).await {
            Ok(source) => source,
            Err(why) => {
                println!("Err starting source: {:?}", why);

                check_msg(msg.reply(ctx, "Error sourcing ffmpeg").await);

                return Ok(());
            },
        };

        handler.play_source(source);

        if !handler.is_deaf() {
            handler.deafen(true).await?;
        }

        check_msg(msg.reply(ctx, "Playing song").await);
    } else {
        check_msg(
            msg.channel_id
                .say(&ctx.http, "Not in a voice channel to play in")
                .await,
        );
    }

    Ok(())
}
