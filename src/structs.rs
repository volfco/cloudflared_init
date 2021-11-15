use std::collections::HashMap;
use serde::{Deserialize, Serialize};

#[derive(structopt::StructOpt, Debug, Clone)]
pub struct Args {
    /// Service Name to use
    #[structopt(long = "service")]
    pub service_name: String,

    /// URL to send traffic to. Passed to cloudflared directly
    #[structopt(long = "target")]
    pub target_url: String,

    /// Wait for the target URL to become healthy before bringing up the tunnel
    #[structopt(long = "delay", default_value = "0")]
    pub health_delay: u32,


}

#[derive(Debug)]
pub struct TunnelConfig {
    pub tunnel_name: String,
    pub target_lb: String,
    pub target_pool: String,
    pub url: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CloudflaredConfig {
    #[serde(rename = "AccountTag")]
    account_tag: String,
    #[serde(rename = "TunnelSecret")]
    tunnel_secret: String,
    #[serde(rename = "TunnelId")]
    tunnel_id: String,
    #[serde(rename = "TunnelName")]
    tunnel_name: String
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EcsTask {
    #[serde(rename = "Cluster")]
    pub cluster: String,
    #[serde(rename = "TaskARN")]
    pub task_arn: String,
    #[serde(rename = "Family")]
    pub family: String,
    #[serde(rename = "Revision")]
    pub revision: String,
    #[serde(rename = "DesiredStatus")]
    pub desired_status: String,
    #[serde(rename = "KnownStatus")]
    pub known_status: String,
    #[serde(rename = "Containers")]
    pub containers: Vec<Container>,
    #[serde(rename = "Limits")]
    pub limits: Limits2,
    #[serde(rename = "PullStartedAt")]
    pub pull_started_at: String,
    #[serde(rename = "PullStoppedAt")]
    pub pull_stopped_at: String,
    #[serde(rename = "AvailabilityZone")]
    pub availability_zone: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Container {
    #[serde(rename = "DockerId")]
    pub docker_id: String,
    #[serde(rename = "Name")]
    pub name: String,
    #[serde(rename = "DockerName")]
    pub docker_name: String,
    #[serde(rename = "Image")]
    pub image: String,
    #[serde(rename = "ImageID")]
    pub image_id: String,
    #[serde(rename = "Labels")]
    pub labels: HashMap<String, String>,
    #[serde(rename = "DesiredStatus")]
    pub desired_status: String,
    #[serde(rename = "KnownStatus")]
    pub known_status: String,
    #[serde(rename = "Limits")]
    pub limits: Limits,
    #[serde(rename = "CreatedAt")]
    pub created_at: String,
    #[serde(rename = "StartedAt")]
    pub started_at: String,
    #[serde(rename = "Type")]
    pub type_field: String,
    #[serde(rename = "Networks")]
    pub networks: Vec<Network>,
    #[serde(rename = "Health")]
    pub health: Option<Health>,
    #[serde(rename = "Volumes")]
    #[serde(default)]
    pub volumes: Vec<Volume>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Limits {
    #[serde(rename = "CPU")]
    pub cpu: i64,
    #[serde(rename = "Memory")]
    pub memory: Option<i64>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Network {
    #[serde(rename = "NetworkMode")]
    pub network_mode: String,
    #[serde(rename = "IPv4Addresses")]
    pub ipv4addresses: Vec<String>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Health {
    pub status: String,
    pub status_since: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Volume {
    #[serde(rename = "DockerName")]
    pub docker_name: String,
    #[serde(rename = "Destination")]
    pub destination: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Limits2 {
    #[serde(rename = "CPU")]
    pub cpu: i64,
    #[serde(rename = "Memory")]
    pub memory: i64,
}

