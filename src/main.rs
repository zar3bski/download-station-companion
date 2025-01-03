mod conf;
mod logger;
mod services;
mod structs;
mod task;
use std::{process, thread, time::Duration};

use dotenv::dotenv;
use log::{info, LevelFilter};
use logger::SimpleLogger;
use services::{discord::DiscordService, download_station::DownloadStation};
use structs::{DownloadingService, MessagingService};
use task::TaskStatus;

static LOGGER: SimpleLogger = SimpleLogger;
const REFRESH_TIME: Duration = Duration::from_secs(10);

fn main() {
    dotenv().ok();
    let _ = log::set_logger(&LOGGER).map(|()| log::set_max_level(LevelFilter::Debug));
    info!("DS-Companion starting");
    let discord = DiscordService::new();
    let download_station = DownloadStation::new();
    let mut tasks = discord.fetch_tasks().unwrap();

    info!("Found {} new download tasks. Proceeding", tasks.len());

    for task in &mut tasks {
        //move occurs because `tasks` has type `Vec<Task<'_>>`, which does not implement the `Copy` trait
        download_station.submit_task(task);
    }

    while tasks.is_empty() == false {
        thread::sleep(REFRESH_TIME);
        download_station.get_jobs_advancement(&mut tasks);
    }

    info!("DS-Companion exiting gracefully");
    process::exit(0)
}
