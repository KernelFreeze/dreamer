#![feature(once_cell)]
#![feature(box_syntax)]
#![feature(result_flattening)]
#![deny(clippy::unwrap_in_result)]
#![deny(clippy::unwrap_used)]
#![deny(unused_must_use)]

use std::collections::HashSet;
use std::env;
use std::env::VarError;
use std::error::Error;
use std::sync::Arc;

use mimalloc::MiMalloc;
use serenity::client::bridge::gateway::{GatewayIntents, ShardManager};
use serenity::client::Client;
use serenity::framework::StandardFramework;
use serenity::http::Http;
use serenity::model::id::UserId;
use serenity::prelude::*;
use songbird::SerenityInit;

mod audio;
mod commands;
mod constants;
mod database;
mod events;
mod hooks;
mod lang;
mod paginator;
mod utils;

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

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

    fn env_var_err(err: VarError) -> String {
        format!("Failed to process environment variable. {:?}", err)
    }

    let token = env::var("DISCORD_TOKEN").map_err(env_var_err)?;
    let prefix = env::var("BOT_PREFIX").map_err(env_var_err)?;
    let database = env::var("DATABASE_URI").map_err(env_var_err)?;

    let owners = get_owners(&token).await?;

    // Connect to database
    database::connect(&database).await?;

    let mut client = Client::builder(&token)
        .event_handler(events::Handler)
        .framework(
            StandardFramework::new()
                .configure(|c| {
                    c.prefix(&prefix)
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
                .await,
        )
        .register_songbird()
        .intents(GatewayIntents::non_privileged())
        .await?;

    init_data_manager(&client).await;
    client.start_autosharded().await?;
    Ok(())
}

/// Fetch bot owners from Discord application
async fn get_owners(token: &str) -> Result<HashSet<UserId>, serenity::Error> {
    let http = Http::new_with_token(token);
    let info = http.get_current_application_info().await?;

    let mut owners = HashSet::new();
    if let Some(team) = info.team {
        owners.insert(team.owner_user_id);
    } else {
        owners.insert(info.owner.id);
    }
    Ok(owners)
}
