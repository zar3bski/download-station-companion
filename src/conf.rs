use derive_new::new;
use lazy_static::lazy_static;

//TODO: should be defined both by env vars and cli args
#[derive(Debug, Default, new)]
pub struct Conf {
    pub discord_token: String,
    pub discord_channel: String,
    pub minutes_delta: usize,
}

lazy_static! {
    pub static ref CONF: Conf = Conf::new(
        std::env::var("DISCORD_TOKEN").expect("DISCORD_TOKEN must be set."),
        std::env::var("CHANNEL_ID").expect("CHANNEL_ID must be set."),
        2
    );
}
