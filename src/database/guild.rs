use serde::{Deserialize, Serialize};
use wither::bson::doc;
use wither::bson::oid::ObjectId;
use wither::prelude::*;

use crate::lang::Language;

#[derive(Debug, Model, Serialize, Deserialize)]
pub struct Guild {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,

    pub discord_id: u64,

    pub lang: Language,

    pub join_date: i64,
}
