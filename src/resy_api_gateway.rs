use std::error::Error;
use reqwest::{Client, Response};
use reqwest::header::{ACCEPT, ACCEPT_LANGUAGE, AUTHORIZATION, CONTENT_TYPE, HeaderMap, HeaderValue};
use serde_json::{json, Value};

const RESY_API_BASE_URL: &str = "https://api.resy.com";

/// Error type for Resy API specific errors.
#[derive(Debug)]
pub struct ResyAPIError {
    pub message: String,
}

impl std::fmt::Display for ResyAPIError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl Error for ResyAPIError {}

impl From<std::io::Error> for ResyAPIError {
    fn from(error: std::io::Error) -> Self {
        ResyAPIError {
            message: error.to_string(),
        }
    }
}

/// Handles communication with the Resy API.
#[derive(Debug)]
pub struct ResyAPIGateway {
    client: Client,
    api_key: String,
    auth_token: String,
}

impl ResyAPIGateway {

    /// Creates a new API gateway instance (without authentication)
    pub fn new() -> Self {
        ResyAPIGateway {
            client: Client::new(),
            api_key: String::new(),
            auth_token: String::new(),
        }
    }

    /// Creates a new API gateway instance with authentication.
    pub fn from_auth(api_key: String, auth_token: String) -> Self {
        ResyAPIGateway {
            client: Client::new(),
            api_key,
            auth_token,
        }
    }

    /// Processes the HTTP response, converting JSON or returning an error.
    async fn process_response(response: Response) -> Result<Value, Box<dyn Error>> {
        if response.status().is_success() {
            let json = response.json().await?;
            Ok(json)
        } else {
            Err(Box::new(ResyAPIError {
                message: format!("API request failed: {}", response.status())
            }))
        }
    }

    /// Sets up the necessary auth headers for making requests to the Resy API.
    fn setup_headers(&self) -> HeaderMap {
        let mut headers = HeaderMap::new();

        // ??
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        headers.insert(ACCEPT, HeaderValue::from_static("application/json, text/plain, */*"));
        headers.insert(ACCEPT_LANGUAGE, HeaderValue::from_static("en-US,en;q=0.9"));

        // auth
        headers.insert(AUTHORIZATION, HeaderValue::from_str(&format!("ResyAPI api_key=\"{}\"", self.api_key)).unwrap());
        headers.insert("x-resy-auth-token", HeaderValue::from_str(&self.auth_token).unwrap());
        headers.insert("x-resy-universal-auth", HeaderValue::from_str(&self.auth_token).unwrap());

        // Additional headers from curl
        headers.insert("cache-control", HeaderValue::from_static("no-cache"));
        headers.insert("dnt", HeaderValue::from_static("1"));
        headers.insert("origin", HeaderValue::from_static("https://widgets.resy.com"));
        headers.insert("priority", HeaderValue::from_static("u=1, i"));
        headers.insert("referer", HeaderValue::from_static("https://widgets.resy.com/"));
        headers.insert("sec-ch-ua", HeaderValue::from_static("\"Not-A.Brand\";v=\"99\", \"Chromium\";v=\"124\""));
        headers.insert("sec-ch-ua-mobile", HeaderValue::from_static("?0"));
        headers.insert("sec-ch-ua-platform", HeaderValue::from_static("\"macOS\""));
        headers.insert("sec-fetch-dest", HeaderValue::from_static("empty"));
        headers.insert("sec-fetch-mode", HeaderValue::from_static("cors"));
        headers.insert("sec-fetch-site", HeaderValue::from_static("same-site"));
        headers.insert("user-agent", HeaderValue::from_static("Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0.0.0 Safari/537.36"));
        headers.insert("x-origin", HeaderValue::from_static("https://widgets.resy.com"));

        headers
    }

    /// Fetches user details from the Resy API.
    pub async fn get_user(&self) -> Result<Value, Box<dyn Error>> {
        let url = format!("{}/2/user", RESY_API_BASE_URL);
        let headers = self.setup_headers();

        let res = self.client.get(url)
            .headers(headers)
            .send()
            .await?;

        Self::process_response(res).await
    }

