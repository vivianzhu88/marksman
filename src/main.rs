use std::io;
use clap::{Command, Arg};
use std::io::Write;
use anyhow::{Context, Result};
use regex::Regex;
use resy_client::ResyClient;

mod resy_client;
mod config;
mod resy_api_gateway;

#[tokio::main]
async fn main() -> Result<()> {
    let config_path = config::get_config_path().context("Failed to get config path")?;
    let marks_config = config::read_config(&config_path)
        .expect("Failed to load configuration");

    let mut resy_client = ResyClient::from_config(marks_config);

    let cli = Command::new("marksman")
        .version("0.1")
        .author("Anish Agrawal")
        .about("Snipe reservations in NYC")
        .subcommand(
            Command::new("hello")
                .about("Prints greeting")
                .arg(
                    Arg::new("name")
                        .help("The name to greet")
                        .value_parser(clap::builder::NonEmptyStringValueParser::new())
                ),
        )
        .subcommand(
            Command::new("venue")
                .about("Details about venue")
                .arg(
                    Arg::new("url")
                        .help("url to Resy booking page")
                        .value_parser(clap::builder::NonEmptyStringValueParser::new())
                        .short('u')
                        .required(false),
                )
                .arg(
                    Arg::new("date")
                        .help("Target date for Resy booking (YYYY-MM-DD)")
                        .value_parser(clap::builder::NonEmptyStringValueParser::new())
                        .short('d')
                        .required(false),
                )
                .arg(
                    Arg::new("target-time")
                        .help("Target time for Resy booking (HHMM)")
                        .value_parser(clap::builder::NonEmptyStringValueParser::new())
                        .short('t')
                        .required(false),
                ),
        )
        .subcommand(
            Command::new("load")
                .about("Load auth credentials for Resy API")
        )
        .subcommand(
            Command::new("state")
                .about("current marksman configuration")
        )
        .subcommand(
            Command::new("snipe")
                .about("configure sniper for the reservation")
        );

    // parse cli
    let matches = cli.get_matches();

    // handling subcommands
    match matches.subcommand() {
        Some(("hello", sub_matches)) => {
            if let Some(name) = sub_matches.get_one::<String>("name") {
                println!("Hello, {}!", name);
            } else {
                println!("Hello, world!");
            }
        }
        Some(("venue", sub_matches)) => {
            let url = sub_matches.get_one::<String>("url").map(String::as_str);
            let date = sub_matches.get_one::<String>("date").map(String::as_str);
            let target_time = sub_matches.get_one::<String>("target-time").map(String::as_str);

            match resy_client.view_venue(url, date, target_time).await {
                Ok(_) => println!("Venue details loaded successfully."),
                Err(e) => println!("Failed to load venue details: {}", e),
            }
        }
        Some(("load", _)) => {
            let mut input_string = String::new();
            println!(">> Enter API Key: ");
            io::stdout().flush().expect("Failed to flush stdout");
            io::stdin().read_line(&mut input_string).expect("Failed to read line");
            let api_key = input_string.trim().to_string().clone();

            input_string.clear();
            println!(">> Enter Auth Token: ");
            io::stdout().flush().expect("Failed to flush stdout");
            io::stdin().read_line(&mut input_string).expect("Failed to read line");
            let auth_token = input_string.trim().to_string().clone();

            resy_client.config.api_key = api_key;
            resy_client.config.auth_token = auth_token;

            println!("Successfully loaded .marksman.config!");
        }
        Some(("state", _)) => {
            let curr_config = config::read_config(&config_path);

            match curr_config {
                Ok(config) => {
                    println!("Current Configuration:\n {:?}", config);
                }
                Err(e) => {
                    println!("Error reading config: {}", e);
                }
            }
        }
        _ => {} // handle new commands
    }

    config::write_config(&resy_client.config, Some(&config_path)).context("Failed to write config")?;
    Ok(())
}


pub fn validate_date_format(val: &str) -> Result<(), String> {
    let re = Regex::new(r"^\d{4}-\d{2}-\d{2}$").unwrap();
    if re.is_match(val) {
        Ok(())
    } else {
        Err(String::from("Date must be in YYYY-MM-DD format"))
    }
}
