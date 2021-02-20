#![feature(once_cell)]
#![feature(result_flattening)]
#![deny(clippy::unwrap_in_result)]
#![deny(clippy::unwrap_used)]
#![deny(clippy::pedantic)]

use std::env;
use std::error::Error;
use std::fs::File;

use dotenv::dotenv;
use log::{error, warn};
use serde_json::json;
use serenity::client::Client;
use serenity::framework::standard::macros::hook;
use serenity::framework::standard::CommandError;
use serenity::framework::StandardFramework;
use serenity::model::prelude::*;
use serenity::prelude::*;
use simplelog::*;
use songbird::SerenityInit;

use crate::errors::check_msg;

mod commands;
mod database;
mod errors;
mod events;
mod lang;

#[hook]
async fn after_hook(ctx: &Context, msg: &Message, cmd_name: &str, error: Result<(), CommandError>) {
    if let Err(why) = error {
        error!("Error in command '{}': {:?}", cmd_name, why);

        let lang = database::get_language(msg.author.id, msg.guild_id).await;
        let text = lang
            .translate("key", json!({ "error": why.to_string() }))
            .unwrap_or("Command failed".into());

        check_msg(msg.reply(ctx, text).await);
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    CombinedLogger::init(vec![
        TermLogger::new(LevelFilter::Warn, Config::default(), TerminalMode::Mixed),
        WriteLogger::new(LevelFilter::Info, Config::default(), File::create("bot.log")?),
    ])?;

    if let Err(err) = dotenv() {
        warn!("Failed to parse environment variables file: {:?}", err);
    }

    database::connect(&env::var("DATABASE_URI")?).await?;

    let framework = StandardFramework::new()
        .configure(|c| {
            c.prefix(".")
                .no_dm_prefix(true)
                .case_insensitivity(true)
                .allow_dm(true)
        })
        .bucket("basic", |b| b.delay(2).time_span(10).limit(3))
        .await
        .after(after_hook)
        .group(&commands::MUSIC_GROUP);

    let mut client = Client::builder(&env::var("DISCORD_TOKEN")?)
        .event_handler(events::Handler)
        .framework(framework)
        .register_songbird()
        .await?;

    client.start_autosharded().await?;
    Ok(())
}
