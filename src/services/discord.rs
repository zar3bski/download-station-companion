use crate::conf::CONF;
use crate::structs::{MessagingService, Task};
use log::{debug, error, info, warn};
use reqwest::blocking::Client;
use reqwest::header::{HeaderMap, AUTHORIZATION, USER_AGENT};
use serde_json;

const BASE_URL: &str = "https://discord.com/api/v10";

#[derive(Debug, Default)]
pub struct DiscordService {
    client: Client,
    headers: HeaderMap,
}

//fn _resp_to_task() -> Task {}

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

    fn fetch_tasks(&self) -> Vec<Task> {
        let tasks: Vec<Task> = vec![];
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
                let messages = res.json::<serde_json::Value>().unwrap(); // TODO: parse and create tasks
            }
            Err(err) => {
                error!(
                    "Could not retrieve tasks from discord channel_id {}: {err}",
                    CONF.discord_channel
                );
            }
        }

        return tasks;
    }
}
