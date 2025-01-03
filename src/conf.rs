use derive_new::new;
use lazy_static::lazy_static;

//TODO: should be defined both by env vars and cli args
#[derive(Debug, Default, new)]
pub struct Conf {
    pub discord_token: String,
    pub discord_channel: String,
    pub minutes_delta: usize,
    pub synology_root_api: String,
    pub synology_user: String,
    pub synology_password: String,
}

lazy_static! {
    pub static ref CONF: Conf = Conf::new(
        std::env::var("DISCORD_TOKEN").expect("DISCORD_TOKEN must be set."),
        std::env::var("CHANNEL_ID").expect("CHANNEL_ID must be set."),
        1,
        std::env::var("ROOT_API").expect("ROOT_API must be set."),
        std::env::var("USERNAME").expect("USERNAME must be set."),
        std::env::var("PASSWORD").expect("PASSWORD must be set."),
    );
}
