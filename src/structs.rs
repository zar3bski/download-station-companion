use core::fmt;
use std::{mem, ptr::NonNull};

use crate::task::Task;

pub const API_USER_AGENT: &str = "Download-Station-Companion";

pub trait MessagingService {
    fn new() -> Self
    where
        Self: Sized;
    fn fetch_tasks(&self) -> Option<Vec<Task>>;
    fn update_task_status(&self, task: &mut Task);
}

pub trait DownloadingService {
    fn new() -> Self;
    fn submit_task(&self, task: &mut Task);
    fn get_jobs_advancement(&self, tasks: &mut Vec<Task>);
    fn drop(&self);
}
