use serenity::client::Context;
use serenity::framework::standard::macros::command;
use serenity::framework::standard::CommandResult;
use serenity::model::channel::Message;

#[command]
#[only_in(guilds)]
#[bucket = "basic"]
#[required_permissions("ADMINISTRATOR")]
async fn guild_settings(_ctx: &Context, _msg: &Message) -> CommandResult {
    Ok(())
}
