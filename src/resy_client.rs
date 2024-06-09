use std::error::Error;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::{Duration as StdDuration, Instant};
use futures::future::join_all;
use chrono::{DateTime, Duration, Local, NaiveDate, NaiveTime, TimeZone, Utc};
use log::{debug, error, info};
use serde_json::{Value};
use serde::Deserialize;
use tokio::sync::Mutex;
use rand;
use rand::Rng;
use tokio::time::{sleep, Duration as TokioDuration};
use crate::config::Config;
use crate::resy_api_gateway::ResyAPIGateway;

#[derive(Debug)]
pub enum ResyClientError {
    NotFound(String),
    NetworkError(String),
    ApiError(String),
    InternalError(String),
    InvalidInput(String),
    ParseError(String),
    BookingError(String),
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

    pub(crate) async fn view_venue(&mut self, url: Option<&str>, date: Option<&str>, party_size: Option<u8>, target_time: Option<&str>) -> ResyResult<(String, Vec<ResySlot>)> {
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

        let mut slots = self._find_reservation_slots().await?;
        if let Some(target_time) = &self.config.target_time {
            slots = sort_slots_by_closest_time(slots, target_time);
        }

        let venue_id = self.config.venue_id.clone();
        Ok((venue_id, slots))
    }

    pub(crate) async fn run_sniper(&mut self, snipe_time: &str, snipe_date: &str) -> ResyResult<String> {
        let date = NaiveDate::parse_from_str(snipe_date, "%Y-%m-%d")
            .map_err(|_| ResyClientError::InvalidInput("Invalid date format".to_string()))?;
        let time = NaiveTime::parse_from_str(snipe_time, "%H%M")
            .map_err(|_| ResyClientError::InvalidInput("Invalid time format".to_string()))?;
        let naive_datetime = date.and_time(time);
        let datetime = Local.from_local_datetime(&naive_datetime).single()
            .ok_or(ResyClientError::InvalidInput("Could not convert to local datetime".to_string()))?;

        if datetime <= Local::now() + Duration::minutes(1) {
            return Err(ResyClientError::InvalidInput("Snipe date/time is in the past".to_string()));
        }

        self.config.snipe_date = snipe_date.to_string();
        self.config.snipe_time = snipe_time.to_string();

        let mut remaining = datetime - Local::now();

        let seconds_to_sleep = remaining.num_seconds() % 60;
        if seconds_to_sleep > 0 {
            sleep(TokioDuration::from_secs(seconds_to_sleep as u64)).await;
        }

        remaining = datetime - Local::now();
        while remaining > Duration::seconds(0) {
            if remaining <= Duration::minutes(2) {
                // Log more frequently as the time approaches
                info!("Time remaining: {} seconds", remaining.num_seconds());
                sleep(TokioDuration::from_secs(1)).await;
            } else {
                // Log periodically
                info!("Time remaining: {} minutes", remaining.num_minutes());
                sleep(TokioDuration::from_secs(60)).await;
            }
            remaining = datetime - Local::now();
        }


        if !self.config.validate() {
            return Err(ResyClientError::InvalidInput("reservation config is not complete".to_string()));
        }

        let mut slots = self._find_reservation_slots().await?;
        if let Some(target_time) = &self.config.target_time {
            slots = sort_slots_by_closest_time(slots, target_time);
        }

        if slots.is_empty() {
            return Err(ResyClientError::NotFound("no reservation slots available".to_string()));
        }

        for slot in slots {
            match self._sniper_task(&slot.token, &slot.start).await {
                Ok(tok) => {
                    return Ok(tok)
                }
                Err(e) => {}
            }
        }

        Err(ResyClientError::BookingError("Booking failure: all slots failed".to_string()))
    }

