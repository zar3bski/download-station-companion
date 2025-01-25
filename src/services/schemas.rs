use once_cell::sync::Lazy;
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::Arc;

use crate::task::TaskStatus;

#[allow(dead_code)]
#[derive(Deserialize)]
pub struct InfoResponse {
    pub data: ApiInformation,
    pub success: bool,
}

#[allow(dead_code)]
#[derive(Deserialize, Debug)]
pub struct SynoApiAuth {
    pub path: String,
    #[serde(rename = "minVersion")]
    pub min_version: usize,
    #[serde(rename = "maxVersion")]
    pub max_version: usize,
}

#[allow(dead_code)]
#[derive(Deserialize, Debug)]
pub struct SynoDsControlerTask {
    pub path: String,
    #[serde(rename = "minVersion")]
    pub min_version: usize,
    #[serde(rename = "maxVersion")]
    pub max_version: usize,
}

#[derive(Deserialize, Debug)]
pub struct ApiInformation {
    #[serde(rename = "SYNO.API.Auth")]
    pub auth: SynoApiAuth,
    #[serde(rename = "SYNO.DownloadStation.Task")]
    pub task: SynoDsControlerTask,
}

#[allow(dead_code)]
#[derive(Deserialize)]
pub struct AttachementObject {
    pub id: String,
    pub filename: String,
    pub url: String,
    pub proxy_url: String,
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

pub static DS_ERROR_CODES: Lazy<Arc<HashMap<u8, &str>>> = Lazy::new(|| {
    let mapping = HashMap::from([
        (100 as u8, "Unknown error"),
        (101 as u8, "Invalid parameter"),
        (102 as u8, "The requested API does not exist"),
        (103 as u8, "The requested method does not exist"),
        (
            104 as u8,
            "The requested version does not support the functionality",
        ),
        (105 as u8, "The logged in session does not have permission"),
        (106 as u8, "Session timeout"),
        (107 as u8, "Session interrupted by duplicate login"),
    ]);
    Arc::new(mapping)
});
