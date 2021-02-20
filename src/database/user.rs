use serde::{Deserialize, Serialize};
use wither::bson::doc;
use wither::bson::oid::ObjectId;
use wither::prelude::*;

use crate::lang::Language;

#[derive(Debug, Model, Serialize, Deserialize)]
#[model(index(keys = r#"doc!{"discord_id": 1}"#, options = r#"doc!{"unique": true}"#))]
pub struct User {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,

    pub discord_id: u64,

    pub username: String,

    pub discriminator: u16,

    pub lang: Language,

    pub first_use: i64,

    pub last_seen: i64,
}
