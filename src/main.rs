mod conf;
mod core;
mod logger;
mod services;
use core::traits::{DownloadingController, MessagingController};
use log::{info, LevelFilter};
use logger::SimpleLogger;
use services::{
    discord::{DiscordController, DiscordService},
    download_station::{DsControler, DsService},
};
use std::{thread, time::Duration};

static LOGGER: SimpleLogger = SimpleLogger;
const REFRESH_TIME: Duration = Duration::from_secs(10);

fn main() {
    let _ = log::set_logger(&LOGGER).map(|()| log::set_max_level(LevelFilter::Debug));
    info!("DS-Companion starting");
    let discord = DiscordController::<DiscordService>::new();
    let mut tasks = discord.fetch_tasks().unwrap();

    info!("Found {} new download tasks. Proceeding", tasks.len());
    if tasks.len() > 0 {
        let download_station = DsControler::<DsService>::new();
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
