use std::io::Cursor;
use std::str::FromStr;

use crate::conf::CONF;
use crate::task::{Source, Task, TaskStatus};
use crate::traits::{HTTPService, MessagingController, Payload};
use bytes::Bytes;
use chrono::{DateTime, TimeDelta, Utc};
use log::{debug, error, warn};
use regex::Regex;
use reqwest::blocking::{Body, Client};
use reqwest::header::{self, HeaderValue, AUTHORIZATION, CONTENT_TYPE, USER_AGENT};

use reqwest::{Method, Url};
use serde::Deserialize;
use serde_json::{self, json, Value};

use super::API_USER_AGENT;

const BASE_URL: &str = "https://discord.com/api/v10";

#[allow(dead_code)]
#[derive(Deserialize)]
struct AttachementObject {
    id: String,
    filename: String,
    url: String,
    proxy_url: String,
}

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
    fn download_file(&self, url: Url) -> Option<Bytes> {
        debug!("Downloading .torrent file from {}", url);
        let resp = self
            .client
            .get(url)
            .header(
                CONTENT_TYPE,
                HeaderValue::from_static("application/x-bittorrent"),
            )
            .send();
        //TODO: make a macro for this
        match resp {
            Ok(resp) => {
                if resp.status().as_u16() < 300 {
                    let b = resp.bytes();
                    match b {
                        Ok(b) => {
                            debug!(".torrent file downloaded successfuly");
                            return Some(b);
                        }
                        Err(_) => {
                            error!("Could not extract .torrent bytes from response");
                            return None;
                        }
                    }
                } else {
                    error!("Could not download .torrent status code={}", resp.status());
                    return None;
                }
            }
            Err(e) => {
                error!("Could not download .torrent: {}", e);
                return None;
            }
        }
    }

    fn send_request(&self, url: Url, method: Method, payload: Option<Payload>) -> Option<Value> {
        let url_log = url.clone();
        let req = match payload {
            Some(payload) => match payload {
                Payload::BODY(body) => self.client.request(method, url).body(body),
                Payload::FORM(form) => self.client.request(method, url).multipart(form),
            },
            None => self.client.request(method, url),
        };

        let resp = req.send();

        match resp {
            Ok(resp) => {
                if resp.status().as_u16() < 300 {
                    return Some(resp.json().unwrap());
                } else {
                    warn!("Could not request {}. response: ", url_log);
                    return None;
                }
            }
            Err(e) => {
                error!("Could not request {}. Error: {}", url_log, e);
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
    if DateTime::parse_from_str(o["timestamp"].as_str().unwrap(), "%+").unwrap() > after {
        let id = String::from(o["id"].as_str().unwrap());
        let user_id = String::from(o["author"]["id"].as_str().unwrap());
        let content = String::from(o["content"].as_str().unwrap());

        // content parsing
        let re_magnet = Regex::new(r"^(?<magnet>magnet:[^\n]+)").unwrap();
        let re_destination = Regex::new(r"[t|T]o:\s*(?<path>[\w\/\s]*)\s*$").unwrap();
        let magnet_match = re_magnet.captures(&content);
        let destination_match = re_destination.captures(&content);

        // attachment extraction
        let attachment: Option<Bytes> = {
            if o.contains_key("attachments") {
                match o["attachments"].as_array() {
                    Some(arr) => {
                        if arr.len() == 0 {
                            None
                        } else {
                            let attachement: AttachementObject =
                                serde_json::from_value(arr.get(0).unwrap().clone()).unwrap();
                            let resp = notifier
                                .service
                                .download_file(Url::parse(attachement.url.as_str()).unwrap());
                            resp
                        }
                    }
                    None => None,
                }
            } else {
                None
            }
        };

        match magnet_match {
            Some(magnet) => {
                return Some(Task::new(
                    Source::MAGNET(String::from(magnet["magnet"].trim())),
                    id,
                    notifier,
                    if destination_match.is_some() {
                        Some(String::from(&destination_match.unwrap()["path"]))
                    } else {
                        None
                    },
                    user_id,
                ))
            }
            None => match attachment {
                Some(attachment) => {
                    return Some(Task::new(
                        Source::FILE(attachment),
                        id,
                        notifier,
                        if destination_match.is_some() {
                            Some(String::from(&destination_match.unwrap()["path"]))
                        } else {
                            None
                        },
                        user_id,
                    ));
                }
                None => {
                    warn!("No magnet link not .torrent found in message");
                    return None;
                }
            },
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
        let content = if message.is_none() {
            if task.get_status() == TaskStatus::DONE || task.get_status() == TaskStatus::FAILED {
                task.get_status().to_string()
                    + &String::from_str(&format!(" <@{}>", task.user_id)).unwrap()
            } else {
                task.get_status().to_string()
            }
        } else {
            message.unwrap().to_string()
        };
        let body = json!({"content":content, "message_reference":{"message_id":task.message_id}, "allowed_mentions": {"users": [task.user_id]}});

        let cursor = Cursor::new(body.to_string());
        let url =
            Url::parse(format!("{BASE_URL}/channels/{}/messages", CONF.discord_channel).as_str())
                .unwrap();

        let resp =
            self.service
                .send_request(url, Method::POST, Some(Payload::BODY(Body::new(cursor))));

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
        let url =
            Url::parse(format!("{BASE_URL}/channels/{}/messages", CONF.discord_channel).as_str())
                .unwrap();

        let resp = self.service.send_request(url, Method::GET, None);
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
    use std::sync::Mutex;

    use bytes::Bytes;
    use reqwest::{Method, Url};
    use serde_json::{json, Value};

    use crate::{
        services::discord::DiscordController,
        task::Source,
        traits::{HTTPService, MessagingController, Payload},
    };

    #[test]
    fn only_uses_magnet_links() {
        struct DiscordServiceMock {}
        impl HTTPService for DiscordServiceMock {
            fn new() -> Self {
                Self {}
            }
            fn download_file(&self, _: Url) -> Option<Bytes> {
                panic!("Not implemented")
            }
            fn send_request(&self, _: Url, _: Method, _: Option<Payload>) -> Option<Value> {
                return Some(json!([
                    {"content": "magnet:aaaa", "id": "1","timestamp": "2044-12-25T19:07:12.600000+00:00", "author":{"id":"xxx"}},
                    {"content": "notmagnet:....", "id": "2","timestamp": "2044-12-25T19:07:12.600000+00:00", "author":{"id":"xxx"}}
                ]));
            }
        }

        let controler = DiscordController::<DiscordServiceMock>::new();
        let mut tasks = controler.fetch_tasks().unwrap();
        assert!(tasks.len() == 1);
        let task = tasks.pop().unwrap();

        //task analysis
        assert!(task.source == Source::MAGNET("magnet:aaaa".to_string()));
        assert!(task.user_id == "xxx")
    }

    #[test]
    fn load_tasks_posterior_to_datetime_delta() {
        struct DiscordServiceMock {}
        impl HTTPService for DiscordServiceMock {
            fn new() -> Self {
                Self {}
            }
            fn download_file(&self, _: Url) -> Option<Bytes> {
                panic!("Not implemented")
            }
            fn send_request(&self, _: Url, _: Method, _: Option<Payload>) -> Option<Value> {
                return Some(json!([
                    {"content": "magnet:bbbb", "id": "3","timestamp": "2004-12-25T19:07:12.600000+00:00", "author":{"id":"xxx"}},
                    {"content": "magnet:cccc  ", "id": "4","timestamp": "2044-12-25T19:07:12.600000+00:00", "author":{"id":"xxx"}}
                ]));
            }
        }
        let controler = DiscordController::<DiscordServiceMock>::new();
        let mut tasks = controler.fetch_tasks().unwrap();
        assert!(tasks.len() == 1);
        //task analysis
        let task = tasks.pop().unwrap();
        assert!(task.source == Source::MAGNET("magnet:cccc".to_string())) // check properly trimed
    }

    #[test]
    fn set_destination_folder() {
        struct DiscordServiceMock {}
        impl HTTPService for DiscordServiceMock {
            fn new() -> Self {
                Self {}
            }
            fn download_file(&self, _: Url) -> Option<Bytes> {
                panic!("Not implemented")
            }
            fn send_request(&self, _: Url, _: Method, _: Option<Payload>) -> Option<Value> {
                return Some(json!([
                    {"content": "magnet:bbbb\nTo: videos/Movies", "id": "5","timestamp": "2044-12-25T19:07:12.600000+00:00", "author":{"id":"xxx"}},
                    {"content": "magnet:bbbb\nto:videos/Series", "id": "6","timestamp": "2044-12-25T19:07:12.600000+00:00", "author":{"id":"xxx"}},
                    {"content": "magnet:bbbb\nTo: videos/ Somewhere", "id": "7","timestamp": "2044-12-25T19:07:12.600000+00:00", "author":{"id":"xxx"}}
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

    #[test]
    fn file_handling() {
        struct DiscordServiceMock {
            time_called: Mutex<i8>,
        }
        impl HTTPService for DiscordServiceMock {
            fn new() -> Self {
                let time_called: Mutex<i8> = Mutex::new(0);
                Self { time_called }
            }
            fn download_file(&self, _: reqwest::Url) -> Option<Bytes> {
                let file = Bytes::from("Hello world");
                return Some(file);
            }
            fn send_request(&self, _: Url, _: Method, _: Option<Payload>) -> Option<Value> {
                let mut value = self.time_called.lock().unwrap();
                *value += 1;
                match *value {
                    1 => {
                        // first request, get messages
                        return Some(json!([
                            {
                                "attachments":
                                [
                                    {
                                        "content_scan_version":0,
                                        "content_type":"application/x-bittorrent",
                                        "filename":"debian-12.9.0-amd64-DVD-1.iso.torrent",
                                        "id":"1331590988820385832",
                                        "proxy_url":"https://media.discordapp.net/attachments/1321495709262024705/1331590988820385832/debian-12.9.0-amd64-DVD-1.iso.torrent?ex=67922c3f&is=6790dabf&hm=8cd67397b3dc6985d8702e4f8adc1a5654eacc74e0b16105edad49a4dff06fb3&",
                                        "size":304228,
                                        "url":"https://cdn.discordapp.com/attachments/1321495709262024705/1331590988820385832/debian-12.9.0-amd64-DVD-1.iso.torrent?ex=67922c3f&is=6790dabf&hm=8cd67397b3dc6985d8702e4f8adc1a5654eacc74e0b16105edad49a4dff06fb3&"
                                    }
                                ],"content": "to: videos/Movies", "id": "5","timestamp": "2044-12-25T19:07:12.600000+00:00", "author":{"id":"xxx"}
                            },
                        ]));
                    }
                    _ => return None,
                }
            }
        }

        let controler = DiscordController::<DiscordServiceMock>::new();
        let mut tasks = controler.fetch_tasks().unwrap();
        assert!(tasks.len() == 1);

        let task = tasks.pop().unwrap();
        assert!(task.source == Source::FILE(Bytes::from("Hello world")))
    }
}
