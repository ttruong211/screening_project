use clap::Parser;
use tokio::{main, task};

mod config;
mod tls;
mod ws;

#[derive(Debug, Parser)]
struct Args {
    /// List of symbols to subscribe to (comma separated), e.g. -s BTCUSDT,ETHUSDT
    #[arg(short, long, value_delimiter = ',')]
    symbols: Vec<String>,
}

#[main]
async fn main() -> anyhow::Result<()> {
    // Parse command-line arguments
    let args = Args::parse();

    if args.symbols.is_empty() {
        eprintln!("⚠️ Please provide at least one symbol using -s BTCUSDT,ETHUSDT,...");
        return Ok(());
    }

    // Build combined stream path (e.g., btcusdt@bookTicker/ethusdt@bookTicker)
    let stream_path = args
        .symbols
        .iter()
        .map(|s| format!("{}@bookTicker", s.to_lowercase()))
        .collect::<Vec<_>>()
        .join("/");

    // Construct the combined WebSocket URL
    let url = format!("wss://fstream.binance.com/stream?streams={}", stream_path);
    println!("📡 Connecting to Binance WebSocket at: {}", url);

    let collector = ws::client::connect_to_binance(&url).await?;
    ws::client::deserialize_book_ticket(collector, args.symbols).await?; 

    Ok(())
}




