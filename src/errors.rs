use log::warn;
use serenity::model::channel::Message;
use serenity::Result as SerenityResult;

/// Checks that a message successfully sent; if not, then logs it
pub fn check_msg(result: SerenityResult<Message>) {
    if let Err(why) = result {
        warn!("Error sending a message: {:?}", why);
    }
}
