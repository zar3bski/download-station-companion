mod conf;
mod logger;
mod services;
mod structs;
mod task;
use std::{thread, time::Duration};

use dotenv::dotenv;
use log::{info, LevelFilter};
use logger::SimpleLogger;
use services::{discord::DiscordService, download_station::DownloadStation};
use structs::{DownloadingService, MessagingService};

static LOGGER: SimpleLogger = SimpleLogger;
const REFRESH_TIME: Duration = Duration::from_secs(10);

fn main() {
    dotenv().ok();
    let _ = log::set_logger(&LOGGER).map(|()| log::set_max_level(LevelFilter::Debug));
    info!("DS-Companion {} starting", env!("CARGO_PKG_VERSION"));
    let discord = DiscordService::new();
    let mut tasks = discord.fetch_tasks().unwrap();

    info!("Found {} new download tasks. Proceeding", tasks.len());
    if tasks.len() > 0 {
        let download_station = DownloadStation::new();
        for task in &mut tasks {
            download_station.submit_task(task);
        }

        while tasks.is_empty() == false {
            thread::sleep(REFRESH_TIME);
            download_station.get_jobs_advancement(&mut tasks);
        }
    }

    info!("DS-Companion exiting gracefully");
}
