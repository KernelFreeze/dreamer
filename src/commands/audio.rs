use serenity::framework::standard::macros::group;

mod join;
mod leave;
mod play;

use self::join::*;
use self::leave::*;
use self::play::*;

#[group]
#[commands(join, leave, play)]
struct Audio;
