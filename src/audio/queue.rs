use std::collections::HashMap;
use std::lazy::SyncLazy;
use std::sync::Arc;
use std::time::Duration;

use serenity::async_trait;
use serenity::http::Http;
use serenity::model::id::{ChannelId, GuildId};
use serenity::prelude::{Mutex, RwLock};
use serenity::utils::Colour;
use smallvec::SmallVec;
use songbird::input::error::Error as InputError;
use songbird::input::Metadata;
use songbird::tracks::{TrackError, TrackHandle, TrackState};
use songbird::{Call, Event, EventContext, EventHandler as VoiceEventHandler, TrackEvent};
use thiserror::Error;
use tokio::sync::{RwLockReadGuard, RwLockWriteGuard};
use tracing::debug;

use super::source::{self, MediaResource};
use crate::constants::MUSIC_ICON;

pub type QueuesType = HashMap<GuildId, MediaQueue>;

static QUEUES: SyncLazy<RwLock<QueuesType>> = SyncLazy::new(|| RwLock::new(HashMap::new()));

#[derive(Debug, Error)]
pub enum MediaQueueError {
    #[error("No elements left in the queue")]
    Empty,

    #[error("Queue has no previous element")]
    NoBack,

    #[error("Queue has no playing element")]
    NotPlaying,

    #[error("Failed to start queue play")]
    FailedToStart,

    #[error("Failed to find an url for the requested media")]
    NoUrl,

    #[error("Failed to play song in your channel")]
    ChannelPlayFailure,

    #[error("Failed to create source input")]
    Input(InputError),

    #[error("Failed track operation")]
    Track(#[from] TrackError),
}

#[derive(Debug, Clone)]
pub struct MediaQueue {
    volume: f32,
    curr: usize,
    inner: SmallVec<[MediaResource; 5]>,
    curr_handle: Option<TrackHandle>,
    handler_lock: Option<Arc<Mutex<Call>>>,
    channel: Option<ChannelId>,
    http: Option<Arc<Http>>,
}

impl Default for MediaQueue {
    fn default() -> Self {
        Self {
            volume: 1.0,
            curr: 0,
            inner: SmallVec::new(),
            curr_handle: None,
            handler_lock: None,
            channel: None,
            http: None,
        }
    }
}

impl MediaQueue {
    pub fn current(&self) -> Option<&MediaResource> {
        self.inner.get(self.curr)
    }

    pub fn current_mut(&mut self) -> Option<&mut MediaResource> {
        self.inner.get_mut(self.curr)
    }

    pub fn get(&self) -> &SmallVec<[MediaResource; 5]> {
        &self.inner
    }

    pub async fn back(&mut self) -> Result<(), MediaQueueError> {
        self.inner
            .get(self.curr - 1)
            .ok_or(MediaQueueError::NoBack)?;
        self.curr -= 1;
        self.play().await?;
        Ok(())
    }

    pub fn shuffle(&mut self) {
        use rand::seq::SliceRandom;
        use rand::thread_rng;

        self.remaining_mut().shuffle(&mut thread_rng());
    }

    pub async fn next(&mut self) -> Result<(), MediaQueueError> {
        self.inner
            .get(self.curr + 1)
            .ok_or(MediaQueueError::Empty)?;
        self.curr += 1;
        self.play().await?;
        Ok(())
    }

    pub fn pause(&mut self) -> Result<(), MediaQueueError> {
        self.curr_handle
            .as_ref()
            .ok_or(MediaQueueError::NotPlaying)?
            .pause()?;
        Ok(())
    }

    pub fn resume(&mut self) -> Result<(), MediaQueueError> {
        self.curr_handle
            .as_ref()
            .ok_or(MediaQueueError::NotPlaying)?
            .play()?;
        Ok(())
    }

    pub fn stop(&mut self) -> Result<(), MediaQueueError> {
        self.curr_handle
            .as_ref()
            .ok_or(MediaQueueError::NotPlaying)?
            .stop()?;
        self.curr_handle = None;
        Ok(())
    }

    pub fn clear(&mut self) -> Result<(), MediaQueueError> {
        self.stop()?;
        self.curr = 0;
        self.inner = SmallVec::new();
        Ok(())
    }

    pub fn volume(&mut self, volume: f32) -> Result<(), MediaQueueError> {
        self.volume = volume;
        self.curr_handle
            .as_ref()
            .map(|track| track.set_volume(volume));
        Ok(())
    }

    pub fn remaining(&self) -> &[MediaResource] {
        let next = self.curr + 1;
        &self.inner[next.min(self.inner.len())..]
    }

