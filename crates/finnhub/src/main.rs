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

async fn run(cli: Cli) -> Result<(), error::FinnhubError> {
    let token = cli::resolve_token(cli.token)?;

    match cli.command {
        Command::Search { query, exchange } => {
            let client = client::FinnhubClient::new(token);
            let response = client.search(&query, exchange.as_deref()).await?;
            format::print_search_results(&response.result);
        }
        Command::Candles {
            symbol,
            resolution,
            from,
            to,
        } => {
            let from_ts = models::parse_date_to_unix(&from)?;
            let to_ts = models::parse_date_to_unix(&to)?;
            let client = client::FinnhubClient::new(token);
            let response = client.candles(&symbol, &resolution, from_ts, to_ts).await?;

            if response.s == "no_data" {
                println!("No data available for {symbol} in the given date range.");
            } else {
                format::print_candles(&response.into_candles());
            }
        }
        Command::Live { symbols } => {
            let stream = ws::LiveStream::new(token, symbols);
            stream.run().await?;
        }
    }

    Ok(())
}
