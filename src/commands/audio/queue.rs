use serenity::client::Context;
use serenity::framework::standard::macros::command;
use serenity::framework::standard::{Args, CommandResult};
use serenity::model::channel::Message;

#[command]
#[only_in(guilds)]
async fn queue(_ctx: &Context, _msg: &Message, _args: Args) -> CommandResult {
    Ok(())
}
