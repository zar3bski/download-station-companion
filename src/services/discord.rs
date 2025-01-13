use std::io::Cursor;

use crate::conf::CONF;
use crate::task::Task;
use crate::traits::{HTTPService, MessagingController};
use chrono::{DateTime, TimeDelta, Utc};
use log::{debug, error, warn};
use regex::Regex;
use reqwest::blocking::{Body, Client, Request};
use reqwest::header::{self, AUTHORIZATION, CONTENT_TYPE, USER_AGENT};

use reqwest::{Method, Url};
use serde_json::{self, json, Value};

use super::API_USER_AGENT;

const BASE_URL: &str = "https://discord.com/api/v10";

#[derive(Default)]
pub struct DiscordController<T> {
    service: T,
}

#[derive(Default)]
pub struct DiscordService {
    client: Client,
}

impl HTTPService for DiscordService {
    fn new() -> Self {
        let client = Client::builder()
            .default_headers(
                [
                    (
                        AUTHORIZATION,
                        format!("Bot {}", CONF.discord_token).parse().unwrap(),
                    ),
                    (USER_AGENT, header::HeaderValue::from_static(API_USER_AGENT)),
                    (
                        CONTENT_TYPE,
                        header::HeaderValue::from_static(super::API_CONTENT_TYPE),
                    ),
                ]
                .into_iter()
                .collect(),
            )
            .build()
            .unwrap();
        Self { client }
    }

    fn send_request(&self, req: Request) -> Option<Value> {
        let url = req.url().clone();
        let resp = self.client.execute(req);
        match resp {
            Ok(resp) => {
                if resp.status().as_u16() < 300 {
                    return resp.json().unwrap();
                } else {
                    warn!(
                        "Could not request {}. response: {}",
                        url,
                        resp.json::<Value>().unwrap()
                    );
                    return None;
                }
            }
            Err(e) => {
                error!("Could not request {}. Error: {}", url, e);
                return None;
            }
        }
    }
}

fn _resp_to_task<T: HTTPService>(
    obj: serde_json::Value,
    notifier: &DiscordController<T>,
) -> Option<Task> {
    let o = obj.as_object().unwrap();
    let after: chrono::DateTime<Utc> = Utc::now() - TimeDelta::minutes(CONF.minutes_delta as i64);
    if o["content"].as_str().unwrap().starts_with("magnet") {
        if DateTime::parse_from_str(o["timestamp"].as_str().unwrap(), "%+").unwrap() > after {
            let id = String::from(o["id"].as_str().unwrap());
            let mut content = o["content"].as_str().unwrap().split('\n');
            let magnet_link = String::from(content.next().unwrap());
            let destination_folder = content.next();
            match destination_folder {
                Some(dest) => {
                    let re = Regex::new(r"^[t|T]o:\s*(?<path>[\w\/\s]*)\s*$").unwrap();
                    let path_match = re.captures(dest);
                    match path_match {
                        Some(path_match) => {
                            return Some(Task::new(
                                magnet_link,
                                id,
                                notifier,
                                Some(String::from(String::from(&path_match["path"]))),
                            ));
                        }
                        None => {
                            //notifier.update_task_status(task);
                            //TODO: notify user that path parsing did not work
                            let mut task = Task::new(magnet_link, id, notifier, None);
                            notifier.update_task_status(&mut task, Some("Destination path could not be parsed, using default destination folder"));
                            return Some(task);
                        }
                    }
                }
                None => {
                    return Some(Task::new(magnet_link, id, notifier, None));
                }
            }
        } else {
            return None;
        }
    } else {
        return None;
    }
}

impl<T: HTTPService> MessagingController for DiscordController<T> {
    fn new() -> Self
    where
        Self: Sized,
        T: Sized,
    {
        let service = T::new();
        return Self { service };
    }

    fn update_task_status(&self, task: &mut Task, message: Option<&str>) {
        let content = if (message.is_none()) {
            task.get_status().to_string()
        } else {
            message.unwrap().to_string()
        };
        let body = json!({"content":content, "message_reference":{"message_id":task.message_id}});
        let cursor = Cursor::new(body.to_string());
        let url = format!("{BASE_URL}/channels/{}/messages", CONF.discord_channel);
        let mut req = Request::new(Method::POST, Url::parse(url.as_str()).unwrap());
        *req.body_mut() = Some(Body::new(cursor));

        let resp = self.service.send_request(req);

        match resp {
            Some(res) => {
                debug!(
                    "Response received from channel {}: {:?}",
                    CONF.discord_channel, res
                );
            }
            None => {
                error!("Could not notify message_id: {}", task.message_id);
            }
        }
    }

