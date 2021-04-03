use std::collections::HashMap;
use std::error::Error;
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
use songbird::tracks::{PlayMode, TrackError, TrackHandle, TrackState};
use songbird::{Call, Event, EventContext, EventHandler as VoiceEventHandler, TrackEvent};
use thiserror::Error;
use tokio::sync::{RwLockReadGuard, RwLockWriteGuard};
use tracing::{debug, instrument, warn};

use super::source::{self, MediaResource};
use crate::constants::MUSIC_ICON;

pub type QueuesType = HashMap<GuildId, RwLock<MediaQueue>>;

static QUEUES: SyncLazy<RwLock<QueuesType>> = SyncLazy::new(|| RwLock::new(HashMap::new()));

#[derive(Debug, Error)]
pub enum MediaQueueError {
    #[error("No elements left in the queue")]
    Empty,

    #[error("Queue has no previous element")]
    NoBack,

    #[error("Queue has no playing element")]
    NotPlaying,

    #[error("Reached end of the queue")]
    QueueEnd,

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
    repeat: bool,
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
            repeat: false,
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
    pub fn toggle_repeat(&mut self) {
        self.repeat = !self.repeat;
    }

    pub fn set_repeat(&mut self, repeat: bool) {
        self.repeat = repeat;
    }

    pub fn repeat(&self) -> bool {
        self.repeat
    }

    pub fn current(&self) -> Option<&MediaResource> {
        self.inner.get(self.curr)
    }

    pub fn get_tracks(&self) -> &SmallVec<[MediaResource; 5]> {
        &self.inner
    }

    pub async fn back(&mut self) -> Result<(), MediaQueueError> {
        debug!("Skipping to previous song");
        if self.curr == 0 {
            return Err(MediaQueueError::NoBack);
        }
        self.curr -= 1;
        self.play().await?;
        Ok(())
    }

    pub fn shuffle(&mut self) {
        use rand::seq::SliceRandom;
        use rand::thread_rng;

        debug!("Mixing queue");
        self.remaining_mut().shuffle(&mut thread_rng());
    }

    pub async fn next(&mut self) -> Result<(), MediaQueueError> {
        debug!("Skipping to next song");
        self.curr += 1;

        if self.is_empty() {
            if self.repeat() {
                self.curr = 0;
            }

            if self.is_empty() {
                return Err(MediaQueueError::Empty);
            }
        }
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

    #[instrument]
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
        Ok(())
    }

    pub async fn update_song(&mut self, song: TrackHandle) {
        if let Err(err) = self.send_song_message(&song).await {
            warn!("Failed to send song message {:?}", err);
        }

        self.curr_handle = Some(song);
    }

    #[instrument]
    pub async fn play(&self) -> Result<TrackHandle, MediaQueueError> {
        debug!("Trying to play current song");

        let url = self
            .current()
            .ok_or(MediaQueueError::Empty)?
            .url()
            .await
            .ok_or(MediaQueueError::NoUrl)?;

        // Create Discord player
        self.create_player(url).await
    }

    #[instrument]
    async fn create_player(&self, url: String) -> Result<TrackHandle, MediaQueueError> {
        debug!("Creating player");
        let handler_lock = self
            .handler_lock
            .clone()
            .ok_or(MediaQueueError::ChannelPlayFailure)?;
        let compressed = source::download(url.clone(), true)
            .await
            .map_err(MediaQueueError::Input)?;

        let (mut track, song) = songbird::tracks::create_player(compressed.into());

        let mut handler = handler_lock.lock().await;
        track.set_volume(self.volume);
        handler.play_only(track);

        Ok(song)
    }

    async fn send_song_message(&self, song: &TrackHandle) -> Result<(), MediaQueueError> {
        let http = self
            .http
            .clone()
            .ok_or(MediaQueueError::ChannelPlayFailure)?;
        let channel = self.channel.ok_or(MediaQueueError::ChannelPlayFailure)?;
        let msg_err = channel
            .send_message(&http, |m| {
                m.embed(|e| {
                    e.thumbnail(MUSIC_ICON);
                    e.color(Colour::DARK_PURPLE);
                    e.title("Now Playing");

                    if let Some(url) = song.metadata().source_url.as_deref() {
                        e.url(url);
                    }
                    if let Some(title) = song.metadata().title.as_deref() {
                        e.field("Title", title, false);
                    }
                    if let Some(artist) = song.metadata().artist.as_deref() {
                        e.field("Artist", artist, true);
                    }
                    if let Some(duration) = song.metadata().duration {
                        e.field("Duration", humantime::format_duration(duration), true);
                    }
                    e
                });
                m
            })
            .await;
        if let Err(err) = msg_err {
            warn!("Failed to send next song message: {:?}", err);
        }
        Ok(())
    }
}

pub async fn get_queues<'a>() -> RwLockReadGuard<'a, QueuesType> {
    QUEUES.read().await
}

pub async fn get_queues_mut<'a>() -> RwLockWriteGuard<'a, QueuesType> {
    QUEUES.write().await
}

pub async fn get_write(queues: &mut QueuesType, id: GuildId) -> RwLockWriteGuard<'_, MediaQueue> {
    queues.entry(id).or_default().write().await
}

pub fn get_or_create(queues: &mut QueuesType, id: GuildId) -> &RwLock<MediaQueue> {
    queues.entry(id).or_default()
}

pub fn get(queues: &QueuesType, id: GuildId) -> Option<&RwLock<MediaQueue>> {
    queues.get(&id)
}

pub async fn try_play_all(
    guild_id: GuildId, next: bool,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let queues = get_queues().await;
    let queue = get(&queues, guild_id).ok_or("There are no active queue for this server")?;

    if next {
        queue.write().await.next().await?;
    }

    let mut song = queue.read().await.play().await;
    while song.is_err() {
        if queue.write().await.next().await.is_err() {
            return Err(MediaQueueError::QueueEnd.into());
        }

        song = queue.read().await.play().await;
    }
    if let Ok(song) = song {
        queue.write().await.update_song(song).await;
    }

    Ok(())
}

pub struct SongEndNotifier {
    pub guild_id: GuildId,
}

#[async_trait]
impl VoiceEventHandler for SongEndNotifier {
    async fn act(&self, ctx: &EventContext<'_>) -> Option<Event> {
        if let EventContext::Track(tracks) = ctx {
            for (state, handle) in *tracks {
                if state.playing != PlayMode::End && state.playing != PlayMode::Stop {
                    continue;
                }
                debug!(
                    "Track {:?} finished with states {:?}",
                    handle.metadata().title,
                    state
                );

                if let Err(_) = try_play_all(self.guild_id, true).await {
                    get_queues_mut().await.remove(&self.guild_id);
                    return Some(Event::Cancel);
                }
            }
        }
        None
    }
}
