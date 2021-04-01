use std::time::Duration;

use async_recursion::async_recursion;
use serde_json::json;
use serenity::builder::CreateEmbed;
use serenity::client::Context;
use serenity::framework::standard::CommandResult;
use serenity::model::channel::Message;
use serenity::utils::Colour;

use crate::database::get_language;

fn create_embed<'a>(
    e: &'a mut CreateEmbed, msg: &Message, title: &String, description: &String,
    thumbnail: &String, footer: &String,
) -> &'a mut CreateEmbed {
    e.author(|a| {
        a.name(&msg.author.name);
        a.icon_url(
            msg.author
                .avatar_url()
                .unwrap_or_else(|| msg.author.default_avatar_url()),
        );
        a
    });
    e.thumbnail(thumbnail);
    e.color(Colour::DARK_PURPLE);
    e.title(&title);
    e.description(description);
    e.footer(|f| f.text(footer));
    e
}

#[async_recursion]
async fn _send_page(
    title: String, pages: Vec<String>, thumbnail: String, page: usize, ctx: &Context,
    msg: &Message, paginated: Option<Message>,
) -> CommandResult {
    let lang = get_language(msg.author.id, msg.guild_id).await;
    let page_number = lang.translate(
        "page.number",
        json!({ "number": page + 1, "total": pages.len() }),
    )?;

    let react_msg = if let Some(mut paginated) = paginated {
        if let Some(page) = pages.get(page) {
            paginated
                .edit(ctx, |m| {
                    m.embed(|e| create_embed(e, msg, &title, &page, &thumbnail, &page_number));
                    m
                })
                .await?;
        }
        paginated
    } else {
        let page = pages.get(0).ok_or("Failed to get first page")?;
        msg.channel_id
            .send_message(&ctx.http, |m| {
                m.reference_message(msg);
                m.allowed_mentions(|f| f.replied_user(false));
                m.embed(|e| create_embed(e, msg, &title, &page, &thumbnail, &page_number));
                m
            })
            .await?
    };

    react_msg.delete_reactions(ctx).await?;

    if page > 0 {
        react_msg.react(ctx, '⬅').await?;
    }
    if page + 1 < pages.len() {
        react_msg.react(ctx, '➡').await?;
    }

    if let Some(reaction) = &react_msg
        .await_reaction(&ctx)
        .timeout(Duration::from_secs(120))
        .author_id(msg.author.id)
        .await
    {
        let emoji = &reaction.as_inner_ref().emoji;

        match emoji.as_data().as_str() {
            "⬅" => {
                if page > 0 {
                    _send_page(title, pages, thumbnail, page - 1, ctx, msg, Some(react_msg))
                        .await?;
                }
            }
            "➡" => {
                if page + 1 < pages.len() {
                    _send_page(title, pages, thumbnail, page + 1, ctx, msg, Some(react_msg))
                        .await?;
                }
            }
            _ => {}
        }
    }
    Ok(())
}

pub async fn send_pages(
    title: String, pages: Vec<String>, thumbnail: String, ctx: &Context, msg: &Message,
) -> CommandResult {
    _send_page(title, pages, thumbnail, 0, ctx, msg, None).await
}
