use std::error::Error;
use std::io::{BufRead, BufReader, Read};
use std::process::{Command, Stdio};
use std::time::Duration;

use serde::{Deserialize, Serialize};
use serenity::async_trait;
use songbird::input::error::{Error as SongbirdError, Result as SongbirdResult};
use songbird::input::restartable::Restart;
use songbird::input::{Codec, Container, Input, Metadata, Reader, Restartable};
use tokio::process::Command as TokioCommand;
use tokio::task;
use tracing::debug;

const YOUTUBE_DL_COMMAND: &str = "youtube-dl";

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MediaResource {
    pub id: Option<String>,
    pub url: Option<String>,
    pub title: Option<String>,
    pub description: Option<String>,
    pub duration: Option<f64>,
    pub view_count: Option<u64>,
    pub uploader: Option<String>,
}

impl MediaResource {
    pub async fn with_query<S: AsRef<str>>(query: S) -> Result<Self, Box<dyn Error>> {
        let results = youtube_music::search(query).await?;
        let result = results.get(0).ok_or("No results found for your query.")?;
        let meta = ytdl_metadata(&result.video_id)
            .await
            .map_err(|err| format!("`youtube-dl` error {:?}", err))?;
        let meta = meta.get(0).ok_or("No results found for your query.")?.clone();

        Ok(meta)
    }

    pub fn title(&self) -> Option<String> {
        if let Some(title) = &self.title {
            return Some(title.clone());
        }
        self.url.clone()
    }

    pub async fn url(&self) -> Option<String> {
        self.url.clone()
    }
}

pub async fn ytdl_metadata<S>(uri: S) -> SongbirdResult<Vec<MediaResource>>
where
    S: AsRef<str>, {
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
        uri.as_ref(),
        "-o",
        "-",
    ];

    let youtube_dl_output = TokioCommand::new(YOUTUBE_DL_COMMAND)
        .args(&ytdl_args)
        .stdin(Stdio::null())
        .output()
        .await?;

    let out = youtube_dl_output
        .stderr
        .lines()
        .filter_map(std::result::Result::ok)
        .map(|mut line| simd_json::serde::from_str(&mut line))
        .filter_map(std::result::Result::ok)
        .collect();

    Ok(out)
}

pub async fn download<P: AsRef<str> + Send + Clone + Sync + 'static>(
    uri: P, lazy: bool,
) -> SongbirdResult<Restartable> {
    Restartable::new(YtdlRestarter { uri }, lazy).await
}

async fn ytdl(uri: &str, pre_args: &[&str]) -> SongbirdResult<Input> {
    let ytdl_args = [
        "--print-json",
        "-f",
        "webm[abr>0]/bestaudio/best",
        "-R",
        "infinite",
        "--no-playlist",
        "--ignore-config",
        "--no-warnings",
        "--skip-unavailable-fragments",
        "-o",
        "-",
        "--",
        uri,
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
    let mut stderr = youtube_dl.stderr.take().ok_or(SongbirdError::Stdout)?;
    let metadata = task::spawn_blocking(move || {
        let mut buffer = String::new();
        let mut serde_read = BufReader::new(stderr.by_ref());

        serde_read
            .read_line(&mut buffer)
            .map_err(|_| SongbirdError::Metadata)?;

        serde_json::from_str(&buffer).map_err(|_| SongbirdError::Metadata)
    })
    .await
    .map_err(|_| SongbirdError::Metadata)?;

    let ffmpeg = Command::new("ffmpeg")
        .args(pre_args)
        .arg("-i")
        .arg("-")
        .args(&ffmpeg_args)
        .stdin(youtube_dl.stdout.take().ok_or(SongbirdError::Stdout)?)
        .stderr(Stdio::null())
        .stdout(Stdio::piped())
        .spawn()?;

    let metadata = Metadata::from_ytdl_output(metadata?);

    debug!("Playing song {:?}", metadata);
    Ok(Input::new(
        true,
        Reader::from(vec![youtube_dl, ffmpeg]),
        Codec::FloatPcm,
        Container::Raw,
        Some(metadata),
    ))
}

async fn _ytdl_metadata(uri: &str) -> SongbirdResult<Metadata> {
    // Most of these flags are likely unused, but we want identical search
    // and/or selection as the above functions.
    let ytdl_args = [
        "-j",
        "-f",
        "webm[abr>0]/bestaudio/best",
        "-R",
        "infinite",
        "--no-playlist",
        "--ignore-config",
        "--no-warnings",
        "-o",
        "-",
        "--",
        uri,
    ];

    let youtube_dl_output = TokioCommand::new(YOUTUBE_DL_COMMAND)
        .args(&ytdl_args)
        .stdin(Stdio::null())
        .output()
        .await?;

    let o_vec = youtube_dl_output.stderr;
    let end = (&o_vec)
        .iter()
        .position(|el| *el == b'\n')
        .unwrap_or_else(|| o_vec.len());

    let value = serde_json::from_slice(&o_vec[..end]).map_err(|err| SongbirdError::Json {
        error: err,
        parsed_text: std::str::from_utf8(&o_vec).unwrap_or_default().to_string(),
    })?;

    let metadata = Metadata::from_ytdl_output(value);

    Ok(metadata)
}

struct YtdlRestarter<P>
where
    P: AsRef<str> + Send + Sync, {
    uri: P,
}

#[async_trait]
impl<P> Restart for YtdlRestarter<P>
where
    P: AsRef<str> + Send + Sync,
{
    async fn call_restart(&mut self, time: Option<Duration>) -> SongbirdResult<Input> {
        if let Some(time) = time {
            let ts = format!("{}.{}", time.as_secs(), time.subsec_millis());

            ytdl(self.uri.as_ref(), &["-ss", &ts]).await
        } else {
            ytdl(self.uri.as_ref(), &[]).await
        }
    }

    async fn lazy_init(&mut self) -> SongbirdResult<(Option<Metadata>, Codec, Container)> {
        _ytdl_metadata(self.uri.as_ref())
            .await
            .map(|m| (Some(m), Codec::FloatPcm, Container::Raw))
    }
}