    pub fn remaining_mut(&mut self) -> &mut [MediaResource] {
        let len = self.inner.len();
        let next = self.curr + 1;
        &mut self.inner[next.min(len)..]
    }

    pub fn is_empty(&self) -> bool {
        self.remaining().len() == 0
    }

    pub fn is_playing(&self) -> bool {
        self.curr_handle.is_some()
    }

    pub async fn track_info(&self) -> Result<Box<TrackState>, MediaQueueError> {
        let state = self
            .curr_handle
            .as_ref()
            .ok_or(MediaQueueError::NotPlaying)?
            .get_info()
            .await?;
        Ok(state)
    }

    pub async fn metadata(&self) -> Result<&Metadata, MediaQueueError> {
        let state = self
            .curr_handle
            .as_ref()
            .ok_or(MediaQueueError::NotPlaying)?
            .metadata();
        Ok(state)
    }

    pub fn add(&mut self, audio: MediaResource) {
        self.inner.push(audio);
    }

    pub fn seek(&mut self, position: Duration) -> Result<(), MediaQueueError> {
        self.curr_handle
            .as_ref()
            .ok_or(MediaQueueError::NotPlaying)?
            .seek_time(position)?;
        Ok(())
    }

    pub async fn start(
        &mut self, handler_lock: Arc<Mutex<Call>>, channel: ChannelId, guild_id: GuildId,
        http: Arc<Http>,
    ) -> Result<(), MediaQueueError> {
        if self.is_playing() {
            debug!("Media player already initialized");
            return Ok(());
        }
        debug!("Creating new media player");
        self.handler_lock = Some(handler_lock.clone());
        self.http = Some(http.clone());
        self.channel = Some(channel);

        handler_lock
            .lock()
            .await
            .add_global_event(Event::Track(TrackEvent::End), SongEndNotifier { guild_id });

        while let Err(_) = self.play().await {
            self.curr += 1;
            if let Err(_) = self.inner.get(self.curr + 1).ok_or(MediaQueueError::Empty) {
                return Err(MediaQueueError::FailedToStart);
            }
        }
        Ok(())
    }

    async fn play(&mut self) -> Result<(), MediaQueueError> {
        let http = self
            .http
            .clone()
            .ok_or(MediaQueueError::ChannelPlayFailure)?;
        let channel = self.channel.ok_or(MediaQueueError::ChannelPlayFailure)?;

        let (url, title) = {
            let current = self.current_mut().ok_or(MediaQueueError::Empty)?;
            let url = current.url_mut().await.ok_or(MediaQueueError::NoUrl)?;
            let title = current.title().unwrap_or_else(|| String::from("Unknown"));

            (url, title)
        };

        // Create Discord player
        self.create_player(url).await?;

        let _ = channel
            .send_message(&http, |m| {
                m.embed(|e| {
                    e.thumbnail(MUSIC_ICON);
                    e.color(Colour::DARK_PURPLE);
                    e.title("Now Playing");
                    e.description(title);
                    e
                });
                m
            })
            .await;
        Ok(())
    }

    async fn create_player(&mut self, url: String) -> Result<(), MediaQueueError> {
        let handler_lock = self
            .handler_lock
            .clone()
            .ok_or(MediaQueueError::ChannelPlayFailure)?;
        let compressed = source::download(url, true)
            .await
            .map_err(MediaQueueError::Input)?;

        let (mut track, song) = songbird::tracks::create_player(compressed.into());

        let mut handler = handler_lock.lock().await;
        track.set_volume(self.volume);
        handler.play_only(track);

        self.curr_handle = Some(song);
        Ok(())
    }
}

pub async fn get_queues<'a>() -> RwLockReadGuard<'a, QueuesType> {
    QUEUES.read().await
}

pub async fn get_queues_mut<'a>() -> RwLockWriteGuard<'a, QueuesType> {
    QUEUES.write().await
}

pub fn get(queues: &mut QueuesType, id: GuildId) -> &mut MediaQueue {
    queues.entry(id).or_default()
}

pub struct SongEndNotifier {
    pub guild_id: GuildId,
}

#[async_trait]
impl VoiceEventHandler for SongEndNotifier {
    async fn act(&self, ctx: &EventContext<'_>) -> Option<Event> {
        if let EventContext::Track(_) = ctx {
            let mut queues = get_queues_mut().await;
            let queue = get(&mut queues, self.guild_id);

            if let Err(err) = queue.next().await {
                match err {
                    MediaQueueError::Empty => {}
                    err => {
                        tracing::error!("{:?}", err)
                    }
                }
                queues.remove(&self.guild_id);
                return Some(Event::Cancel);
            }
        }
        None
    }
}
