use std::collections::HashMap;
use std::str::FromStr;
use std::sync::Arc;

use crate::conf::CONF;
use crate::task::{Task, TaskStatus};
use crate::traits::{DownloadingController, HTTPService};
use log::{debug, error};
use once_cell::sync::Lazy;
use reqwest::blocking::{Client, Request};
use reqwest::header::{self, USER_AGENT};
use reqwest::{Method, Url};
use serde::Deserialize;
use serde_json::Value;

use super::API_USER_AGENT;

//https://global.download.synology.com/download/Document/Software/DeveloperGuide/Package/DsControler/All/enu/Synology_Download_Station_Web_API.pdf

#[allow(dead_code)]
#[derive(Deserialize)]
struct InfoResponse {
    data: ApiInformation,
    success: bool,
}

#[allow(dead_code)]
#[derive(Deserialize, Debug)]
struct SynoApiAuth {
    path: String,
    #[serde(rename = "minVersion")]
    min_version: usize,
    #[serde(rename = "maxVersion")]
    max_version: usize,
}

#[allow(dead_code)]
#[derive(Deserialize, Debug)]
struct SynoDsControlerTask {
    path: String,
    #[serde(rename = "minVersion")]
    min_version: usize,
    #[serde(rename = "maxVersion")]
    max_version: usize,
}

#[derive(Deserialize, Debug)]
struct ApiInformation {
    #[serde(rename = "SYNO.API.Auth")]
    auth: SynoApiAuth,
    #[serde(rename = "SYNO.DownloadStation.Task")]
    task: SynoDsControlerTask,
}

pub static DS_TO_COMPANION_MAPPING: Lazy<Arc<HashMap<&'static str, TaskStatus>>> =
    Lazy::new(|| {
        let hash = HashMap::from([
            ("waiting", TaskStatus::SUBMITTED),
            ("downloading", TaskStatus::DOWNLOADING),
            ("paused", TaskStatus::DOWNLOADING),
            ("finishing", TaskStatus::DOWNLOADING),
            ("finished", TaskStatus::DONE),
            ("hash_checking", TaskStatus::SUBMITTED),
            ("seeding", TaskStatus::DONE),
            ("filehosting_waiting", TaskStatus::SUBMITTED),
            ("extracting", TaskStatus::DOWNLOADING),
            ("error", TaskStatus::FAILED),
        ]);
        Arc::new(hash)
    });

pub struct DsControler<T> {
    service: T,
}

#[allow(dead_code)]
pub struct DsService {
    client: Client,
    sid: String,
    api_information: ApiInformation,
    root_url: Url,
}

impl HTTPService for DsService {
    fn new() -> Self {
        let client = Client::builder()
            .default_headers(
                [(USER_AGENT, header::HeaderValue::from_static(API_USER_AGENT))]
                    .into_iter()
                    .collect(),
            )
            .build()
            .unwrap();

        // API Info
        let resp = client
            .get(format!(
                "{}/webapi/query.cgi?api=SYNO.API.Info&version=1&method=query&query=SYNO.API.Auth,SYNO.DownloadStation.Task",
                CONF.synology_root_api
            ))
            .send();

        match resp {
            Ok(resp) => {
                let root: InfoResponse =
                    serde_json::from_str(resp.text().unwrap().as_str()).unwrap();
                let api_information = root.data;
                debug!(
                    "Information gathered for Synology API: {:?}",
                    api_information
                );
                // auth
                let auth_resp = client
                    .get(format!(
                        "{}/webapi/{}?api=SYNO.API.Auth&version={}&method=login&account={}&passwd={}&session=DownloadStation&format=sid",
                        CONF.synology_root_api,
                        api_information.auth.path,
                        6,
                        CONF.synology_user,
                        CONF.synology_password
                    ))
                    .send()
                    .unwrap();
                debug!(
                    "Auth request received status:{} headers: {:?}",
                    auth_resp.status(),
                    auth_resp.headers()
                );
                let data: Value = serde_json::from_str(&auth_resp.text().unwrap()).unwrap();
                if data.get("success").unwrap() == true {
                    debug!("Login successful: {:?}", data);

                    let sid = String::from_str(data["data"]["sid"].as_str().unwrap()).unwrap();
                    let url = format!(
                        "{}/webapi/{}",
                        CONF.synology_root_api, api_information.task.path,
                    );
                    let root_url = Url::parse(url.as_str()).unwrap();
                    debug!("sid: {:?} root_url: {}", sid, root_url);
                    return Self {
                        client: client,
                        api_information: api_information,
                        sid,
                        root_url,
                    };
                } else {
                    error!("Could not login to Synology API: {}", data.to_string());
                    panic!()
                }
            }
            Err(err) => {
                error!("Could not get Download station API information: {err}");
                panic!()
            }
        }
    }