    async fn _sniper_task(&self, config_id: &str, time_slot: &str) -> ResyResult<String> {
        info!("Running snipe @ {} (token: {})", time_slot, config_id);

        let book_token = match self.api_gateway.get_reservation_details(1, &config_id, self.config.party_size, &self.config.date).await {
            Ok(json) => {
                debug!("Reservation details response {:#?}", json);

                if json.get("book_token").is_some() {
                    match json["book_token"]["value"].as_str() {
                        Some(token) => token.to_string(),
                        None => return Err(ResyClientError::BookingError("Book token not found".to_string()))
                    }
                } else {
                    return Err(ResyClientError::BookingError("Error fetching book token".to_string())) // didn't get it in time!
                }
            }
            Err(e) => {
                error!("Error getting book token {:?}", e);
                return Err(ResyClientError::BookingError("Error fetching book token".to_string()))
            }
        };

        info!("Book token acquired @ {} (token: {})", time_slot, book_token);

        return match self.api_gateway.book_reservation(&book_token, &self.config.payment_id).await {
            Ok(json) => {
                debug!("Booking reservation response {:#?}", json);

                match json.get("resy_token") {
                    Some(token) => {
                        info!("acquired {} (token: {})", time_slot, token);
                        Ok(token.to_string())
                    },
                    None => Err(ResyClientError::BookingError("Error booking reservation".to_string())),
                }
            }
            Err(e) => {
                error!("Error booking reservation {:?}", e);
                Err(ResyClientError::BookingError("Error booking reservation".to_string()))
            }
        };
    }

    // pub(crate) async fn run_snipe(self: Arc<ResyClient>) -> ResyResult<String> {
    //     if !self.config.validate() {
    //         return Err(ResyClientError::InvalidInput("reservation config is not complete".to_string()));
    //     }
    //
    //     let mut slots = self._find_reservation_slots().await?;
    //
    //     if slots.is_empty() {
    //         return Err(ResyClientError::NotFound("no reservation slots available".to_string()));
    //     }
    //
    //     let mut tasks = vec![];
    //     let mutex = Arc::new(Mutex::new(()));
    //     let booking_successful = Arc::new(AtomicBool::new(false));
    //
    //     for slot in slots {
    //         // Only spawn tasks if the slot has a valid 'config_id'
    //         let cloned_config_id = slot.token.clone();
    //         let time_slot = slot.start.clone();
    //         let self_clone: Arc<ResyClient> = Arc::clone(&self);
    //         let lock = mutex.clone();
    //         let booking_successful_clone = Arc::clone(&booking_successful);
    //
    //         tasks.push(tokio::spawn(async move {
    //             self_clone._snipe_task(cloned_config_id, time_slot, lock, booking_successful_clone).await
    //         }));
    //     }
    //
    //     let results = join_all(tasks).await;
    //
    //     Ok("Placeholder for compilation".to_string())
    // }
    //
    // async fn _snipe_task(&self, config_id: String, time_slot: String, book_mutex: Arc<Mutex<()>>, booking_successful: Arc<AtomicBool>) -> Option<String> {
    //     info!("Running snipe @ {} (token: {})", time_slot, config_id);
    //
    //     let book_token = match self.api_gateway.get_reservation_details(1, &config_id, self.config.party_size, &self.config.date).await {
    //         Ok(json) => {
    //             debug!("Reservation details response {:#?}", json);
    //
    //             if json.get("book_token").is_some() {
    //                 match json["book_token"]["value"].as_str() {
    //                     Some(token) => token.to_string(),
    //                     None => return None,
    //                 }
    //             } else {
    //                 return None // didn't get it in time!
    //             }
    //         }
    //         Err(e) => {
    //             error!("Error getting book token {:?}", e);
    //             return None
    //         }
    //     };
    //
    //     info!("Book token acquired @ {} (token: {})", time_slot, book_token);
    //
    //     // locked block, one task at a time
    //     {
    //         let _guard = book_mutex.lock().await;
    //
    //         if booking_successful.load(Ordering::SeqCst) {
    //             info!("Already got a booking!");
    //             return None; // Recheck the flag after acquiring the lock to avoid race condition
    //         }
    //
    //         // let mut rng = rand::thread_rng(); // Get a random number generator
    //         // let num = rng.gen_range(0..=1);
    //         //
    //         // if num != 0 {
    //         //     println!("locked a reservation");
    //         //     booking_successful.store(true, Ordering::SeqCst);
    //         //     return true
    //         // }
    //         // println!("failed a reservation");
    //
    //         let resy_token = match self.api_gateway.book_reservation(&book_token, &self.config.payment_id).await {
    //             Ok(json) => {
    //                 debug!("Booking reservation response {:#?}", json);
    //
    //                 match json.get("resy_token") {
    //                     Some(token) => {
    //                         booking_successful.store(true, Ordering::SeqCst);
    //                         info!("acquired {} (token: {})", time_slot, token);
    //                         Some(token.to_string())
    //                     },
    //                     None => None,
    //                 }
    //             }
    //             Err(e) => {
    //                 error!("Error booking reservation {:?}", e);
    //                 None
    //             }
    //         };
    //
    //         info!("token... @ {:?}", resy_token);
    //     }
    //
    //     None
    // }

