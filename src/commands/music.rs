use serenity::framework::standard::macros::group;

mod join;
mod leave;
mod play;

use join::*;
use leave::*;
use play::*;

#[group]
#[commands(join, leave, play)]
struct Music;
