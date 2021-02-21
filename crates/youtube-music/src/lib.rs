use std::error::Error;

use async_process::Command;
use serde::{Deserialize, Serialize};

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchResult {
    pub title: String,
    pub video_id: String,
    pub result_type: Option<String>,
    pub duration: Option<String>,
    pub thumbnails: Option<Vec<Thumbnail>>,
    pub album: Option<Album>,
    pub year: Option<String>,
    pub artists: Option<Vec<Artist>>,
    pub is_explicit: Option<bool>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Album {
    pub name: String,
    pub id: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Artist {
    pub name: String,
    pub id: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Thumbnail {
    pub url: String,
    pub width: Option<i64>,
    pub height: Option<i64>,
}

pub async fn search(query: &str) -> Result<Vec<SearchResult>, Box<dyn Error>> {
    let output = Command::new("python3")
        .arg("-c")
        .arg(include_str!("search.py"))
        .args(query.split(" "))
        .output()
        .await?;
    let output = String::from_utf8(output.stdout)?;

    let list = serde_json::from_str(&output)?;
    Ok(list)
}

#[cfg(test)]
mod tests {
    #[tokio::test]
    async fn search() {
        super::search("Wake me Up Avicii")
            .await
            .expect("Failed to search");
    }
}
