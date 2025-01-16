use std::sync::Arc;

use clap::Parser;
use once_cell::sync::Lazy;

#[derive(Debug, Parser, Default)]
#[clap(version)]
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

pub static CONF: Lazy<Arc<Conf>> = Lazy::new(|| {
    let cfg = Conf::parse();
    Arc::new(cfg)
});
