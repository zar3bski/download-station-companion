use bytes::Bytes;

use reqwest::{
    blocking::{multipart::Form, Body},
    Method, Url,
};
use serde_json::Value;

use super::task::Task;

pub trait MessagingController {
    fn new() -> Self
    where
        Self: Sized;
    fn fetch_tasks(&self) -> Option<Vec<Task>>;
    fn update_task_status(&self, task: &mut Task, message: Option<&str>);
}

pub trait DownloadingController {
    fn new() -> Self;
    fn submit_task(&self, task: &mut Task);
    fn get_jobs_advancement(&self, tasks: &mut Vec<Task>);
}

#[allow(dead_code)]
pub enum Payload {
    BODY(Body),
    FORM(Form), // dead code, to be implemented
}

pub trait HTTPService {
    fn new() -> Self;
    fn send_request(&self, url: Url, method: Method, payload: Option<Payload>) -> Option<Value>;
    fn download_file(&self, url: Url) -> Option<Bytes>;
}
