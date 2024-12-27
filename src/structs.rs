use core::fmt;
use std::{collections::HashMap, iter::Enumerate};

#[derive(Debug)]
pub enum TaskStatus {
    RECEIVED,
    SUBMITTED,
    DOWNLOADING,
    FAILED,
    DONE,
}

impl fmt::Display for TaskStatus {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[derive(Debug)]
pub struct Task {
    pub status: TaskStatus,
    pub message_id: String,
    pub magnet_link: String,
}

impl Task {
    pub fn new(magnet_link: String, message_id: String) -> Self {
        Self {
            magnet_link: magnet_link,
            message_id: message_id,
            status: TaskStatus::RECEIVED,
        }
    }
}

pub trait MessagingService {
    fn new() -> Self;
    fn fetch_tasks(&self) -> Option<Vec<Task>>;
    fn update_task_status(&self, task: Task);
}

trait DownloadingService {
    fn new(&self) -> Self;
    fn submit_task(&self, task: Task) -> String;
}
