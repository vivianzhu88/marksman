use clap::{Command, Arg};
use std::io::{Read, Write};
use dirs;
use anyhow::{Context, Result};
// use resy_client::ResyClient;

mod resy_client;
mod config;

fn main() -> Result<()> {
    let config_path = config::get_config_path().context("Failed to get config path")?;
    let mut config = config::read_config(&config_path).unwrap_or_else(|_| config::Config {
        api_key: String::new(),
        auth_token: String::new(),
    });


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
        _ => {} // handle new commands
    }

    Ok(())
}