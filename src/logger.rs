use chrono::Local;
use log::{Level, Metadata, Record};

pub struct SimpleLogger;

impl log::Log for SimpleLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= Level::Debug // Here to change log level, find something better
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            println!(
                "{} [{}] - {}",
                record.level(),
                Local::now().format("%Y-%m-%dT%H:%M:%S"),
                record.args()
            );
        }
    }

    fn flush(&self) {}
}
