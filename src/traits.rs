use reqwest::blocking::Request;
use serde_json::Value;

use crate::task::Task;

pub trait MessagingController {
    fn new() -> Self
    where
        Self: Sized;
    fn fetch_tasks(&self) -> Option<Vec<Task>>;
    fn update_task_status(&self, task: &mut Task);
}

pub trait DownloadingController {
    fn new() -> Self;
    fn submit_task(&self, task: &mut Task);
    fn get_jobs_advancement(&self, tasks: &mut Vec<Task>);
    //fn drop(self);
}

pub trait HTTPService {
    fn new() -> Self;
    fn send_request(&self, req: Request) -> Option<Value>;
}
