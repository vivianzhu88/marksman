use std::error::Error;
use prettytable::{row, Table};
use prettytable::cell::Cell;
use prettytable::row::Row;
use reqwest::Client;
use reqwest::header::{AUTHORIZATION, HeaderMap, HeaderValue};
use serde_json::{json, Value};

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

    pub async fn fetch_venue_id(&self, venue_slug: &str) -> Result<String, Box<dyn Error>> {
        let client = reqwest::Client::new();
        let url = format!("https://api.resy.com/3/venue?url_slug={}&location=new-york-ny", venue_slug);

        let mut headers = HeaderMap::new();
        headers.insert(AUTHORIZATION, HeaderValue::from_str(&format!("ResyAPI api_key=\"{}\"", self.api_key)).unwrap());
        headers.insert("x-resy-auth-token", HeaderValue::from_str(&self.auth_token).unwrap());

        let res = client.get(url)
            .headers(headers)
            .send()
            .await?;

        if res.status().is_success() {
            let body = res.text().await?;
            let json: Value = serde_json::from_str(&body)?;
            if let Some(venue_id) = json["id"]["resy"].as_u64() {
                let venue_id_str = venue_id.to_string();
                return Ok(venue_id_str);
            } else {
                println!("venue_id not found");
            }
        } else {
            println!("failed to fetch venue_id: {}", res.status());
        }

        Ok(String::new())
    }

    pub async fn find_reservation_slots(&self, venue_id: &str, day: &str, party_size: u8) -> Result<Vec<Value>, Box<dyn Error>> {
        let client = reqwest::Client::new();
        let url = format!("https://api.resy.com/4/find?lat=0&long=0&day={}&party_size={}&venue_id={}", day, party_size, venue_id);

        let mut headers = HeaderMap::new();
        headers.insert(AUTHORIZATION, HeaderValue::from_str(&format!("ResyAPI api_key=\"{}\"", self.api_key))?);
        headers.insert("x-resy-auth-token", HeaderValue::from_str(&self.auth_token)?);

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

                print_slots_table(&summarized);
                return Ok(summarized);
            } else {
                return Ok(Vec::new());
            }
        } else {
            println!("Failed to fetch reservations: {}", res.status());
            Err(Box::new(std::io::Error::new(std::io::ErrorKind::Other, "Failed to fetch reservations")))
        }
    }


    pub async fn get_reservation_details(
        &self,
        commit: u8, // 0 for dry run, 1 for token gen
        config_id: &str,
        party_size: u8,
        day: &str,
    ) -> Result<String, Box<dyn Error>> {
        let client = reqwest::Client::new();
        let url = "https://api.resy.com/3/details";

        let mut headers = HeaderMap::new();
        headers.insert(AUTHORIZATION, HeaderValue::from_str(&format!("ResyAPI api_key=\"{}\"", self.api_key))?);
        headers.insert("x-resy-auth-token", HeaderValue::from_str(&self.auth_token)?);

        let data = json!({
        "commit": commit,
        "config_id": config_id,
        "day": day,
        "party_size": party_size
    });

        let res = client.post(url)
            .headers(headers)
            .json(&data)
            .send()
            .await?;

        if res.status().is_success() {
            let body = res.text().await?;
            Ok(body)
        } else {
            Err(Box::new(std::io::Error::new(std::io::ErrorKind::Other, format!("Failed to fetch reservation details: {}", res.status()))))
        }
    }
}


fn print_slots_table(slots: &[Value]) {
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