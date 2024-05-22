use std::error::Error;
use chrono::{Duration, Utc};
use reqwest::header::{HeaderMap, HeaderValue, CONTENT_TYPE, AUTHORIZATION};
use serde_json::{json, Value};
use prettytable::{row, cell, Table};
use prettytable::row::Row;
use prettytable::cell::Cell;
use crate::config::Config;
use crate::resy_api_gateway::ResyAPIGateway;


#[derive(Debug)]
pub enum ResyClientError {
    NotFound(String),
    NetworkError(String),
    ApiError(String),
    InternalError(String),
}

impl std::fmt::Display for ResyClientError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl Error for ResyClientError {}

type ResyResult<T> = Result<T, ResyClientError>;

#[derive(Debug)]
pub struct ResyClient {
    config: Config,
    api_gateway: ResyAPIGateway
}

impl ResyClient {
    pub(crate) fn new() -> Self {
        ResyClient {
            config: Config::default(),
            api_gateway: ResyAPIGateway::new(),
        }
    }

    pub(crate) fn from_config(config: Config) -> Self {
        let api_key = config.api_key.clone();
        let auth_token = config.auth_token.clone();

        ResyClient {
            config,
            api_gateway: ResyAPIGateway::from_auth(api_key, auth_token)
        }
    }

    pub(crate) fn update_auth(&mut self, api_key: String, auth_token: String) {
        let api_key_clone = api_key.clone();
        let auth_token_clone = auth_token.clone();

        self.config.api_key = api_key;
        self.config.auth_token = auth_token;

        self.api_gateway = ResyAPIGateway::from_auth(api_key_clone, auth_token_clone)
    }

    pub(crate) async fn get_venue_info(&mut self, url: Option<&str>) {
        if let Some(url) = url {
            self.load_venue_id_from_url(url).await.expect("TODO: panic message");
        }

        let day = "2024-06-04";
        match find_reservation_slots(&self.user_auth.api_key, &self.venue_id, &self.user_auth.auth_token, &day, 6).await {
            Ok(slots) => {},
            Err(e) => eprintln!("Error: {}", e),
        }
    }

    async fn load_venue_id_from_url(&mut self, url: &str) -> ResyResult<str>{
        let venue_slug = extract_venue_slug(url);

        match self.api_gateway.get_venue(venue_slug.as_str()).await {
            Ok(venue_info) => {
                if let Some(venue_id) = venue_info["id"]["resy"].as_u8() {
                    self.config.venue_id = venue_id.to_string();

                    Ok(venue_id)
                } else {
                    Err(ResyClientError::NotFound("Venue ID not found".to_string()))
                }
            },
            Err(e) => {
                Err(ResyClientError::ApiError(format!("Error fetching venue: {:?}", e)))
            }
        }
    }
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

pub async fn find_reservation_slots(api_key: &str, venue_id: &str, auth_token: &str, day: &str, party_size: u8) -> Result<Vec<Value>, Box<dyn Error>> {
    let client = reqwest::Client::new();
    let url = format!("https://api.resy.com/4/find?lat=0&long=0&day={}&party_size={}&venue_id={}", day, party_size, venue_id);

    let mut headers = HeaderMap::new();
    headers.insert(AUTHORIZATION, HeaderValue::from_str(&format!("ResyAPI api_key=\"{}\"", api_key))?);
    headers.insert("x-resy-auth-token", HeaderValue::from_str(auth_token)?);

    let res = client.get(url)
        .headers(headers)
        .send()
        .await?;

    if res.status().is_success() {
        let body = res.text().await?;
        let json: Value = serde_json::from_str(&body)?;
        if let Some(slot_info) = json["results"]["venues"][0]["slots"].as_array() {
            let mut summarized = Vec::new();

            for slot in slot_info {
                if let (Some(config), Some(date)) = (slot["config"].as_object(), slot["date"].as_object()) {
                    if let (Some(id), Some(token), Some(slot_type), Some(start), Some(end)) = (
                        config.get("id"),
                        config.get("token"),
                        config.get("type"),
                        date.get("start"),
                        date.get("end")
                    ) {
                        summarized.push(json!({
                    "id": id,
                    "token": token,
                    "type": slot_type,
                    "start": start,
                    "end": end
                }));
                    }
                }
            }

            print_table(&summarized);
            return Ok(summarized);
        } else {
            return Ok(Vec::new());
        }
    } else {
        println!("Failed to fetch reservations: {}", res.status());
        Err(Box::new(std::io::Error::new(std::io::ErrorKind::Other, "Failed to fetch reservations")))
    }
}

fn print_table(slots: &[Value]) {
    let mut table = Table::new();

    table.add_row(row!["Type", "Start", "End", "ID", "Token"]);

    for slot in slots {
        if let (Some(slot_type), Some(start), Some(end), Some(id), Some(token)) = (
            slot.get("type"),
            slot.get("start"),
            slot.get("end"),
            slot.get("id"),
            slot.get("token"),
        ) {
            let id_str = if id.is_number() {
                id.to_string()
            } else {
                id.as_str().unwrap_or("").to_string()
            };

            table.add_row(Row::new(vec![
                Cell::new(slot_type.as_str().unwrap_or("")),
                Cell::new(start.as_str().unwrap_or("")),
                Cell::new(end.as_str().unwrap_or("")),
                Cell::new(&id_str),
                Cell::new(token.as_str().unwrap_or("")),
            ]));
        }
    }

    table.printstd();
}