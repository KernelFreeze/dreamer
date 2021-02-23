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
use songbird::{input::{Metadata, cached::Compressed}, tracks::TrackState};
use songbird::input::error::Error as InputError;
use songbird::input::Restartable;
use songbird::tracks::{TrackError, TrackHandle};
use songbird::{Bitrate, Call, Event, EventContext, EventHandler as VoiceEventHandler, TrackEvent};
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

    #[error("Failed to find an url for the requested media")]
    NoUrl,

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

    pub async fn back(&mut self) -> Result<(), MediaQueueError> {
        self.inner
            .get(self.curr - 1)
            .ok_or(MediaQueueError::NoBack)?;
        self.curr -= 1;
        self.continue_queue().await?;
        Ok(())
    }

    pub async fn next(&mut self) -> Result<(), MediaQueueError> {
        self.inner
            .get(self.curr + 1)
            .ok_or(MediaQueueError::Empty)?;
        self.curr += 1;
        self.continue_queue().await?;
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
        &self.inner[self.curr.min(self.inner.len())..]
    }

    pub fn is_empty(&self) -> bool {
        self.remaining().len() == 0
    }

    pub fn is_playing(&self) -> bool {
        self.curr_handle.is_some()
    }

    pub async fn track_info(&self) -> Result<Box<TrackState>, MediaQueueError> {
        let state = self.curr_handle.as_ref().ok_or(MediaQueueError::NotPlaying)?.get_info().await?;
        Ok(state)
    }

    pub async fn metadata(&self) -> Result<&Metadata, MediaQueueError> {
        let state = self.curr_handle.as_ref().ok_or(MediaQueueError::NotPlaying)?.metadata();
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
        debug!("Trying to start media player");

        if self.is_playing() {
            debug!("Media player already initialized");
            return Ok(());
        }
        self.handler_lock = Some(handler_lock.clone());
        self.http = Some(http.clone());
        self.channel = Some(channel);
        {
            let mut handler = handler_lock.lock().await;
            handler.add_global_event(Event::Track(TrackEvent::End), SongEndNotifier { guild_id });
        }

        self.play().await
    }

    async fn continue_queue(&mut self) -> Result<(), MediaQueueError> {
        self.play().await?;
        Ok(())
    }

    async fn play(&mut self) -> Result<(), MediaQueueError> {
        let handler_lock = self.handler_lock.clone().ok_or(MediaQueueError::NoUrl)?;
        let http = self.http.clone().ok_or(MediaQueueError::NoUrl)?;
        let channel = self.channel.ok_or(MediaQueueError::NoUrl)?;

        let mut handler = handler_lock.lock().await;

        let current = self.current_mut().ok_or(MediaQueueError::Empty)?;
        let url = current.url().await.ok_or(MediaQueueError::NoUrl)?;

        let compressed = source::download(url, true)
            .await
            .map_err(MediaQueueError::Input)?;
        let (track, song) = songbird::tracks::create_player(compressed.into());
        handler.play_only(track);

        let _ = channel
            .send_message(&http, |m| {
                m.embed(|e| {
                    e.thumbnail(MUSIC_ICON);
                    e.color(Colour::DARK_PURPLE);
                    e.title("Now Playing");
                    e.description(current.title().unwrap_or(String::from("Unknown")));
                    e
                });
                m
            })
            .await;

        song.set_volume(self.volume)?;
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

pub fn get<'a>(queues: &'a mut QueuesType, id: GuildId) -> &'a mut MediaQueue {
    queues.entry(id).or_default()
}

pub struct SongEndNotifier {
    pub guild_id: GuildId,
}

#[async_trait]
impl VoiceEventHandler for SongEndNotifier {
    async fn act(&self, _ctx: &EventContext<'_>) -> Option<Event> {
        let mut queues = get_queues_mut().await;
        let queue = get(&mut queues, self.guild_id);

        if let Err(err) = queue.next().await {
            match err {
                MediaQueueError::Empty => {},
                err => {
                    tracing::error!("{:?}", err)
                },
            }
            queues.remove(&self.guild_id);
            return Some(Event::Cancel);
        }
        None
    }
}
