use serde_json::json;
use serenity::client::Context;
use serenity::framework::standard::macros::command;
use serenity::framework::standard::{Args, CommandResult};
use serenity::model::channel::Message;

use crate::audio::queue;
use crate::audio::source::MediaResource;
use crate::constants;
use crate::database::get_language;
use crate::paginator::send_pages;

#[command]
#[aliases("q")]
#[only_in(guilds)]
#[description = "Get the music queue and the current playing song"]
async fn queue(ctx: &Context, msg: &Message, _args: Args) -> CommandResult {
    let guild = msg.guild(ctx).await.ok_or("Failed to fetch guild")?;
    let guild_id = guild.id;

    let lang = get_language(msg.author.id, msg.guild_id).await;
    let empty = lang.get("queue.empty");

    let queues = queue::get_queues().await;
    let queue: Vec<String> = queues
        .get(&guild_id)
        .ok_or(empty)?
        .get()
        .iter()
        .filter_map(MediaResource::title)
        .enumerate()
        .map(|(index, title)| format!("{}. {}\n", index + 1, title))
        .collect();

    if queue.is_empty() {
        return Err(empty.into());
    }

    let title = lang.translate("queue.title", json!({"guild": guild.name}))?;
    let queued_songs = lang.translate("queue.songs", json!({"total": queue.len()}))?;

    let pages = queue
        .chunks(5)
        .map(|titles| {
            let mut out = String::from("\n");
            out.push_str(&queued_songs);
            out.push('\n');
            out.push_str("```rust\n");
            out.push_str(&titles.join("\n"));
            out.push_str("```");
            out
        })
        .collect();

    send_pages(title, pages, constants::MUSIC_ICON.into(), ctx, msg).await
}
