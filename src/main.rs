use std::io;
use clap::{Command, Arg};
use std::io::{Read, Write};
use dirs;
use anyhow::{Context, Result};
use resy_client::ResyClient;

mod resy_client;
mod config;

fn main() -> Result<()> {
    let config_path = config::get_config_path().context("Failed to get config path")?;
    let mut auth_config = config::read_config(&config_path).unwrap_or_else(|_| config::Config {
        api_key: String::new(),
        auth_token: String::new(),
    });
    let mut client = ResyClient::new_from_config(auth_config.api_key, auth_config.auth_token);


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
                        .required(true)
                ),
        )
        .subcommand(
            Command::new("load")
                .about("Load auth credentials for Resy API")
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
            let url = sub_matches.get_one::<String>("url").expect("URL is required");
            client.get_venue_id(url);
        }
        Some(("load", _)) => {
            let mut input_string = String::new();
            println!("Enter API Key:");
            io::stdin().read_line(&mut input_string).expect("Failed to read line");
            let api_key = input_string.trim().to_string();

            input_string.clear();
            println!("Enter Auth Token:");
            io::stdin().read_line(&mut input_string).expect("Failed to read line");
            let auth_token = input_string.trim().to_string();

            auth_config.api_key = api_key;
            auth_config.auth_token = auth_token;
            config::write_config(&auth_config, &config_path).context("Failed to write config")?;
        }
        _ => {} // handle new commands
    }

    Ok(())
}