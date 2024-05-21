use std::error::Error;
use prettytable::{row, Table};
use prettytable::cell::Cell;
use prettytable::row::Row;
use reqwest::Client;
use reqwest::header::{AUTHORIZATION, HeaderMap, HeaderValue};
use serde_json::{json, Value};

const RESY_API_BASE_URL: &str = "https://api.resy.com";


// Define Resy API Error
#[derive(Debug)]
pub struct ResyAPIError {
    pub message: String,
}

impl std::fmt::Display for ResyAPIError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for ResyAPIError {}

impl From<std::io::Error> for ResyAPIError {
    fn from(error: std::io::Error) -> Self {
        ResyAPIError {
            message: error.to_string(),
        }
    }
}

// Resy API Gateway
pub struct ResyAPIGateway {
    client: Client,
    api_key: String,
    auth_token: String,
}

impl ResyAPIGateway {
    pub fn new(api_key: String, auth_token: String) -> Self {
        ResyAPIGateway {
            client: Client::new(),
            api_key,
            auth_token,
        }
    }

    pub async fn get_user(&self) -> Result<Value, Box<dyn Error>> {
        let url = format!("{}/2/user", RESY_API_BASE_URL);

        let mut headers = HeaderMap::new();
        headers.insert(AUTHORIZATION, HeaderValue::from_str(&format!("ResyAPI api_key=\"{}\"", self.api_key)).unwrap());
        headers.insert("x-resy-auth-token", HeaderValue::from_str(&self.auth_token).unwrap());

        let res = self.client.get(url)
            .headers(headers)
            .send()
            .await?;

        if res.status().is_success() {
            let response = res.json().await?;
            Ok(response)
        } else {
            Err(Box::new(ResyAPIError {
                message: format!("Failed to fetch user: {}", res.status())
            }))
        }
    }

    pub async fn get_venue(&self, venue_slug: &str) -> Result<Value, Box<dyn Error>> {
        let url = format!("{}/3/venue?url_slug={}&location=new-york-ny", RESY_API_BASE_URL, venue_slug);

        let mut headers = HeaderMap::new();
        headers.insert(AUTHORIZATION, HeaderValue::from_str(&format!("ResyAPI api_key=\"{}\"", self.api_key)).unwrap());
        headers.insert("x-resy-auth-token", HeaderValue::from_str(&self.auth_token).unwrap());

        let res = self.client.get(url)
            .headers(headers)
            .send()
            .await?;

        if res.status().is_success() {
            let response = res.json().await?;
            Ok(response)
        } else {
            Err(Box::new(ResyAPIError {
                message: format!("Failed to fetch venue: {}", res.status())
            }))
        }
    }

    pub async fn find_reservation(&self, venue_id: &str, day: &str, party_size: u8) -> Result<Value, Box<dyn Error>> {
        let url = format!("{}/4/find?lat=0&long=0&day={}&party_size={}&venue_id={}", RESY_API_BASE_URL, day, party_size, venue_id);

        let mut headers = HeaderMap::new();
        headers.insert(AUTHORIZATION, HeaderValue::from_str(&format!("ResyAPI api_key=\"{}\"", self.api_key))?);
        headers.insert("x-resy-auth-token", HeaderValue::from_str(&self.auth_token)?);

        let res = self.client.get(url)
            .headers(headers)
            .send()
            .await?;

        if res.status().is_success() {
            let response = res.json().await?;
            Ok(response)
        } else {
            Err(Box::new(ResyAPIError {
                message: format!("Failed to find reservation slots: {}", res.status())
            }))
        }
    }

    pub async fn get_reservation_details(
        &self,
        commit: u8, // 0 for dry run, 1 for token gen
        config_id: &str,
        party_size: u8,
        day: &str,
    ) -> Result<String, Box<dyn Error>> {
        let url = format!("{}/3/details", RESY_API_BASE_URL);

        let mut headers = HeaderMap::new();
        headers.insert(AUTHORIZATION, HeaderValue::from_str(&format!("ResyAPI api_key=\"{}\"", self.api_key))?);
        headers.insert("x-resy-auth-token", HeaderValue::from_str(&self.auth_token)?);

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

        if res.status().is_success() {
            let response = res.json().await?;
            Ok(response)
        } else {
            Err(Box::new(ResyAPIError {
                message: format!("Failed to find reservation details: {}", res.status())
            }))
        }
    }
}