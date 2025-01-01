use std::str::FromStr;

use crate::conf::CONF;
use crate::structs::{DownloadingService, Task, TaskStatus, API_USER_AGENT};
use log::{debug, error};
use reqwest::blocking::Client;
use reqwest::header;
use serde::Deserialize;
use serde_json::{json, Value};

//https://global.download.synology.com/download/Document/Software/DeveloperGuide/Package/DownloadStation/All/enu/Synology_Download_Station_Web_API.pdf

#[derive(Deserialize)]
struct InfoResponse {
    data: ApiInformation,
    success: bool,
}

#[derive(Deserialize, Debug)]
struct SynoApiAuth {
    path: String,
    minVersion: usize,
    maxVersion: usize,
}

#[derive(Deserialize, Debug)]
struct SynoDownloadStationTask {
    path: String,
    minVersion: usize,
    maxVersion: usize,
}

#[derive(Deserialize, Debug)]
struct ApiInformation {
    #[serde(rename = "SYNO.API.Auth")]
    auth: SynoApiAuth,
    #[serde(rename = "SYNO.DownloadStation.Task")]
    task: SynoDownloadStationTask,
}

pub struct DownloadStation {
    client: Client,
    api_information: ApiInformation,
    sid: String,
}

impl DownloadingService for DownloadStation {
    fn new() -> Self {
        let client = Client::new();
        let mut request_headers = header::HeaderMap::new();
        request_headers.insert(
            header::USER_AGENT,
            header::HeaderValue::from_static(API_USER_AGENT),
        );
        let resp = client
            .get(format!(
                "{}/webapi/query.cgi?api=SYNO.API.Info&version=1&method=query&quer
y=SYNO.API.Auth,SYNO.DownloadStation.Task",
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
                        "{}/webapi/{}?api=SYNO.API.Auth&version={}&method=login&accou
nt={}&passwd={}&session=DownloadStation&format=sid",
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
                    debug!("sid: {:?}", sid);
                    return Self {
                        client: client,
                        api_information: api_information,
                        sid,
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

    fn submit_task(&self, task: Task) {
        let resp = self
            .client
            .get(format!(
                "{}/webapi/{}?api=SYNO.DownloadStation.Task&version=1&session=DownloadStation&method=create&_sid={}&uri={}",
                CONF.synology_root_api, &self.api_information.task.path, &self.sid, urlencoding::encode(task.magnet_link.as_str())//trouver le moyen de l'échapper
            ))
            .send();
        task.set_status(TaskStatus::SUBMITTED);
        match resp {
            Ok(res) => {
                debug!("Task submitted successfully: {}", res.text().unwrap());
                //TODO: parse response
            }
            Err(err) => {
                error!("Could not submit download task: {err}");
            }
        }
    }

    fn drop(&self) {
        let _ = &self
            .client
            .get(format!(
                "{}/webapi/{}?api=SYNO.API.Auth&version={}&method=logout&session=DownloadStation",
                CONF.synology_root_api,
                &self.api_information.auth.path,
                &self.api_information.auth.maxVersion
            ))
            .send();
        debug!("Closed session for DownloadStation");
    }
}