    fn send_request(&self, req: Request) -> Option<Value> {
        // FIXME This entire mess comes from the fact that reqwest does not
        // allow relative urls. Remove this decoration when a solution
        // is found
        let method = req.method().clone();
        let url = req.url().to_string();
        let query = &url[url.find("?").unwrap()..url.len()];
        let query = query.to_owned() + format!("&_sid={}", self.sid).as_str();
        let url = self.root_url.join(&query).unwrap();
        let req = Request::new(method, url);
        debug!("Request: {:?}", req);
        let resp = self.client.execute(req);

        match resp {
            Ok(resp) => {
                if resp.status().as_u16() < 300 {
                    return resp.json().unwrap();
                } else {
                    return None;
                }
            }
            Err(e) => {
                error!("Could not request. Error: {}", e);
                return None;
            }
        }
    }
}

impl<T: HTTPService> DownloadingController for DsControler<T> {
    fn new() -> Self
    where
        Self: Sized,
        T: Sized,
    {
        let service = T::new();
        return Self { service };
    }

    fn get_jobs_advancement(&self, tasks: &mut Vec<Task>) {
        let url = format!(
            "{}?api=SYNO.DownloadStation.Task&version=1&session=DownloadStation&method=list&additional=detail&username={}",
            CONF.synology_root_api, CONF.synology_user
        );
        let req = Request::new(Method::GET, Url::parse(url.as_str()).unwrap());
        let resp = self.service.send_request(req).unwrap();

        if resp["success"] == true {
            let distant_tasks: &Vec<Value> = resp["data"]["tasks"].as_array().unwrap();
            for obj in distant_tasks {
                let uri = obj["additional"]["detail"]["uri"].as_str().unwrap();
                let status = obj["status"].as_str().unwrap();
                for task_idx in 0..tasks.len() {
                    let task = &mut tasks[task_idx];

                    if task.magnet_link == uri
                        && DS_TO_COMPANION_MAPPING[status] != task.get_status()
                    {
                        let s: TaskStatus = DS_TO_COMPANION_MAPPING[status];
                        task.set_status(s);
                        if s == TaskStatus::DONE || s == TaskStatus::FAILED {
                            tasks.remove(task_idx);
                        }
                    } else {
                        debug!("Nothing new for task: {}", task.magnet_link);
                    }
                }
            }
        } else {
            error!("Could not withdraw job status: {resp}")
        }
    }

    fn submit_task(&self, task: &mut Task) {
        let mut url = format!(
            "{}?api=SYNO.DownloadStation.Task&version=1&session=DownloadStation&method=create&uri={}",
            CONF.synology_root_api,
            urlencoding::encode(task.magnet_link.as_str())
        );

        if task.destination_folder.is_some() {
            url.push_str(
                format!(
                    "&destination={}",
                    urlencoding::encode(task.destination_folder.clone().unwrap().as_str())
                        .into_owned()
                )
                .as_str(),
            );
        }

        let req = Request::new(Method::GET, Url::parse(url.as_str()).unwrap());
        let resp = self.service.send_request(req);

        match resp {
            Some(res) => {
                debug!("Task submitted successfully: {}", res);
                task.set_status(TaskStatus::SUBMITTED);
            }
            None => {
                error!("Could not submit download task: {}", task.message_id);
            }
        }
    }
}

#[cfg(test)]
pub mod tests {

    use std::{cell::RefCell, str::FromStr};

    use reqwest::{blocking::Request, Method, Url};
    use serde_json::{json, Value};

    use crate::{
        services::{
            discord::DiscordController,
            download_station::{DsControler, DS_TO_COMPANION_MAPPING},
        },
        task::{Task, TaskStatus},
        traits::{DownloadingController, HTTPService, MessagingController},
    };

    struct DiscordServiceMock {}
    impl HTTPService for DiscordServiceMock {
        fn new() -> Self {
            Self {}
        }
        fn send_request(&self, _: Request) -> Option<Value> {
            return Some(json!({}));
        }
    }

    #[test]
    fn status_mapping() {
        let s = String::from_str("downloading").unwrap();
        assert!(DS_TO_COMPANION_MAPPING[s.as_str()] == TaskStatus::DOWNLOADING);

        let t = String::from_str("hash_checking").unwrap();
        assert!(DS_TO_COMPANION_MAPPING[t.as_str()] == TaskStatus::SUBMITTED);
    }

    #[test]
    fn destination_folder_set_in_url() {
        struct DsServiceMock {
            request: RefCell<Request>,
        }

        impl HTTPService for DsServiceMock {
            fn new() -> Self {
                let request: RefCell<Request> = RefCell::new(Request::new(
                    Method::GET,
                    Url::parse("http://nowhere").unwrap(),
                )); // inject here
                Self { request }
            }
            fn send_request(&self, req: Request) -> Option<Value> {
                // copy request in reqs
                self.request.replace_with(|old| req);
                let data = json!({
                    "nothing":
                    "to say"
                });
                return Some(data);
            }
        }

        let controler = DsControler::<DsServiceMock>::new();
        let messaging_controler = DiscordController::<DiscordServiceMock>::new();
        let mut task = Task::new(
            String::from_str("magnet:aaaaa").unwrap(),
            String::from_str("1").unwrap(),
            &messaging_controler,
            Some(String::from_str("videos/Movies").unwrap()),
        );

        controler.submit_task(&mut task);
        assert!(controler
            .service
            .request
            .into_inner()
            .url()
            .as_str()
            .contains("&destination=videos%2FMovies"))
    }
}
