#[macro_use] extern crate prettytable;
use std::io;
use clap::{Command, Arg, ArgAction};
use std::io::Write;
use anyhow::{Context, Result};
use regex::Regex;
use resy_client::ResyClient;
use std::sync::Arc;
use env_logger::{Env};
use chrono::{Local, Duration};

mod resy_client;
mod config;
mod resy_api_gateway;
mod view_utils;

#[tokio::main]
async fn main() -> Result<()> {

    // setup logging
    let env = Env::default().default_filter_or("trace");
    env_logger::init_from_env(env);

    let config_path = config::get_config_path().context("Failed to get config path")?;
    let marks_config = config::read_config(&config_path)
        .expect("Failed to load configuration");

    let mut resy_client = ResyClient::from_config(marks_config);

    // define cli commands
    let cli = Command::new("marksman")
        .version("0.1.0")
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
                        .long("url")
                        .required(false),
                )
                .arg(
                    Arg::new("date")
                        .help("Target date for Resy booking (YYYY-MM-DD)")
                        .value_parser(clap::builder::NonEmptyStringValueParser::new())
                        .short('d')
                        .long("date")
                        .required(false),
                )
                .arg(
                    Arg::new("party-size")
                        .help("Party size for Resy booking")
                        .value_parser(clap::value_parser!(u8))
                        .short('p')
                        .long("party-size")
                        .required(false),
                )
                .arg(
                    Arg::new("target-time")
                        .help("Target time for Resy booking (HHMM)")
                        .value_parser(clap::builder::NonEmptyStringValueParser::new())
                        .short('t')
                        .long("target-time")
                        .required(false),
                )
                .arg(
                    Arg::new("reset-time")
                        .help("Reset target time for Resy booking (None)")
                        .short('r')
                        .long("reset-time")
                        .action(ArgAction::SetTrue),
                ),
        )
        .subcommand(
            Command::new("load")
                .about("Load auth credentials for Resy API")
                .arg(
                    Arg::new("skip")
                        .help("skip loading new credentials (sets payment id)")
                        .short('s')
                        .long("skip")
                        .action(ArgAction::SetTrue),
                ),
        )
        .subcommand(
            Command::new("state")
                .about("current marksman configuration")
        )
        .subcommand(
            Command::new("snipe")
                .about("configure sniper for the reservation")
                .arg(
                    Arg::new("snipe-time")
                        .help("Snipe time for Resy booking (HHMM)")
                        .value_parser(clap::builder::NonEmptyStringValueParser::new())
                        .short('t')
                        .long("snipe-time")
                        .required(false),
                )
                .arg(
                    Arg::new("snipe-date")
                        .help("Snipe date for Resy booking (YYYY-MM-DD). shortcut dates with 'today' or 'tmrw'")
                        .value_parser(clap::builder::NonEmptyStringValueParser::new())
                        .short('d')
                        .long("snipe-date")
                        .required(false),
                )
        )
        .subcommand(
            Command::new("setup")
                .about("configure setup wizard")
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
        Some(("setup", _)) => {

        }
        Some(("venue", sub_matches)) => {
            let url = sub_matches.get_one("url").map(String::as_str);
            let date = sub_matches.get_one("date").map(String::as_str);
            let party_size = sub_matches.get_one("party-size").copied();
            let mut target_time = sub_matches.get_one("target-time").map(String::as_str);

            if sub_matches.get_flag("reset-time") {
                target_time = None;
            }

            match resy_client.view_venue(url, date, party_size, target_time).await {
                Ok((_, slots)) => {
                    println!("venue details loaded successfully");
                    view_utils::print_table(&slots);
                },
                Err(e) => println!("Failed to load venue details: {}", e),
            }
        }
        Some(("load", sub_matches)) => {
            if !sub_matches.get_flag("skip") {
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

            match resy_client.get_payment_id().await {
                Ok(payment_id) => println!("Payment id found: {}", payment_id),
                Err(e) => println!("Failed to load payment_id: {}", e),
            }

        }
        Some(("state", _)) => {
            match serde_json::to_string_pretty(&resy_client.config) {
                Ok(json_string) => println!("Current Configuration:\n{}", json_string),
                Err(e) => println!("Failed to serialize config: {}", e),
            }
        }
        Some(("snipe", sub_matches)) => {
            let snipe_time = sub_matches.get_one("snipe-time").map(String::as_str).unwrap_or("0000");
            let snipe_date = sub_matches.get_one("snipe-date").map(String::as_str);

            // Determine the date based on input
            let formatted_date = match snipe_date {
                Some("today") => Local::now().format("%Y-%m-%d").to_string(),
                Some("tmrw") => (Local::now() + Duration::days(1)).format("%Y-%m-%d").to_string(),
                _ => snipe_date.unwrap_or_default().to_string(),
            };

            match resy_client.run_sniper(snipe_time, &formatted_date).await {
                Ok(tok) => println!("Successful booking! (token: {:#?})", tok),
                Err(e) => println!("Snipe failed with {}", e)
            }
        }
        _ => {} // handle new commands
    }

    config::write_config(&resy_client.config, Some(&config_path)).context("Failed to write config")?;
    Ok(())
}
