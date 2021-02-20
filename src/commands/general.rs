use serenity::framework::standard::macros::group;

mod settings;
mod guild_settings;
mod latency;

use self::settings::*;
use self::guild_settings::*;
use self::latency::*;

#[group]
#[commands(settings, guild_settings, latency)]
struct General;
