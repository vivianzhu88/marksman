use std::error::Error;
use chrono::{NaiveDate};
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
    InvalidInput(String),
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
    pub config: Config,
    api_gateway: ResyAPIGateway,
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
            api_gateway: ResyAPIGateway::from_auth(api_key, auth_token),
        }
    }

    pub(crate) fn load_config(&mut self, config: Config) {
        let api_key = config.api_key.clone();
        let auth_token = config.auth_token.clone();

        self.config = config;
        self.api_gateway = ResyAPIGateway::from_auth(api_key, auth_token);
    }

    pub(crate) fn update_auth(&mut self, api_key: String, auth_token: String) {
        let api_key_clone = api_key.clone();
        let auth_token_clone = auth_token.clone();

        self.config.api_key = api_key;
        self.config.auth_token = auth_token;

        self.api_gateway = ResyAPIGateway::from_auth(api_key_clone, auth_token_clone)
    }

    pub(crate) async fn view_venue(&mut self, url: Option<&str>, date: Option<&str>, target_time: Option<&str>) -> ResyResult<(String, Vec<Value>)> {
        if let Some(url) = url {
            let _ = self.load_venue_id_from_url(url).await?;
        }

        if let Some(date) = date {
            let parsed_date = NaiveDate::parse_from_str(date, "%Y-%m-%d")
                .map_err(|_| ResyClientError::InvalidInput("Invalid date format. Please use YYYY-MM-DD.".to_string()))?;
            self.config.date = parsed_date.to_string();
        }

        if let Some(target_time) = target_time {
            if target_time.len() == 4 && target_time.chars().all(|c| c.is_digit(10)) {
                let hours = &target_time[..2].parse::<u32>().unwrap();
                let minutes = &target_time[2..].parse::<u32>().unwrap();
                if *hours < 24 && *minutes < 60 {
                    self.config.target_time = Some(target_time.to_string());
                } else {
                    return Err(ResyClientError::InvalidInput("Invalid time format. Please use HHMM format, where HH is 00 to 23 and MM is 00 to 59.".to_string()));
                }
            } else {
                return Err(ResyClientError::InvalidInput("Invalid time format. Please use HHMM format, where HH is 00 to 23 and MM is 00 to 59.".to_string()));
            }
        }

        let slots = self.find_reservation_slots().await?;

        let venue_id = self.config.venue_id.clone();
        Ok((venue_id, slots))
    }

    pub(crate) async fn run_snipe(&mut self) -> ResyResult<String> {


        Ok("Placeholder fuck errors".to_string())
    }

    async fn load_venue_id_from_url(&mut self, url: &str) -> ResyResult<u64> {
        let venue_slug = extract_venue_slug(url)?;

        match self.api_gateway.get_venue(venue_slug.as_str()).await {
            Ok(venue_info) => {
                if let Some(venue_id) = venue_info["id"]["resy"].as_u64() {
                    self.config.venue_id = venue_id.to_string();

                    Ok(venue_id)
                } else {
                    Err(ResyClientError::NotFound("Venue ID not found".to_string()))
                }
            }
            Err(e) => {
                Err(ResyClientError::ApiError(format!("Error fetching venue: {:?}", e)))
            }
        }
    }

    async fn find_reservation_slots(&mut self) -> ResyResult<Vec<Value>> {
        match self.api_gateway.find_reservation(self.config.venue_id.as_str(), self.config.date.as_str(), self.config.party_size, self.config.target_time.as_deref()).await {
            Ok(json) => {
                if let Some(slot_info) = json["results"]["venues"][0]["slots"].as_array() {
                    let mut summarized = Vec::new();
                    for slot in slot_info {
                        if let (
                            Some(config),
                            Some(date),
                            Some(size),
                            Some(quantity)
                            // Some(payment)
                        ) = (
                            slot["config"].as_object(),
                            slot["date"].as_object(),
                            slot["size"].as_object(),
                            slot["quantity"].as_u64(),
                            // slot["payment"].as_object()
                        ) {
                            if let (
                                Some(id),
                                Some(token),
                                Some(slot_type),
                                Some(start),
                                Some(end),
                                Some(min_size),
                                Some(max_size)
                            ) = (
                                config.get("id"),
                                config.get("token"),
                                config.get("type"),
                                date.get("start"), // format: "2024-05-28 13:00:00"
                                date.get("end"),
                                size.get("min"),
                                size.get("max")
                            ) {
                                summarized.push(json!({
                                    "id": id,
                                    "token": token,
                                    "type": slot_type,
                                    "start": start,
                                    "end": end,
                                    "min_size": min_size,
                                    "max_size": max_size,
                                    "quantity": quantity,
                                }));
                            }
                        }
                    }
                    Ok(summarized)
                } else {
                    Ok(Vec::new())
                }
            }
            Err(e) => {
                Err(ResyClientError::ApiError(format!("Error fetching venue: {:?}", e)))
            }
        }
    }
}


fn extract_venue_slug(url: &str) -> ResyResult<String> {
    if let Some(start) = url.find("venues/") {
        let start = start + "venues/".len();
        let end = url[start..].find('?').unwrap_or_else(|| url[start..].len());
        return Ok(url[start..start + end].to_string());
    }
    Err(ResyClientError::InvalidInput("invalid resy url".to_string()))
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