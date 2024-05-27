use std::error::Error;
use std::future::Future;
use chrono::{NaiveDate};
use serde_json::{json, Value};
use prettytable::{row, cell, Table};
use prettytable::row::Row;
use prettytable::cell::Cell;
use tokio::task::JoinSet;
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

    pub(crate) async fn view_venue(&mut self, url: Option<&str>, date: Option<&str>, party_size: Option<u8>, target_time: Option<&str>) -> ResyResult<(String, Vec<Value>)> {
        if let Some(url) = url {
            let _ = self.load_venue_id_from_url(url).await?;
        }

        if let Some(date) = date {
            let parsed_date = NaiveDate::parse_from_str(date, "%Y-%m-%d")
                .map_err(|_| ResyClientError::InvalidInput("Invalid date format. Please use YYYY-MM-DD.".to_string()))?;
            self.config.date = parsed_date.to_string();
        }

        if let Some(party_size) = party_size {
            self.config.party_size = party_size;
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
        } else {
            self.config.target_time = None;
        }

        let slots = self.find_reservation_slots().await?;

        let venue_id = self.config.venue_id.clone();
        Ok((venue_id, slots))
    }

    pub(crate) async fn run_snipe(&self) -> ResyResult<String> {
        if !self.config.validate() {
            return Err(ResyClientError::InvalidInput("reservation config is not complete".to_string()));
        }

        let slots = match self.api_gateway.find_reservation(
            self.config.venue_id.as_str(),
            self.config.date.as_str(),
            self.config.party_size,
            self.config.target_time.as_deref()
        ).await {
            Ok(json) => match format_slots(json) {
                Ok(slots) => slots,
                Err(e) => panic!("Error formatting reservation slots: {:?}", e),
            },
            Err(e) => {
                panic!("Error formatting reservation slots: {:?}", e)
            }
        };

        let mut handles = vec![];

        let mut set = JoinSet::new();

        for slot in slots {
            // Only spawn tasks if the slot has a valid 'config_id'
            if let Some(config_id) = slot["id"].as_str() {
                let handle = tokio::spawn(async move {
                    self._snipe_task(config_id).await
                });
                handles.push(handle);

                set.spawn(async move {
                    self._snipe_task(config_id).await
                });


            } else {
                println!("Skipping slot with missing 'id'");
            }
        }

        let results = join_all(handles).await;

        Ok("Placeholder for compilation".to_string())
    }

    async fn _snipe_task(&self, config_id: &str) -> bool {
        let slots = match self.api_gateway.get_reservation_details(0, config_id, self.config.party_size, &self.config.date).await {
            Ok(json) => {}
            Err(e) => {}
        };

        return true
    }

    pub(crate) async fn get_payment_id(&mut self) -> ResyResult<String> {
        match self.api_gateway.get_user().await {
            Ok(user_data) => {
                let payment_methods = user_data["payment_methods"]
                    .as_array()
                    .ok_or_else(|| ResyClientError::NotFound("No payment method found in resy account".to_string()))?;

                println!("{:?}", payment_methods);
                let payment_id = payment_methods.get(0)
                    .ok_or_else(|| ResyClientError::NotFound("Payment method list is empty".to_string()))?
                    .get("id")
                    .and_then(|id| id.as_i64())
                    .map(|id| id.to_string())
                    .ok_or_else(|| ResyClientError::NotFound("Payment ID not found".to_string()))?;

                self.config.payment_id = payment_id.clone();
                Ok(payment_id)
            }
            Err(e) => {
                Err(ResyClientError::ApiError(format!("Error fetching payment_id: {:?}", e)))
            }
        }
    }

    async fn load_venue_id_from_url(&mut self, url: &str) -> ResyResult<u64> {
        let venue_slug = extract_venue_slug(url)?;
        self.config.venue_slug = venue_slug.clone();

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

    async fn find_reservation_slots(&self) -> ResyResult<Vec<Value>> {
        match self.api_gateway.find_reservation(self.config.venue_id.as_str(), self.config.date.as_str(), self.config.party_size, self.config.target_time.as_deref()).await {
            Ok(json) => format_slots(json),
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

fn format_slots(json: Value) -> ResyResult<Vec<Value>> {
    if let Some(slot_info) = json["results"]["venues"][0]["slots"].as_array() {
        let mut summarized = Vec::new();
        for slot in slot_info {
            if let (
                Some(config),
                Some(date),
                Some(size),
                Some(quantity)
            ) = (
                slot["config"].as_object(),
                slot["date"].as_object(),
                slot["size"].as_object(),
                slot["quantity"].as_u64()
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
