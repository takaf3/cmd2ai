use crate::api::RequestBody;
use crate::error::Result;
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE};

pub async fn make_api_request(
    api_key: &str,
    api_endpoint: &str,
    request_body: &RequestBody,
) -> Result<reqwest::Response> {
    let mut headers = HeaderMap::new();
    headers.insert(
        AUTHORIZATION,
        HeaderValue::from_str(&format!("Bearer {}", api_key))
            .map_err(|e| crate::error::Cmd2AiError::Other(format!("Invalid authorization header: {}", e)))?,
    );
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));

    let client = reqwest::Client::builder()
        .default_headers(headers)
        .build()?;

    let response = client.post(api_endpoint).json(&request_body).send().await?;
    Ok(response)
}

