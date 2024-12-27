use std::iter::Enumerate;

#[derive(Debug)]
enum TaskStatus {
    RECEIVED,
    SUBMITTED,
    PENDING,
    FAILED,
    DONE,
}

#[derive(Debug)]
pub struct Task {
    status: TaskStatus,
    message_id: String,
    magnet_link: String,
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
}

trait DownloadingService {
    fn new(&self) -> Self;
    fn submit_task(&self, task: Task) -> String;
}
