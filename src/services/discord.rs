use crate::conf::CONF;
use crate::structs::{MessagingService, API_USER_AGENT};
use crate::task::{Task, TaskStatus};
use chrono::{DateTime, TimeDelta, Utc};
use log::{debug, error};
use mockall::automock;
use reqwest::blocking::Client;
use reqwest::header::{self, HeaderMap, AUTHORIZATION, USER_AGENT};
use serde_json::{self, json};

const BASE_URL: &str = "https://discord.com/api/v10";

#[derive(Debug)]
pub struct DiscordService {
    client: Client,
    headers: HeaderMap,
}

fn _resp_to_task(obj: serde_json::Value, notifier: &DiscordService) -> Option<Task> {
    let o = obj.as_object().unwrap();
    let after: chrono::DateTime<Utc> = Utc::now() - TimeDelta::minutes(CONF.minutes_delta as i64);
    if o["content"].as_str().unwrap().starts_with("magnet") {
        if DateTime::parse_from_str(o["timestamp"].as_str().unwrap(), "%+").unwrap() > after {
            // TODO test delta filter
            let content = String::from(o["content"].as_str().unwrap());
            let id = String::from(o["id"].as_str().unwrap());
            let mut task = Task::new(content, id, notifier);
            return Some(task);
        } else {
            return None;
        }
    } else {
        return None;
    }
}

impl MessagingService for DiscordService {
    fn new() -> Self
    where
        Self: Sized,
    {
        let client = Client::new();
        let mut headers = HeaderMap::new();
        headers.append(
            AUTHORIZATION,
            format!("Bot {}", CONF.discord_token).parse().unwrap(),
        );
        headers.append(USER_AGENT, header::HeaderValue::from_static(API_USER_AGENT));

        return Self { client, headers };
    }

    fn update_task_status(&self, task: &mut Task) {
        let body = json!({"content":task.get_status(), "message_reference":{"message_id":task.message_id}});
        let resp = self
            .client
            .post(format!(
                "{BASE_URL}/channels/{}/messages",
                CONF.discord_channel
            ))
            .headers(self.headers.clone())
            .json(&body)
            .send();

        match resp {
            Ok(res) => {
                debug!(
                    "Response received from channel {} status code: {}: {}",
                    CONF.discord_channel,
                    res.status(),
                    res.text().unwrap()
                );
            }
            Err(err) => {
                error!("Could not notify message_id: {}: {err}", task.message_id);
            }
        }
    }

    fn fetch_tasks(&self) -> Option<Vec<Task>> {
        let resp: Result<reqwest::blocking::Response, reqwest::Error> = self
            .client
            .get(format!(
                "{BASE_URL}/channels/{}/messages
",
                CONF.discord_channel
            ))
            .headers(self.headers.clone())
            .send();
        match resp {
            Ok(res) => {
                debug!(
                    "Response received from channel {} status code: {}",
                    CONF.discord_channel,
                    res.status()
                );
                //FIXME : lourdingue
                let tasks: Vec<Task> = res
                    .json::<serde_json::Value>()
                    .unwrap()
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|x| _resp_to_task(x.clone(), &self))
                    .filter(|x| x.is_some())
                    .map(|x| x.unwrap())
                    .collect();

                return Some(tasks);
            }
            Err(err) => {
                error!(
                    "Could not retrieve tasks from discord channel_id {}: {err}",
                    CONF.discord_channel
                );
                return None;
            }
        }
    }
}

/////Unit Tests/////

//TODO: fix tests
#[test]
fn only_uses_magnet_links() {
    let notifier: DiscordService = DiscordService::new();
    let s = json!({"content": "magnet:....", "id": "1","timestamp": "2044-12-25T19:07:12.600000+00:00"});
    let task = _resp_to_task(s.clone(), &notifier);
    assert!(task.is_some());
    let t = task.unwrap();
    assert!(t.message_id == "1");
    assert!(t.magnet_link == "magnet:....");
    assert!(t.status == TaskStatus::RECEIVED);

    let t = json!({"content": "toto", "id": "1","timestamp": "2044-12-25T19:07:12.600000+00:00"});
    let task = _resp_to_task(t.clone(), &notifier);
    assert!(task.is_none());
}
