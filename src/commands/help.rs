use std::collections::HashSet;

use serenity::client::Context;
use serenity::framework::standard::macros::help;
use serenity::framework::standard::{
    help_commands, Args, CommandGroup, CommandResult, HelpOptions,
};
use serenity::model::channel::Message;
use serenity::model::id::UserId;

#[help]
#[individual_command_tip = "Hello! こんにちは！¡Hola! Bonjour! 您好! 안녕하세요~\n\nWelcome to \
                            Dreamer, a powerful and easy to use Discord bot.\nIf you want more \
                            information about a specific command, just pass the command as \
                            argument."]
#[command_not_found_text = "Could not find command: `{}`."]
#[max_levenshtein_distance(3)]
#[indention_prefix = "."]
#[lacking_permissions = "Strike"]
#[lacking_role = "Strike"]
#[wrong_channel = "Strike"]
async fn help(
    context: &Context, msg: &Message, args: Args, help_options: &'static HelpOptions,
    groups: &[&'static CommandGroup], owners: HashSet<UserId>,
) -> CommandResult {
    help_commands::with_embeds(context, msg, args, help_options, groups, owners)
        .await
        .ok_or("Failed to send help message")?;
    Ok(())
}
