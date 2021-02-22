use serenity::framework::standard::macros::group;

mod guild_settings;
mod latency;
mod settings;

use self::guild_settings::GUILD_SETTINGS_COMMAND;
use self::latency::LATENCY_COMMAND;
use self::settings::SETTINGS_COMMAND;

#[group]
#[commands(settings, guild_settings, latency)]
struct General;