    pub(crate) async fn get_payment_id(&mut self) -> ResyResult<String> {
        match self.api_gateway.get_user().await {
            Ok(user_data) => {
                let payment_methods = user_data["payment_methods"]
                    .as_array()
                    .ok_or_else(|| ResyClientError::NotFound("No payment method found in resy account".to_string()))?;

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

    async fn _find_reservation_slots(&self) -> ResyResult<Vec<ResySlot>> {
        match self.api_gateway.find_reservation(self.config.venue_id.as_str(), self.config.date.as_str(), self.config.party_size, self.config.target_time.as_deref()).await {
            Ok(json) => Ok(format_slots(json)),
            Err(e) => {
                Err(ResyClientError::ApiError(format!("Error fetching venue: {:?}", e)))
            }
        }
    }
}

// UTILS

fn extract_venue_slug(url: &str) -> ResyResult<String> {
    if let Some(start) = url.find("venues/") {
        let start = start + "venues/".len();
        let end = url[start..].find('?').unwrap_or_else(|| url[start..].len());
        return Ok(url[start..start + end].to_string());
    }
    Err(ResyClientError::InvalidInput("invalid resy url".to_string()))
}

#[derive(Deserialize, Debug)]
pub(crate) struct ResySlot {
    pub(crate) id: String,
    pub(crate) token: String,
    pub(crate) slot_type: String,
    pub(crate) start: String,
    pub(crate) end: String,
    pub(crate) min_size: u64,
    pub(crate) max_size: u64,
    pub(crate) quantity: u64,
}

fn format_slots(json: Value) -> Vec<ResySlot> {
    if let Some(slots) = json["results"]["venues"][0]["slots"].as_array() {
        let summarized: Vec<ResySlot> = slots.iter().filter_map(|slot| {

            let config = slot["config"].as_object()?;
            let date = slot["date"].as_object()?;
            let size = slot["size"].as_object()?;

            Some(ResySlot {
                id: config.get("id")?.as_number()?.to_string(),
                token: config.get("token")?.as_str()?.to_string(),
                slot_type: config.get("type")?.as_str()?.to_string(),
                start: date.get("start")?.as_str()?.to_string(),
                end: date.get("end")?.as_str()?.to_string(),
                min_size: size.get("min")?.as_u64()?,
                max_size: size.get("max")?.as_u64()?,
                quantity: slot.get("quantity")?.as_u64()?,
            })
        }).collect();

        summarized
    } else {
        Vec::new()
    }
}

fn sort_slots_by_closest_time(slots: Vec<ResySlot>, target_time: &str) -> Vec<ResySlot> {
    let target_time = match NaiveTime::parse_from_str(target_time, "%H%M") {
        Ok(time) => time,
        Err(_) => return Vec::new(), // Return an empty vector if there's a parsing error
    };

    let mut slots_with_time: Vec<(ResySlot, NaiveTime)> = slots.into_iter().filter_map(|slot| {
        NaiveTime::parse_from_str(&slot.start[11..16], "%H:%M").ok().map(|time| (slot, time))
    }).collect();

    slots_with_time.sort_by_key(|(_, time)| {
        let duration = if *time > target_time {
            time.signed_duration_since(target_time)
        } else {
            target_time.signed_duration_since(*time)
        };
        duration.num_minutes().abs() as u64 // Abs to avoid panic on negative durations
    });

    slots_with_time.into_iter().map(|(slot, _)| slot).collect()
}