    fn fetch_tasks(&self) -> Option<Vec<Task>> {
        let url = format!("{BASE_URL}/channels/{}/messages", CONF.discord_channel);
        let req = Request::new(Method::GET, Url::parse(url.as_str()).unwrap());

        let resp = self.service.send_request(req);
        match resp {
            Some(res) => {
                let tasks: Vec<Task> = res
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|x| _resp_to_task(x.clone(), &self))
                    .filter(|x| x.is_some())
                    .map(|x| x.unwrap())
                    .collect();

                return Some(tasks);
            }
            None => {
                error!(
                    "Could not retrieve tasks from discord channel_id {}",
                    CONF.discord_channel
                );
                return None;
            }
        }
    }
}

/////Unit Tests/////

#[cfg(test)]
pub mod tests {
    use reqwest::blocking::Request;
    use serde_json::{json, Value};

    use crate::{
        services::discord::DiscordController,
        traits::{HTTPService, MessagingController},
    };

    #[test]
    fn only_uses_magnet_links() {
        struct DiscordServiceMock {}
        impl HTTPService for DiscordServiceMock {
            fn new() -> Self {
                Self {}
            }
            fn send_request(&self, _: Request) -> Option<Value> {
                return Some(json!([
                    {"content": "magnet:aaaa", "id": "1","timestamp": "2044-12-25T19:07:12.600000+00:00"},
                    {"content": "notmagnet:....", "id": "2","timestamp": "2044-12-25T19:07:12.600000+00:00"}
                ]));
            }
        }

        let controler = DiscordController::<DiscordServiceMock>::new();
        let mut tasks = controler.fetch_tasks().unwrap();
        assert!(tasks.len() == 1);
        let task = tasks.pop().unwrap();

        //task analysis
        assert!(task.magnet_link == "magnet:aaaa")
    }

    #[test]
    fn load_tasks_posterior_to_datetime_delta() {
        struct DiscordServiceMock {}
        impl HTTPService for DiscordServiceMock {
            fn new() -> Self {
                Self {}
            }
            fn send_request(&self, _: Request) -> Option<Value> {
                return Some(json!([
                    {"content": "magnet:bbbb", "id": "3","timestamp": "2004-12-25T19:07:12.600000+00:00"},
                    {"content": "magnet:cccc", "id": "4","timestamp": "2044-12-25T19:07:12.600000+00:00"}
                ]));
            }
        }
        let controler = DiscordController::<DiscordServiceMock>::new();
        let mut tasks = controler.fetch_tasks().unwrap();
        assert!(tasks.len() == 1);
        //task analysis
        let task = tasks.pop().unwrap();
        assert!(task.magnet_link == "magnet:cccc")
    }

    #[test]
    fn set_destination_folder() {
        struct DiscordServiceMock {}
        impl HTTPService for DiscordServiceMock {
            fn new() -> Self {
                Self {}
            }
            fn send_request(&self, _: Request) -> Option<Value> {
                return Some(json!([
                    {"content": "magnet:bbbb\nTo: videos/Movies", "id": "5","timestamp": "2044-12-25T19:07:12.600000+00:00"},
                    {"content": "magnet:bbbb\nto:videos/Series", "id": "6","timestamp": "2044-12-25T19:07:12.600000+00:00"},
                    {"content": "magnet:bbbb\nTo: videos/ Somewhere", "id": "7","timestamp": "2044-12-25T19:07:12.600000+00:00"}
                ]));
            }
        }
        let controler = DiscordController::<DiscordServiceMock>::new();
        let mut tasks = controler.fetch_tasks().unwrap();
        assert!(tasks.len() == 3);
        //task analysis

        let task = tasks.pop().unwrap();
        assert!(task.destination_folder.unwrap() == "videos/ Somewhere");

        let task = tasks.pop().unwrap();
        assert!(task.destination_folder.unwrap() == "videos/Series");

        let task = tasks.pop().unwrap();
        assert!(task.destination_folder.unwrap() == "videos/Movies");
    }
}
