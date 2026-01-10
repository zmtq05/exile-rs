use anyhow::Context;
use serde::Deserialize;

pub const PROXY_URL: &str = "https://exile-rs-proxy.zmtq05.workers.dev";

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProxyResponse {
    pub id: String,
    pub name: String,
    pub size: u64,
    pub mime_type: String,
    pub modified_time: String,
}

pub async fn fetch_remote_pob(client: reqwest::Client) -> anyhow::Result<ProxyResponse> {
    let resp = client
        .get(PROXY_URL)
        .send()
        .await
        .context("Failed to send request to proxy URL")?;

    let status = resp.status();
    if !status.is_success() {
        anyhow::bail!("Proxy server returned error status: {}", status);
    }

    let json = resp
        .json::<ProxyResponse>()
        .await
        .context("Failed to parse JSON response from proxy server")?;

    Ok(json)
}

pub async fn download_file(client: reqwest::Client, file_id: &str, file_name: &str) -> anyhow::Result<reqwest::Response> {
    let url = format!("{}/download/{}?name={}", PROXY_URL, file_id, file_name);
    let resp = client
        .get(&url)
        .send()
        .await
        .context("Failed to send request to download URL")?;

    let status = resp.status();
    if !status.is_success() {
        anyhow::bail!("Download request returned error status: {}", status);
    }

    Ok(resp)
}