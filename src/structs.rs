use core::fmt;

pub const API_USER_AGENT: &str = "Download-Station-Companion";

#[derive(Debug, PartialEq, Eq, Hash)]
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

//#[derive(Debug)]
pub struct Task<'a> {
    pub status: TaskStatus,
    pub message_id: String,
    pub magnet_link: String,
    pub notifier: &'a dyn MessagingService,
}

impl<'a> Task<'a> {
    pub fn new(
        magnet_link: String,
        message_id: String,
        notifier: &'a dyn MessagingService,
    ) -> Self {
        Self {
            magnet_link,
            message_id,
            status: TaskStatus::RECEIVED,
            notifier,
        }
    }
    // Update private field status and call the associated
    // notifier
    pub fn set_status(mut self, status: TaskStatus) {
        self.status = status;
        self.notifier.update_task_status(&self);
    }

    pub fn get_status(&self) -> String {
        self.status.to_string()
    }
}

pub trait MessagingService {
    fn new() -> Self
    where
        Self: Sized;
    fn fetch_tasks(&self) -> Option<Vec<Task>>;
    fn update_task_status(&self, task: &Task);
}

pub trait DownloadingService {
    fn new() -> Self;
    fn submit_task(&self, task: Task);
    fn drop(&self);
}
