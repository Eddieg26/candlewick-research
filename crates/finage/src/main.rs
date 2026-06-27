mod cli;
mod client;
mod error;
mod format;
mod models;
mod ws;

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

async fn run(cli: Cli) -> Result<(), error::FinageError> {
    let token = cli::resolve_token(cli.token)?;

    match cli.command {
        Command::Search { query, market } => {
            models::validate_market(&market)?;
            let client = client::FinageClient::new(token);
            let response = client.search(&query, &market).await?;
            if response.symbols.is_empty() {
                println!("No symbols matched the query.");
            } else {
                format::print_search_results(&response.symbols);
            }
        }
        Command::Bars {
            symbol,
            multiplier,
            timespan,
            from,
            to,
            sort,
            limit,
        } => {
            // Validate in order: timespan → sort → multiplier → limit → from → to → range
            models::validate_timespan(&timespan)?;
            models::validate_sort(&sort)?;
            models::validate_multiplier(multiplier)?;
            models::validate_limit(limit)?;
            let from_ts = models::parse_date_to_unix(&from)?;
            let to_ts = models::parse_date_to_unix(&to)?;
            if from_ts >= to_ts {
                return Err(error::FinageError::InvalidDateRange);
            }
            let client = client::FinageClient::new(token);
            let response = client
                .bars(&symbol, multiplier, &timespan, &from, &to, &sort, limit)
                .await?;
            if response.results.is_empty() {
                println!("No data available for {symbol} in the given date range.");
            } else {
                format::print_bars(&response.results);
            }
        }
        Command::Live { symbols } => {
            if symbols.is_empty() || symbols.len() > 20 {
                return Err(error::FinageError::Other(
                    "Must provide between 1 and 20 symbols.".to_string(),
                ));
            }
            let stream = ws::LiveStream::new(token, symbols);
            stream.run().await?;
        }
    }

    Ok(())
}
