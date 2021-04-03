use serenity::framework::standard::macros::group;

mod back;
mod effects;
mod fast_forward;
mod join;
mod leave;
mod lyrics;
mod next;
mod pause;
mod play;
mod queue;
mod repeat;
mod resume;
mod rewind;
mod seek;
mod shuffle;
mod song;
mod stop;
mod volume;

use self::back::BACK_COMMAND;
use self::fast_forward::FAST_FORWARD_COMMAND;
use self::join::JOIN_COMMAND;
use self::leave::LEAVE_COMMAND;
use self::lyrics::LYRICS_COMMAND;
use self::next::NEXT_COMMAND;
use self::pause::PAUSE_COMMAND;
use self::play::PLAY_COMMAND;
use self::queue::QUEUE_COMMAND;
use self::repeat::REPEAT_COMMAND;
use self::resume::RESUME_COMMAND;
use self::rewind::REWIND_COMMAND;
use self::seek::SEEK_COMMAND;
use self::shuffle::SHUFFLE_COMMAND;
use self::song::SONG_COMMAND;
use self::stop::STOP_COMMAND;
use self::volume::VOLUME_COMMAND;

#[group]
#[commands(
    join,
    leave,
    play,
    next,
    queue,
    stop,
    pause,
    resume,
    seek,
    lyrics,
    volume,
    song,
    shuffle,
    back,
    fast_forward,
    rewind,
    repeat
)]
struct Audio;
