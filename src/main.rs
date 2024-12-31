mod conf;
mod logger;
mod services;
mod structs;
use std::process;

use dotenv::dotenv;
use log::{error, info, LevelFilter};
use logger::SimpleLogger;
use services::{discord::DiscordService, download_station::DownloadStation};
use structs::{DownloadingService, MessagingService};

static LOGGER: SimpleLogger = SimpleLogger;

fn main() {
    dotenv().ok();
    let _ = log::set_logger(&LOGGER).map(|()| log::set_max_level(LevelFilter::Debug));
    info!("DS-Companion starting");
    let discord = DiscordService::new();
    let download_station = DownloadStation::new();
    let tasks = discord.fetch_tasks();
    match tasks {
        Some(tasks) => {
            info!("Found {} new download tasks. Proceeding", tasks.len());
            ///logic here
            for task in tasks {
                download_station.submit_task(task);
            }
            ///
            info!("DS-Companion exiting gracefully");
            process::exit(0)
        }
        None => {
            error!("DS-Companion exiting with errors");
            process::exit(1)
        }
    }
}
