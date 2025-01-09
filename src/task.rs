use crate::traits::MessagingController;
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

pub struct Task<'a> {
    status: TaskStatus,
    pub message_id: String,
    pub magnet_link: String,
    pub notifier: &'a dyn MessagingController,
}

impl<'a> Task<'a> {
    pub fn new(
        magnet_link: String,
        message_id: String,
        notifier: &'a dyn MessagingController,
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
    pub fn set_status(&mut self, status: TaskStatus) {
        self.status = status;
        self.notifier.update_task_status(self);
    }

    pub fn get_status(&self) -> TaskStatus {
        self.status
    }
}
