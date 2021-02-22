use std::io::{BufRead, BufReader, Cursor, Read};
use std::process::{Command, Stdio};
use std::time::Duration;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use serenity::async_trait;
use songbird::input::error::{Error, Result};
use songbird::input::{Codec, Container, Input, Metadata, Reader};
use tokio::process::Command as TokioCommand;
use tokio::task;

use super::restartable::Restart;

const YOUTUBE_DL_COMMAND: &str = "youtube-dl";

pub enum YouTubeType {
    Uri(VideoMetadata),
    Search(String),
}

pub struct YouTubeRestarter {
    uri: YouTubeType,
}

impl YouTubeRestarter {
    pub fn new(uri: YouTubeType) -> Self {
        YouTubeRestarter { uri }
    }

    async fn convert_url(&mut self) -> Result<String> {
        let result = match &self.uri {
            YouTubeType::Uri(metadata) => metadata.url.clone(),
            YouTubeType::Search(query) => {
                let search = youtube_music::search(&query)
                    .await
                    .or(Err(Error::Metadata))?;
                let result = search.get(0).ok_or(Error::Metadata)?;

                self.uri = YouTubeType::Uri(VideoMetadata {
                    url: result.video_id.clone(),
                    title: Some(result.title.clone()),
                    search_query: Some(query.clone()),
                    ..Default::default()
                });
                result.video_id.clone()
            },
        };
        Ok(result)
    }
}

#[async_trait]
impl Restart for YouTubeRestarter {
    async fn call_restart(&mut self, time: Option<Duration>) -> Result<Input> {
        let uri = self.convert_url().await?;

        if let Some(time) = time {
            let ts = format!("{}.{}", time.as_secs(), time.subsec_millis());

            _ytdl(&uri, &["-ss", &ts]).await
        } else {
            ytdl(&uri).await
        }
    }

    async fn lazy_init(&mut self) -> Result<(Codec, Container)> {
        Ok((Codec::FloatPcm, Container::Raw))
    }
}

/// Creates a streamed audio source with `youtube-dl` and `ffmpeg`.
///
/// This source is not seek-compatible.
/// If you need looping or track seeking, then consider using
/// [`Restartable::ytdl`].
///
/// Uses `youtube-dlc` if the `"youtube-dlc"` feature is enabled.
///
/// [`Restartable::ytdl`]: crate::input::restartable::Restartable::ytdl
pub async fn ytdl(uri: &str) -> Result<Input> {
    _ytdl(uri, &[]).await
}

async fn _ytdl(uri: &str, pre_args: &[&str]) -> Result<Input> {
    let ytdl_args = [
        "--print-json",
        "-f",
        "webm[abr>0]/bestaudio/best",
        "-R",
        "10",
        "--no-playlist",
        "--ignore-config",
        "--no-warnings",
        uri,
        "-o",
        "-",
    ];

    let ffmpeg_args = [
        "-f",
        "s16le",
        "-ac",
        "2",
        "-ar",
        "48000",
        "-acodec",
        "pcm_f32le",
        "-",
    ];

    let mut youtube_dl = Command::new(YOUTUBE_DL_COMMAND)
        .args(&ytdl_args)
        .stdin(Stdio::null())
        .stderr(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()?;

    // This rigmarole is required due to the inner synchronous reading context.
    let stderr = youtube_dl.stderr.take();
    let (returned_stderr, value) = task::spawn_blocking(move || {
        let mut s = stderr.unwrap();
        let out: Result<Value> = {
            let mut o_vec = vec![];
            let mut serde_read = BufReader::new(s.by_ref());
            // Newline...
            if let Ok(len) = serde_read.read_until(0xA, &mut o_vec) {
                serde_json::from_slice(&o_vec[..len]).map_err(|err| Error::Json {
                    error: err,
                    parsed_text: std::str::from_utf8(&o_vec).unwrap_or_default().to_string(),
                })
            } else {
                Result::Err(Error::Metadata)
            }
        };

        (s, out)
    })
    .await
    .map_err(|_| Error::Metadata)?;

    youtube_dl.stderr = Some(returned_stderr);

    let taken_stdout = youtube_dl.stdout.take().ok_or(Error::Stdout)?;

    let ffmpeg = Command::new("ffmpeg")
        .args(pre_args)
        .arg("-i")
        .arg("-")
        .args(&ffmpeg_args)
        .stdin(taken_stdout)
        .stderr(Stdio::null())
        .stdout(Stdio::piped())
        .spawn()?;

    let metadata = Metadata::from_ytdl_output(value?);

    Ok(Input::new(
        true,
        Reader::from(vec![youtube_dl, ffmpeg]),
        Codec::FloatPcm,
        Container::Raw,
        Some(metadata),
    ))
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VideoMetadata {
    pub id: Option<String>,
    pub url: String,
    pub title: Option<String>,
    pub description: Option<String>,
    pub duration: Option<f64>,
    pub view_count: Option<u64>,
    pub uploader: Option<String>,
    pub search_query: Option<String>,
}

pub async fn ytdl_metadata(uri: &str) -> Result<Vec<VideoMetadata>> {
    // Most of these flags are likely unused, but we want identical search
    // and/or selection as the above functions.
    let ytdl_args = [
        "--dump-json",
        "-f", // format
        "webm[abr>0]/bestaudio/best",
        "-R", // retries
        "10",
        "--youtube-skip-dash-manifest",
        "--ignore-config",
        "--no-warnings",
        "--flat-playlist",
        uri,
        "-o",
        "-",
    ];

    let youtube_dl_output = TokioCommand::new(YOUTUBE_DL_COMMAND)
        .args(&ytdl_args)
        .stdin(Stdio::null())
        .output()
        .await?;

    let out = Cursor::new(youtube_dl_output.stderr)
        .lines()
        .filter_map(std::result::Result::ok)
        .map(|line| serde_json::from_str(&line))
        .filter_map(std::result::Result::ok)
        .collect();

    Ok(out)
}
