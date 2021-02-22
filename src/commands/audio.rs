use serenity::framework::standard::macros::group;

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
mod song;
mod stop;
mod volume;

use self::join::JOIN_COMMAND;
use self::leave::LEAVE_COMMAND;
use self::next::NEXT_COMMAND;
use self::pause::PAUSE_COMMAND;
use self::play::PLAY_COMMAND;
use self::queue::QUEUE_COMMAND;
use self::resume::RESUME_COMMAND;
use self::seek::SEEK_COMMAND;
use self::stop::STOP_COMMAND;
use self::lyrics::LYRICS_COMMAND;

#[group]
#[commands(join, leave, play, next, queue, stop, pause, resume, seek, lyrics)]
struct Audio;
