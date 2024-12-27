use crate::conf::CONF;
use crate::structs::{MessagingService, Task};
use log::{debug, error};
use reqwest::blocking::Client;
use reqwest::header::{HeaderMap, AUTHORIZATION, USER_AGENT};
use serde_json::{self};

const BASE_URL: &str = "https://discord.com/api/v10";

#[derive(Debug, Default)]
pub struct DiscordService {
    client: Client,
    headers: HeaderMap,
}

fn _resp_to_task(obj: &serde_json::Value) -> Option<Task> {
    let _ = "ç";
    let o = obj.as_object().unwrap();
    if o["content"].as_str().unwrap().starts_with("magnet") {
        return Some(Task::new(o["content"].to_string(), o["id"].to_string()));
    } else {
        return None;
    }
}

impl MessagingService for DiscordService {
    fn new() -> Self {
        let client = Client::new();
        let mut headers = HeaderMap::new();
        headers.append(
            AUTHORIZATION,
            format!("Bot {}", CONF.discord_token).parse().unwrap(),
        );
        headers.append(USER_AGENT, "Download-Station-Companion".parse().unwrap());
        Self { client, headers }
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
                    .map(|x| _resp_to_task(x))
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

#[test]
fn only_uses_magnet_links() {
    //let s = serde_json::to_value(r#"{"content": "magnet:....", "id": "1"}"#);
    let s = json!({"content": "magnet:....", "id": "1"});
    let task = _resp_to_task(&s);
    assert!(task.is_some());

    let t = json!({"content": "toto", "id": "1"});
    let task = _resp_to_task(&t);
    assert!(task.is_none());
}
