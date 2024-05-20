use std::error::Error;
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE};
use serde_json::Value;
use crate::config::Config;

struct UserAuth {
    api_key: String,
    auth_token: String,
}

pub(crate) struct ResyClient {
    venue_id: String,
    user_auth: UserAuth,
}

impl ResyClient {
    pub(crate) fn new() -> Self {
        ResyClient {
            venue_id: String::new(),
            user_auth: UserAuth {
                api_key: String::new(),
                auth_token: String::new(),
            },
        }
    }

    pub(crate) fn from_config(venue_id: String, api_key: String, auth_token: String) -> Self {
        ResyClient {
            venue_id,
            user_auth: UserAuth {
                api_key,
                auth_token,
            },
        }
    }

    pub(crate) fn load_config(&mut self, config: Config) {
        self.user_auth.auth_token = config.auth_token;
        self.user_auth.api_key = config.api_key;
    }

    pub(crate) async fn get_venue_info(&mut self, url: Option<&str>) {
        if let Some(url) = url {
            self.load_venue_from_url(url).await;
        }
        println!("here");
    }

    async fn load_venue_from_url(&mut self, url: &str) {
        let venue_slug = extract_venue_slug(url);
        println!("venue found: {}!", venue_slug);

        self.venue_id = fetch_venue_id(&venue_slug, &self.user_auth.api_key, &self.user_auth.auth_token).await.expect("error loading venue_id");
    }

    // // Simulates checking reservations
    // fn check_reservations(&self) {
    //     println!("Checking reservations for Venue ID: {}", self.venue_id);
    //     // Implementation to check reservations
    // }
    //
    // // Simulates getting available slots
    // fn get_slots(&self) {
    //     println!("Getting slots for Venue ID: {}", self.venue_id);
    //     // Implementation to get available slots
    // }
}


fn extract_venue_slug(url: &str) -> String {
    if let Some(start) = url.find("venues/") {
        let start = start + "venues/".len();
        let end = url[start..].find('?').unwrap_or_else(|| url[start..].len());
        return url[start..start + end].to_string();
    }
    String::new()
}

// API CALLS
pub async fn fetch_venue_id(venue_slug: &str, api_key: &str, auth_token: &str) -> Result<String, Box<dyn Error>> {
    let client = reqwest::Client::new();
    let url = format!("https://api.resy.com/3/venue?url_slug={}&location=new-york-ny", venue_slug);

    let mut headers = HeaderMap::new();
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
    headers.insert(AUTHORIZATION, HeaderValue::from_str(&format!("ResyAPI api_key=\"{}\"", api_key)).unwrap());
    headers.insert("x-resy-auth-token", HeaderValue::from_str(auth_token).unwrap());
    headers.insert("x-resy-universal-auth", HeaderValue::from_str(auth_token).unwrap());

    let res = client.get(url)
        .headers(headers)
        .send()
        .await?;

    if res.status().is_success() {
        let body = res.text().await?;
        let json: Value = serde_json::from_str(&body)?;
        if let Some(venue_id) = json["id"]["resy"].as_u64() {
            let venue_id_str = venue_id.to_string();
            println!("venue_id found: {}!", venue_id_str);
            return Ok(venue_id_str);
        } else {
            println!("venue ID not found");
        }
    } else {
        println!("failed to fetch venue_id: {}", res.status());
    }

    Ok(String::new())
}