use serenity::framework::standard::macros::group;

mod guild_settings;
mod latency;
mod settings;

use self::guild_settings::*;
use self::latency::*;
use self::settings::*;

#[group]
#[commands(settings, guild_settings, latency)]
struct General;
