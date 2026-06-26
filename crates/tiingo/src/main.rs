mod cli;
mod client;
mod error;
mod format;
mod models;
mod ws;

use chrono::NaiveDate;
use clap::Parser;
use cli::{Cli, Command};

#[tokio::main]
async fn main() {
    rustls::crypto::aws_lc_rs::default_provider()
        .install_default()
        .expect("Failed to install default CryptoProvider");

    let cli = Cli::parse();
    if let Err(e) = run(cli).await {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}

async fn run(cli: Cli) -> Result<(), error::TiingoError> {
    let token = cli::resolve_token(cli.token)?;

    match cli.command {
        Command::Search { query } => {
            let client = client::TiingoClient::new(token);
            let results = client.search(&query).await?;
            format::print_search_results(&results);
        }
        Command::Prices {
            symbol,
            frequency,
            from,
            to,
        } => {
            models::validate_frequency(&frequency)?;
            models::validate_date_format(&from)?;
            models::validate_date_format(&to)?;

            let from_date = NaiveDate::parse_from_str(&from, "%Y-%m-%d").unwrap();
            let to_date = NaiveDate::parse_from_str(&to, "%Y-%m-%d").unwrap();
            if from_date >= to_date {
                return Err(error::TiingoError::InvalidDateRange);
            }

            let client = client::TiingoClient::new(token);
            let bars = if models::is_intraday(&frequency) {
                client.intraday_prices(&symbol, &frequency, &from, &to).await?
            } else {
                client.daily_prices(&symbol, &from, &to).await?
            };

            if bars.is_empty() {
                println!("No data available for {symbol} in the given date range.");
            } else {
                format::print_prices(&bars);
            }
        }
        Command::Live { symbols } => {
            let stream = ws::LiveStream::new(token, symbols);
            stream.run().await?;
        }
    }

    Ok(())
}
