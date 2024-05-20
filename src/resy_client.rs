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

    pub(crate) fn new_from_config(api_key: String, auth_token: String) -> Self {
        ResyClient {
            venue_id: String::new(), // Assuming you are handling venue_id elsewhere
            user_auth: UserAuth {
                api_key,
                auth_token,
            },
        }
    }

    // extract venue_id from restaurant page
    pub(crate) fn get_venue_id(&mut self, url: &str) {
        let venue_slug = extract_venue_slug(url);
        println!("Venue found: {}", venue_slug);
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