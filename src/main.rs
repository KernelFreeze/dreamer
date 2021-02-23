#![feature(once_cell)]
#![feature(box_syntax)]
#![feature(result_flattening)]
#![deny(clippy::unwrap_in_result)]
#![deny(clippy::unwrap_used)]
#![deny(unused_must_use)]
#![warn(clippy::pedantic)]

use std::collections::HashSet;
use std::env;
use std::error::Error;
use std::sync::Arc;

use dotenv::dotenv;
use serenity::client::bridge::gateway::ShardManager;
use serenity::client::Client;
use serenity::framework::StandardFramework;
use serenity::http::Http;
use serenity::prelude::*;
use songbird::SerenityInit;
use tracing::warn;

mod audio;
mod commands;
mod constants;
mod database;
mod events;
mod hooks;
mod lang;
mod spotify;

// A container type is created for inserting into the Client's `data`, which
// allows for data to be accessible across all events and framework commands, or
// anywhere else that has a copy of the `data` Arc.
pub struct ShardManagerContainer;

impl TypeMapKey for ShardManagerContainer {
    type Value = Arc<Mutex<ShardManager>>;
}

async fn init_data_manager(client: &Client) {
    let mut data = client.data.write().await;
    data.insert::<ShardManagerContainer>(Arc::clone(&client.shard_manager));
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    tracing_subscriber::fmt::init();

    // Fetch environment variables from .env file
    if let Err(err) = dotenv() {
        warn!("Failed to parse environment variables file: {:?}", err);
    }

    // Fetch discord token
    let token = env::var("DISCORD_TOKEN")?;

    // Connect to database
    database::connect(&env::var("DATABASE_URI")?).await?;

    // Fetch bot owners from Discord application
    let http = Http::new_with_token(&token);
    let (owners, _) = match http.get_current_application_info().await {
        Ok(info) => {
            let mut owners = HashSet::new();
            if let Some(team) = info.team {
                owners.insert(team.owner_user_id);
            } else {
                owners.insert(info.owner.id);
            }
            match http.get_current_user().await {
                Ok(bot_id) => (owners, bot_id.id),
                Err(why) => panic!("Could not access the bot id: {:?}", why),
            }
        },
        Err(why) => panic!("Could not access application info: {:?}", why),
    };

    let framework = StandardFramework::new()
        .configure(|c| {
            c.prefix(".")
                .no_dm_prefix(true)
                .case_insensitivity(true)
                .allow_dm(true)
                .owners(owners)
        })
        .after(hooks::after_hook)
        .help(&commands::help::HELP)
        .group(&commands::AUDIO_GROUP)
        .group(&commands::GENERAL_GROUP)
        .bucket("basic", |b| {
            b.time_span(10).limit(4).delay_action(hooks::delay_action)
        })
        .await;

    let mut client = Client::builder(&token)
        .event_handler(events::Handler)
        .framework(framework)
        .register_songbird()
        .await?;
    init_data_manager(&client).await;

    client.start().await?;
    Ok(())
}