    /// Retrieves details about a venue from the Resy API.
    pub async fn get_venue(&self, venue_slug: &str) -> Result<Value, Box<dyn Error>> {
        let url = format!("{}/3/venue?url_slug={}&location=new-york-ny", RESY_API_BASE_URL, venue_slug);
        let headers = self.setup_headers();

        let res = self.client.get(url)
            .headers(headers)
            .send()
            .await?;

        Self::process_response(res).await
    }

    /// Finds reservations at a venue.
    pub async fn find_reservation(&self, venue_id: &str, day: &str, party_size: u8, target_time: Option<&str>) -> Result<Value, Box<dyn Error>> {
        let mut url = format!("{}/4/find?lat=0&long=0&day={}&party_size={}&venue_id={}", RESY_API_BASE_URL, day, party_size, venue_id);

        if let Some(time) = target_time {
            let formatted_time = format!("{}:{}", &time[..2], &time[2..]);
            url = format!("{}&time_filter={}", url, formatted_time);
        }

        let headers = self.setup_headers();

        let res = self.client.get(url)
            .headers(headers)
            .send()
            .await?;

        Self::process_response(res).await
    }

    /// Gets reservation details from the Resy API.
    pub async fn get_reservation_details(
        &self,
        commit: u8, // 0 for dry run, 1 for token gen
        config_id: &str,
        party_size: u8,
        day: &str,
    ) -> Result<Value, Box<dyn Error>> {
        let url = format!("{}/3/details", RESY_API_BASE_URL);
        let headers = self.setup_headers();

        let data = json!({
            "commit": commit,
            "config_id": config_id,
            "day": day,
            "party_size": party_size
        });

        let res = self.client.post(url)
            .headers(headers)
            .json(&data)
            .send()
            .await?;

        Self::process_response(res).await
    }

    fn setup_book_headers(&self) -> HeaderMap {
        let mut headers = HeaderMap::new();

        // Content Type
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/x-www-form-urlencoded"));

        // Accept
        headers.insert(ACCEPT, HeaderValue::from_static("application/json, text/plain, */*"));

        // Accept Language
        headers.insert(ACCEPT_LANGUAGE, HeaderValue::from_static("en-US,en;q=0.9"));

        // Authorization and Token
        headers.insert(AUTHORIZATION, HeaderValue::from_str(&format!("ResyAPI api_key=\"{}\"", self.api_key)).unwrap());
        headers.insert("x-resy-auth-token", HeaderValue::from_str(&self.auth_token).unwrap());
        headers.insert("x-resy-universal-auth", HeaderValue::from_str(&self.auth_token).unwrap());

        // Additional headers from curl
        headers.insert("cache-control", HeaderValue::from_static("no-cache"));
        headers.insert("dnt", HeaderValue::from_static("1"));
        headers.insert("origin", HeaderValue::from_static("https://widgets.resy.com"));
        headers.insert("priority", HeaderValue::from_static("u=1, i"));
        headers.insert("referer", HeaderValue::from_static("https://widgets.resy.com/"));
        headers.insert("sec-ch-ua", HeaderValue::from_static("\"Not-A.Brand\";v=\"99\", \"Chromium\";v=\"124\""));
        headers.insert("sec-ch-ua-mobile", HeaderValue::from_static("?0"));
        headers.insert("sec-ch-ua-platform", HeaderValue::from_static("\"macOS\""));
        headers.insert("sec-fetch-dest", HeaderValue::from_static("empty"));
        headers.insert("sec-fetch-mode", HeaderValue::from_static("cors"));
        headers.insert("sec-fetch-site", HeaderValue::from_static("same-site"));
        headers.insert("user-agent", HeaderValue::from_static("Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0.0.0 Safari/537.36"));
        headers.insert("x-origin", HeaderValue::from_static("https://widgets.resy.com"));

        headers
    }

    /// Books reservation via the Resy API (dry run possible)
    pub async fn book_reservation(&self, book_token: &str, payment_id: &str) -> Result<Value, Box<dyn Error>> {
        let url = format!("{}/3/book", RESY_API_BASE_URL);
        let headers = self.setup_book_headers();

        let body = format!(
            "book_token={}&struct_payment_method={{\"id\":{}}}",
            urlencoding::encode(book_token), payment_id
        );

        let res = self.client.post(&url)
            .headers(headers)
            .body(body)
            .send()
            .await?;

        Self::process_response(res).await
    }
}