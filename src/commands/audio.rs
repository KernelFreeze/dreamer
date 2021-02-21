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

use self::join::*;
use self::leave::*;
use self::next::*;
use self::play::*;
use self::queue::*;
use self::stop::*;

#[group]
#[commands(join, leave, play, next, queue, stop)]
struct Audio;
