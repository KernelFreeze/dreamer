//! A source which supports seeking by recreating its input stream.
//!
//! This is intended for use with single-use audio tracks which
//! may require looping or seeking, but where additional memory
//! cannot be spared. Forward seeks will drain the track until reaching
//! the desired timestamp.
//!
//! Restarting occurs by temporarily pausing the track, running the restart
//! mechanism, and then passing the handle back to the mixer thread. Until
//! success/failure is confirmed, the track produces silence.
use std::io::{
    Error as IoError, ErrorKind as IoErrorKind, Read, Result as IoResult, Seek, SeekFrom,
};
use std::time::Duration;

use flume::{Receiver, TryRecvError};
use serenity::async_trait;
use songbird::input::error::Result;
use songbird::input::{utils, Codec, Container, Input, Reader};
use tokio::runtime::Handle;

use super::audio_ext::ReadAudioExt;

type Recreator = Box<dyn Restart + Send + 'static>;
type RecreateChannel = Receiver<Result<(Box<Input>, Recreator)>>;

// Use options here to make "take" more doable from a mut ref.
enum LazyProgress {
    Dead(Option<Recreator>, Codec, Container),
    Live(Box<Input>, Option<Recreator>),
    Working(Codec, Container, bool, RecreateChannel),
}

/// A wrapper around a method to create a new [`Input`] which
/// seeks backward by recreating the source.
///
/// The main purpose of this wrapper is to enable seeking on
/// incompatible sources (i.e., ffmpeg output) and to ease resource
/// consumption for commonly reused/shared tracks. [`Compressed`]
/// and [`Memory`] offer the same functionality with different
/// tradeoffs.
///
/// This is intended for use with single-use audio tracks which
/// may require looping or seeking, but where additional memory
/// cannot be spared. Forward seeks will drain the track until reaching
/// the desired timestamp.
///
/// [`Input`]: Input
/// [`Memory`]: cached::Memory
/// [`Compressed`]: cached::Compressed
pub struct Restartable {
    async_handle: Option<Handle>,
    position: usize,
    source: LazyProgress,
}

impl Restartable {
    /// Create a new source, which can be restarted using a `recreator`
    /// function.
    ///
    /// Lazy sources will not run their input recreator until the first byte
    /// is needed, or are sent
    /// [`Track::make_playable`]/[`TrackHandle::make_playable`].
    ///
    /// [`Track::make_playable`]: crate::tracks::Track::make_playable
    /// [`TrackHandle::make_playable`]:
    /// crate::tracks::TrackHandle::make_playable
    pub async fn new(mut recreator: impl Restart + Send + 'static) -> Result<Self> {
        recreator.lazy_init().await.map(move |(kind, codec)| Self {
            async_handle: Handle::try_current().ok(),
            position: 0,
            source: LazyProgress::Dead(Some(Box::new(recreator)), kind, codec),
        })
    }
}

/// Trait used to create an instance of a [`Reader`] at instantiation and when
/// a backwards seek is needed.
///
/// [`Reader`]: reader::Reader
#[async_trait]
pub trait Restart {
    /// Tries to create a replacement source.
    async fn call_restart(&mut self, time: Option<Duration>) -> Result<Input>;

    /// Optionally retrieve metadata for a source which has been lazily
    /// initialised.
    ///
    /// This is particularly useful for sources intended to be queued, which
    /// should occupy few resources when not live BUT have as much information
    /// as possible made available at creation.
    async fn lazy_init(&mut self) -> Result<(Codec, Container)>;
}

impl From<Restartable> for Input {
    fn from(mut src: Restartable) -> Self {
        let (meta, stereo, kind, container) = match &mut src.source {
            LazyProgress::Dead(_, kind, container) => (None, true, kind.clone(), *container),
            LazyProgress::Live(ref mut input, _rec) => (
                Some(input.metadata.take()),
                input.stereo,
                input.kind.clone(),
                input.container,
            ),
            // This branch should never be taken: this is an emergency measure.
            LazyProgress::Working(kind, container, stereo, _) => {
                (None, *stereo, kind.clone(), *container)
            },
        };
        Input::new(
            stereo,
            Reader::ExtensionSeek(Box::new(src)),
            kind,
            container,
            meta,
        )
    }
}

// How do these work at a high level?
// If you need to restart, send a request to do this to the async context.
// if a request is pending, then just output all zeroes.

