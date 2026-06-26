mod cli;
mod client;
mod error;
mod format;
mod models;
mod ws;

use chrono::NaiveDate;
use clap::Parser;
use cli::{Cli, Command};
use client::MassiveClient;
use error::MassiveError;

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

async fn run(cli: Cli) -> Result<(), MassiveError> {
    let api_key = cli::resolve_token(cli.token)?;

    match cli.command {
        Command::Search { query, limit } => {
            models::validate_limit(limit, 1000)?;
            let client = MassiveClient::new(api_key);
            let response = client.search(&query, limit).await?;
            format::print_search_results(&response.results);
        }
        Command::Bars {
            ticker,
            multiplier,
            timespan,
            from,
            to,
            sort,
            limit,
        } => {
            models::validate_timespan(&timespan)?;
            models::validate_sort(&sort)?;
            models::validate_date_format(&from)?;
            models::validate_date_format(&to)?;
            models::validate_limit(limit, 50_000)?;
            // Validate date ordering
            let from_date = NaiveDate::parse_from_str(&from, "%Y-%m-%d").unwrap();
            let to_date = NaiveDate::parse_from_str(&to, "%Y-%m-%d").unwrap();
            if from_date >= to_date {
                return Err(MassiveError::InvalidDateRange);
            }
            // Validate multiplier
            if multiplier < 1 {
                return Err(MassiveError::Other(
                    "multiplier must be a positive integer".to_string(),
                ));
            }
            let client = MassiveClient::new(api_key);
            let response = client
                .bars(&ticker, multiplier, &timespan, &from, &to, &sort, limit)
                .await?;
            if response.results.is_empty() {
                println!("No data available for {ticker} in the given date range.");
            } else {
                format::print_bars(&response.results);
            }
        }
        Command::Quote { from, to } => {
            let client = MassiveClient::new(api_key);
            let response = client.last_quote(&from, &to).await?;
            format::print_quote(&response);
        }
        Command::Convert {
            from,
            to,
            amount,
            precision,
        } => {
            if amount <= 0.0 {
                return Err(MassiveError::Other(
                    "amount must be greater than 0".to_string(),
                ));
            }
            if precision > 10 {
                return Err(MassiveError::Other(
                    "precision must be no greater than 10".to_string(),
                ));
            }
            let client = MassiveClient::new(api_key);
            let response = client.convert(&from, &to, amount, precision).await?;
            format::print_conversion(&response);
        }
        Command::Snapshot { ticker } => {
            let client = MassiveClient::new(api_key);
            let response = client.snapshot(&ticker).await?;
            format::print_snapshot(&response.ticker);
        }
        Command::Live { pairs } => {
            let stream = ws::LiveStream::new(api_key, pairs);
            stream.run().await?;
        }
    }
    Ok(())
}
