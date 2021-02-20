use serenity::client::Context;
use serenity::framework::standard::macros::command;
use serenity::framework::standard::CommandResult;
use serenity::model::channel::Message;

#[command]
#[bucket = "basic"]
async fn settings(_ctx: &Context, _msg: &Message) -> CommandResult {
    Ok(())
}
