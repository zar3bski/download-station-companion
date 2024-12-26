mod conf;
mod logger;
mod services;
mod structs;
use dotenv::dotenv;
use log::LevelFilter;
use logger::SimpleLogger;
use services::discord::{self, DiscordService};
use structs::MessagingService;

static LOGGER: SimpleLogger = SimpleLogger;

fn main() {
    dotenv().ok();
    let _ = log::set_logger(&LOGGER).map(|()| log::set_max_level(LevelFilter::Info));
    println!("Hello, world!");
    let discord = DiscordService::new();
    discord.fetch_tasks();
}
