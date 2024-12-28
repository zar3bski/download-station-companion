use crate::conf::CONF;
use crate::structs::{DownloadingService, Task};
use log::{debug, error};
use reqwest::blocking::Client;
use serde::Deserialize;
use serde_json::Value;

//https://global.download.synology.com/download/Document/Software/DeveloperGuide/Package/DownloadStation/All/enu/Synology_Download_Station_Web_API.pdf

#[derive(Deserialize)]
struct InfoResponse {
    data: ApiInformation,
    success: bool,
}

#[derive(Deserialize)]
struct SynoApiAuth {
    path: String,
    minVersion: usize,
    maxVersion: usize,
}

#[derive(Deserialize)]
struct SynoDownloadStationTask {
    path: String,
    minVersion: usize,
    maxVersion: usize,
}

#[derive(Deserialize)]
struct ApiInformation {
    #[serde(rename = "SYNO.API.Auth")]
    auth: SynoApiAuth,
    #[serde(rename = "SYNO.DownloadStation.Task")]
    task: SynoDownloadStationTask,
}

pub struct DownloadStation {
    client: Client,
    api_information: ApiInformation,
}

impl DownloadingService for DownloadStation {
    fn new() -> Self {
        let client = Client::new();
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
                debug!("Information gathered for Synology API");
                return Self {
                    client: client,
                    api_information: root.data,
                };
            }
            Err(err) => {
                error!("Could not get Download station API information: {err}");
                panic!()
            }
        }
    }
    fn submit_task(&self, task: Task) {}
}
