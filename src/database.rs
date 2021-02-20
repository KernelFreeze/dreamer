use std::error::Error;
use std::lazy::SyncOnceCell;

use serenity::model::id::{GuildId, UserId};
use wither::mongodb::{Client, Database};
use wither::{bson, Model};

use self::guild::Guild;
use self::user::User;
use crate::lang::Language;

pub mod guild;
pub mod rpg;
pub mod user;

static DATABASE: SyncOnceCell<Database> = SyncOnceCell::new();

pub fn database() -> &'static Database {
    DATABASE.get().expect("Database connection was not initialized")
}

pub async fn connect(uri: &str) -> Result<(), Box<dyn Error>> {
    let db = Client::with_uri_str(uri).await?.database("dreamer");
    sync_collections(&db).await?;
    DATABASE.set(db).map_err(|_| "Database already initialized")?;
    Ok(())
}

async fn sync_collections(db: &Database) -> Result<(), Box<dyn Error>> {
    User::sync(db).await?;
    Guild::sync(db).await?;
    Ok(())
}

pub async fn get_user(user_id: UserId) -> Option<User> {
    let filter = Some(bson::doc! {
      "discord_id": user_id.0
    });
    User::find_one(&database(), filter, None).await.ok().flatten()
}

pub async fn get_guild(guild_id: GuildId) -> Option<Guild> {
    let filter = Some(bson::doc! {
      "discord_id": guild_id.0
    });
    Guild::find_one(&database(), filter, None).await.ok().flatten()
}

pub async fn get_language(user_id: UserId, guild_id: Option<GuildId>) -> Language {
    if let Some(user) = get_user(user_id).await {
        return user.lang;
    }
    if let Some(guild_id) = guild_id {
        if let Some(guild) = get_guild(guild_id).await {
            return guild.lang;
        }
    }
    Language::default()
}
