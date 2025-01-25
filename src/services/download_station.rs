use std::str::FromStr;

use crate::conf::CONF;
use crate::services::schemas::{InfoResponse, DS_ERROR_CODES};
use crate::services::API_CONTENT_TYPE;
use crate::task::{Source, Task, TaskStatus};
use crate::traits::{DownloadingController, HTTPService, Payload};
use bytes::Bytes;
use log::{debug, error, warn};

use reqwest::blocking::Client;
use reqwest::header::{self, HeaderValue, ACCEPT, USER_AGENT};
use reqwest::{Method, Url};
use serde_json::Value;

use super::schemas::{ApiInformation, DS_TO_COMPANION_MAPPING};
use super::API_USER_AGENT;

//https://global.download.synology.com/download/Document/Software/DeveloperGuide/Package/DsControler/All/enu/Synology_Download_Station_Web_API.pdf

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

    fn download_file(&self, _: Url) -> Option<Bytes> {
        panic!("Not implemented")
    }

    fn send_request(
        &self,
        mut url: Url,
        method: Method,
        payload: Option<Payload>,
    ) -> Option<Value> {
        // extract query from url and override req.url with the proper
        // API root_url and the sid
        url.query_pairs_mut().append_pair("_sid", &self.sid);

        let mut final_url = self.root_url.clone();
        final_url.set_query(url.query());

        let url_log = url.clone();
        let req = match payload {
            Some(payload) => match payload {
                Payload::BODY(body) => self.client.request(method, final_url).body(body),
                Payload::FORM(form) => self
                    .client
                    .request(method, final_url)
                    .multipart(form)
                    .header(ACCEPT, HeaderValue::from_static(&API_CONTENT_TYPE)),
            },
            None => self.client.request(method, final_url),
        };

        debug!("Request: {:?}", req);
        let resp = req.send();

        match resp {
            Ok(resp) => {
                let status = resp.status().as_u16();
                if status < 300 {
                    return Some(resp.json().unwrap());
                } else {
                    warn!("Status code {} received from {}", status, url_log);
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
        let url = Url::parse(format!(
            "{}?api=SYNO.DownloadStation.Task&version=1&session=DownloadStation&method=list&additional=detail&username={}",
            CONF.synology_root_api, CONF.synology_user
        ).as_str()).unwrap();
        let resp = self.service.send_request(url, Method::GET, None).unwrap();

        if resp["success"] == true {
            let distant_tasks: &Vec<Value> = resp["data"]["tasks"].as_array().unwrap();
            for obj in distant_tasks {
                let uri = obj["additional"]["detail"]["uri"].as_str().unwrap();
                let status = obj["status"].as_str().unwrap();
                for task_idx in 0..tasks.len() {
                    let task = &mut tasks[task_idx];
                    match &task.source {
                        Source::MAGNET(magnet_link) => {
                            if magnet_link == uri
                                && DS_TO_COMPANION_MAPPING[status] != task.get_status()
                            {
                                let s: TaskStatus = DS_TO_COMPANION_MAPPING[status];
                                task.set_status(s);
                                if s == TaskStatus::DONE || s == TaskStatus::FAILED {
                                    tasks.remove(task_idx); // TODO: call deleter from set_status instead?
                                }
                            } else {
                                debug!("Nothing new for task: {}", magnet_link);
                            }
                        }
                        Source::FILE(_) => {
                            panic!("Not implemented")
                        }
                    }
                }
            }
        } else {
            error!("Could not withdraw job status: {resp}")
        }
    }

    fn submit_task(&self, task: &mut Task) {
        let mut url = Url::parse(
            format!(
                "{}?api=SYNO.DownloadStation.Task&version=1&session=DownloadStation&method=create",
                CONF.synology_root_api
            )
            .as_str(),
        )
        .unwrap();
        if task.destination_folder.is_some() {
            url.query_pairs_mut().append_pair(
                "destination",
                task.destination_folder.clone().unwrap().as_str(),
            );
        }

        let resp = match &task.source {
            Source::MAGNET(magnet_link) => {
                url.query_pairs_mut().append_pair("uri", &magnet_link);

                let resp = self.service.send_request(url, Method::GET, None);
                resp
            }
            Source::FILE(_) => {
                // TODO: implement when missing documentation is fixed
                //let part = Part::bytes(file.to_vec())
                //    .file_name("bio.torrent") // TODO: make variable
                //    .mime_str("application/x-bittorrent")
                //    .unwrap();
                //let form = reqwest::blocking::multipart::Form::new().part("file", part);
                //let resp = self
                //    .service
                //    .send_request(url, Method::POST, Some(Payload::FORM(form)));
                //resp
                panic!("Not implemented")
            }
        };
        match resp {
            Some(resp) => {
                let data = resp.as_object().unwrap();
                if data.contains_key("error") {
                    error!(
                        "Could not submit download task: {}. Error code {}: {}",
                        task.message_id,
                        data["error"]["code"],
                        DS_ERROR_CODES[&(data["error"]["code"].as_u64().unwrap() as u8)]
                    );
                    task.set_status(TaskStatus::FAILED);
                } else {
                    debug!("Task submitted successfully: {:?}", data);
                    task.set_status(TaskStatus::SUBMITTED);
                }
            }
            None => {
                error!(
                    "Could not submit download task: {}. No response from API",
                    task.message_id
                );
            }
        }
    }
}

#[cfg(test)]
pub mod tests {

    use std::{cell::RefCell, str::FromStr, sync::Mutex};

    use bytes::Bytes;
    use reqwest::{blocking::Body, Method, Url};
    use serde_json::{json, Value};

    use crate::{
        services::{
            discord::DiscordController,
            download_station::{DsControler, DS_TO_COMPANION_MAPPING},
        },
        task::{Source, Task, TaskStatus},
        traits::{DownloadingController, HTTPService, MessagingController, Payload},
    };

    struct DiscordServiceMock {}
    impl HTTPService for DiscordServiceMock {
        fn new() -> Self {
            Self {}
        }
        fn send_request(&self, _: Url, _: Method, _: Option<Payload>) -> Option<Value> {
            return Some(json!({}));
        }
        fn download_file(&self, _: Url) -> Option<Bytes> {
            panic!("Not implemented")
        }
    }

    struct DsServiceMock {
        payload: RefCell<Option<Payload>>,
        url: RefCell<Url>,
    }

    impl HTTPService for DsServiceMock {
        fn new() -> Self {
            let payload: RefCell<Option<Payload>> =
                RefCell::new(Some(Payload::BODY(Body::from(vec![])))); // inject here
            let url = RefCell::new(Url::parse("http://somewhere").unwrap());
            Self { payload, url }
        }
        fn send_request(&self, url: Url, _: Method, payload: Option<Payload>) -> Option<Value> {
            // copy request in reqs
            self.payload.replace_with(|old| payload);
            self.url.replace_with(|old| url);
            let data = json!({
                "nothing":
                "to say"
            });
            return Some(data);
        }
        fn download_file(&self, _: Url) -> Option<Bytes> {
            panic!("Not implemented")
        }
    }

    #[test]
    fn status_mapping() {
        let s = String::from_str("downloading").unwrap();
        assert!(DS_TO_COMPANION_MAPPING[s.as_str()] == TaskStatus::DOWNLOADING);

        let t = String::from_str("hash_checking").unwrap();
        assert!(DS_TO_COMPANION_MAPPING[t.as_str()] == TaskStatus::SUBMITTED);
    }

    //
    //#[test]
    //fn file_handling() {
    //    let controler = DsControler::<DsServiceMock>::new();
    //    let messaging_controler = DiscordController::<DiscordServiceMock>::new();
    //    let file = Bytes::from("SOME_FILE");
    //    let mut task = Task::new(
    //        Source::FILE(file),
    //        String::from_str("1").unwrap(),
    //        &messaging_controler,
    //        Some(String::from_str("videos/Movies").unwrap()),
    //        String::from_str("1").unwrap(),
    //    );
    //    controler.submit_task(&mut task);
    //    let receive_req = controler.service.payload.into_inner().unwrap();
    //    // HERE: test
    //}

    #[test]
    fn destination_folder_set_in_url() {
        let controler = DsControler::<DsServiceMock>::new();
        let messaging_controler = DiscordController::<DiscordServiceMock>::new();
        let mut task = Task::new(
            Source::MAGNET(
                String::from_str("magnet:?xt=urn:btih:A3057BB12D25F9F391806D819A9420FA29A86712&")
                    .unwrap(),
            ),
            String::from_str("1").unwrap(),
            &messaging_controler,
            Some(String::from_str("videos/Movies").unwrap()),
            String::from_str("1").unwrap(),
        );

        controler.submit_task(&mut task);
        let url_str = controler.service.url.into_inner();
        assert!(url_str.as_str().contains("&destination=videos%2FMovies"));
        assert!(url_str.as_str().contains(
            "&uri=magnet%3A%3Fxt%3Durn%3Abtih%3AA3057BB12D25F9F391806D819A9420FA29A86712%2"
        ))
    }

    #[test]
    fn advancement_updating() {
        struct DsServiceMock {
            time_called: Mutex<i8>,
        }

        impl HTTPService for DsServiceMock {
            fn new() -> Self {
                let time_called: Mutex<i8> = Mutex::new(0);
                Self { time_called }
            }
            fn send_request(
                &self,
                url: Url,
                method: Method,
                payload: Option<Payload>,
            ) -> Option<Value> {
                // copy request in reqs
                let mut value = self.time_called.lock().unwrap();
                *value += 1;
                match *value {
                    1 => return Some(json!({"success": false})),
                    2 => {
                        return Some(
                            json!({"success": true, "data":{"tasks":[{"status":"downloading", "additional":{"detail":{"uri":"magnet:?xt9420FA29A"}}}]}}),
                        )
                    }
                    3 => {
                        return Some(
                            json!({"success": true, "data":{"tasks":[{"status":"downloading", "additional":{"detail":{"uri":"magnet:?xt9420FA29A"}}}]}}),
                        )
                    }
                    4 => {
                        return Some(
                            json!({"success": true, "data":{"tasks":[{"status":"finished", "additional":{"detail":{"uri":"magnet:?xt9420FA29A"}}}]}}),
                        )
                    }
                    _ => None,
                }
            }
            fn download_file(&self, _: Url) -> Option<Bytes> {
                panic!("Not implemented")
            }
        }
        let controler = DsControler::<DsServiceMock>::new();
        let messaging_controler = DiscordController::<DiscordServiceMock>::new();
        let task = Task::new(
            Source::MAGNET(String::from_str("magnet:?xt9420FA29A").unwrap()),
            String::from_str("1").unwrap(),
            &messaging_controler,
            Some(String::from_str("videos/Movies").unwrap()),
            String::from_str("1").unwrap(),
        );
        let mut tasks = vec![task];
        matches!(tasks[0].get_status(), TaskStatus::RECEIVED);
        controler.get_jobs_advancement(&mut tasks);
        matches!(tasks[0].get_status(), TaskStatus::RECEIVED);
        controler.get_jobs_advancement(&mut tasks);
        matches!(tasks[0].get_status(), TaskStatus::DOWNLOADING);
        controler.get_jobs_advancement(&mut tasks);
        matches!(tasks[0].get_status(), TaskStatus::DONE);
    }

    // TODO: ERROR TESTING
}
