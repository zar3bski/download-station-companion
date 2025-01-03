use clap::Parser;
use lazy_static::lazy_static;

//TODO: should be defined both by env vars and cli args
#[derive(Debug, Parser)]
pub struct Conf {
    #[arg(long, env)]
    pub discord_token: String,
    #[arg(long, env)]
    pub discord_channel: String,
    #[arg(short, long, default_value_t = 2, env)]
    pub minutes_delta: usize,
    #[arg(long, env)]
    pub synology_root_api: String,
    #[arg(long, env)]
    pub synology_user: String,
    #[arg(long, env)]
    pub synology_password: String,
}

lazy_static! {
    pub static ref CONF: Conf = {
        let cfg = Conf::parse();
        cfg
    };
}
