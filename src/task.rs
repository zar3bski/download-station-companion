use crate::traits::MessagingController;
use bytes::Bytes;
use core::fmt;

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
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

#[derive(PartialEq)]
pub enum Source {
    FILE(Bytes),
    MAGNET(String),
}

pub struct Task<'a> {
    status: TaskStatus,
    pub message_id: String,
    pub source: Source,
    pub notifier: &'a dyn MessagingController,
    pub destination_folder: Option<String>,
    pub user_id: String,
}

impl<'a> Task<'a> {
    pub fn new(
        source: Source,
        message_id: String,
        notifier: &'a dyn MessagingController,
        destination_folder: Option<String>,
        user_id: String,
    ) -> Self {
        Self {
            source,
            message_id,
            status: TaskStatus::RECEIVED,
            notifier,
            destination_folder,
            user_id,
        }
    }
    // Update private field status and call the associated
    // notifier
    pub fn set_status(&mut self, status: TaskStatus) {
        self.status = status;
        self.notifier.update_task_status(self, None);
    }

    pub fn get_status(&self) -> TaskStatus {
        self.status
    }
}
