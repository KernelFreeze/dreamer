use std::error::Error;

use serde::{Deserialize, Serialize};
use tokio::process::Command;

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SearchResult {
    pub title: String,
    pub lyrics: String,
}

pub async fn search<S: AsRef<str>>(query: S) -> Result<SearchResult, Box<dyn Error>> {
    let query = query.as_ref();
    let output = Command::new("python3")
        .arg("-c")
        .arg(include_str!("lyrics.py"))
        .args(query.split(' '))
        .output()
        .await?;
    let output = String::from_utf8(output.stdout)?;

    Ok(serde_json::from_str(&output)?)
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