impl Read for Restartable {
    fn read(&mut self, buffer: &mut [u8]) -> IoResult<usize> {
        let (out_val, march_pos, next_source) = match &mut self.source {
            LazyProgress::Dead(rec, kind, container) => {
                let handle = self.async_handle.clone();
                let new_chan = if let Some(rec) = rec.take() {
                    Some(regenerate_channel(
                        rec,
                        0,
                        true,
                        kind.clone(),
                        *container,
                        handle,
                    )?)
                } else {
                    return Err(IoError::new(
                        IoErrorKind::UnexpectedEof,
                        "Illegal state: taken recreator was observed.".to_string(),
                    ));
                };

                // Then, output all zeroes.
                for el in buffer.iter_mut() {
                    *el = 0;
                }
                (Ok(buffer.len()), false, new_chan)
            },
            LazyProgress::Live(source, _) => (Read::read(source, buffer), true, None),
            LazyProgress::Working(_, _, _, chan) => {
                match chan.try_recv() {
                    Ok(Ok((mut new_source, recreator))) => {
                        // Completed!
                        // Do read, then replace inner progress.
                        let bytes_read = Read::read(&mut new_source, buffer);

                        (
                            bytes_read,
                            true,
                            Some(LazyProgress::Live(new_source, Some(recreator))),
                        )
                    },
                    Ok(Err(source_error)) => {
                        let e = Err(IoError::new(
                            IoErrorKind::UnexpectedEof,
                            format!("Failed to create new reader: {:?}.", source_error),
                        ));
                        (e, false, None)
                    },
                    Err(TryRecvError::Empty) => {
                        // Output all zeroes.
                        for el in buffer.iter_mut() {
                            *el = 0;
                        }
                        (Ok(buffer.len()), false, None)
                    },
                    Err(_) => {
                        let e = Err(IoError::new(
                            IoErrorKind::UnexpectedEof,
                            "Failed to create new reader: dropped.",
                        ));
                        (e, false, None)
                    },
                }
            },
        };

        if let Some(src) = next_source {
            self.source = src;
        }

        if march_pos {
            out_val.map(|a| {
                self.position += a;
                a
            })
        } else {
            out_val
        }
    }
}

impl Seek for Restartable {
    fn seek(&mut self, pos: SeekFrom) -> IoResult<u64> {
        let _local_pos = self.position as u64;

        use SeekFrom::*;
        match pos {
            Start(offset) => {
                let offset = offset as usize;
                let handle = self.async_handle.clone();

                match &mut self.source {
                    LazyProgress::Dead(rec, kind, container) => {
                        // regen at given start point
                        self.source = if let Some(rec) = rec.take() {
                            regenerate_channel(rec, offset, true, kind.clone(), *container, handle)?
                        } else {
                            return Err(IoError::new(
                                IoErrorKind::UnexpectedEof,
                                "Illegal state: taken recreator was observed.".to_string(),
                            ));
                        };

                        self.position = offset;
                    },
                    LazyProgress::Live(input, rec) => {
                        if offset < self.position {
                            // regen at given start point
                            // We're going back in time.
                            self.source = if let Some(rec) = rec.take() {
                                regenerate_channel(
                                    rec,
                                    offset,
                                    input.stereo,
                                    input.kind.clone(),
                                    input.container,
                                    handle,
                                )?
                            } else {
                                return Err(IoError::new(
                                    IoErrorKind::UnexpectedEof,
                                    "Illegal state: taken recreator was observed.".to_string(),
                                ));
                            };

                            self.position = offset;
                        } else {
                            // march on with live source.
                            self.position += input.consume(offset - self.position);
                        }
                    },
                    LazyProgress::Working(..) => {
                        return Err(IoError::new(
                            IoErrorKind::Interrupted,
                            "Previous seek in progress.",
                        ));
                    },
                }

                Ok(offset as u64)
            },
            End(_offset) => Err(IoError::new(
                IoErrorKind::InvalidInput,
                "End point for Restartables is not known.",
            )),
            Current(_offset) => unimplemented!(),
        }
    }
}

fn regenerate_channel(
    mut rec: Recreator, offset: usize, stereo: bool, kind: Codec, container: Container,
    handle: Option<Handle>,
) -> IoResult<LazyProgress> {
    if let Some(handle) = handle.as_ref() {
        let (tx, rx) = flume::bounded(1);

        handle.spawn(async move {
            let ret_val = rec
                .call_restart(Some(utils::byte_count_to_timestamp(offset, stereo)))
                .await;

            let _ = tx.send(ret_val.map(Box::new).map(|v| (v, rec)));
        });

        Ok(LazyProgress::Working(kind, container, stereo, rx))
    } else {
        Err(IoError::new(
            IoErrorKind::Interrupted,
            "Cannot safely call seek until provided an async context handle.",
        ))
    }
}
