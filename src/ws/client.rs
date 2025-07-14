// Handles the connection to Binance API 

use fastwebsockets::{handshake, FragmentCollector, OpCode, WebSocket}; use hyper_util::rt::TokioIo;
// WebSocket functionality from the fastwebsockets crate  
use tokio::{io::{AsyncRead, AsyncWrite}, net::TcpStream}; // Asynchronous I/O and networking 
use tokio_rustls::rustls::{ServerName}; 
use hyper::{Request, Uri, upgrade::Upgraded}; // HTTP request and URI handling
use hyper::header::{CONNECTION, UPGRADE}; // HTTP headers for WebSocket upgrade
use std::{result, str::FromStr};  // implement a method through trait 
use std::collections::{HashMap, VecDeque}; 
use crate::tls::connector::tls_connect;
use anyhow::Result;
use http_body_util::{combinators::Collect, Empty}; 
use bytes::Bytes; 
use serde_json::{Value}; 
use std::time::{Duration, Instant}; 

struct SpawnExecutor; 
impl<Fut> hyper::rt::Executor<Fut> for SpawnExecutor
where
  Fut: Future + Send + 'static,
  Fut::Output: Send + 'static,
{
  fn execute(&self, fut: Fut) {
    tokio::task::spawn(fut);
  }
} 

pub async fn deserialize_book_ticket(mut collector: FragmentCollector<TokioIo<Upgraded>>, symbols: Vec<String>) -> Result<()> {
    // let mut mid_prices: Vec<f64> = Vec::new();
    let mut last_print = Instant::now(); // Track last print time 
    let mut symbol_prices = HashMap::new(); 
    let mut mid_price_history: HashMap<String, VecDeque<(Instant, f64, f64)>> = HashMap::new(); // Store mid prices for each symbol 

    loop {
        let frame = collector.read_frame().await?;

        if frame.opcode == OpCode::Text {
            let text = std::str::from_utf8(&frame.payload)?;
            let book_ticket_array = serde_json::from_str::<serde_json::Value>(text).unwrap();
 
            if let Some(data) = book_ticket_array.get("data") {
                let symbol = data.get("s").and_then(|v| v.as_str()); // Streamed symbol 
                let bid = data.get("b").and_then(|v| v.as_str()).and_then(|s| s.parse::<f64>().ok());
                let ask = data.get("a").and_then(|v| v.as_str()).and_then(|s| s.parse::<f64>().ok());
                let bid_qty = data.get("B").and_then(|v| v.as_str()).and_then(|s| s.parse::<f64>().ok());
                let ask_qty = data.get("A").and_then(|v| v.as_str()).and_then(|s| s.parse::<f64>().ok());
                
                if let (Some(symbol), Some(bid), Some(ask), Some(bid_qty), Some(ask_qty)) = (symbol, bid, ask, bid_qty, ask_qty) {
                    let now = Instant::now(); // Current time 
                    let mid_price = (bid + ask) / 2.0; // Calculate mid price 
                    let weighted_mid_price = (bid * ask_qty + ask * bid_qty) / (bid_qty + ask_qty); // Weighted mid price 

                    // Update map 
                    symbol_prices.insert(symbol.to_string(), (mid_price, weighted_mid_price));  

                    let history = mid_price_history
                        .entry(symbol.to_string())
                        .or_insert(VecDeque::new());                        
                    
                    history.push_back((now, mid_price, weighted_mid_price)); 

                    // Remove data older than 60 seconds
                    while let Some((timestamp, _, _)) = history.front() {
                        if now.duration_since(*timestamp) > Duration::from_secs(60) {
                            history.pop_front();
                        } else {
                            break;
                        }
                    } 

                    // Every 5 seconds, compute and print summary
                    if last_print.elapsed() >= Duration::from_secs(5) && symbols.iter().all(|s| symbol_prices.contains_key(s)) {
                        println!("----- Market Summary -----");
                        for (symbol, (mid_price, weighted_mid_price)) in &symbol_prices {
                            let empty = VecDeque::new(); 
                            let history = mid_price_history.get(symbol).unwrap_or(&empty);

                            let (min_mid, max_mid) = history
                                .iter()
                                .map(|(_, p, _)| *p)
                                .fold((f64::INFINITY, f64::NEG_INFINITY), |(min, max), val| {
                                    (min.min(val), max.max(val))
                                });

                            let (min_weighted_mid, max_weighted_mid) = history
                                .iter()
                                .map(|(_, _, p)| *p)
                                .fold((f64::INFINITY, f64::NEG_INFINITY), |(min, max), val| {
                                    (min.min(val), max.max(val))
                                });

                            println!(
                                "Symbol: {}, mid_price: {:.2}, weighted_mid_price: {:.2}, 1min_max_mid: {:.2}, 1min_min_mid: {:.2}, 1min_max_weighted_mid”: {:.2}, 1min_min_weighted_mid: {:.2}",
                                symbol, mid_price, weighted_mid_price, max_mid, min_mid, max_weighted_mid, min_weighted_mid
                            );
                        }
                        println!("--------------------------\n"); 
                        last_print = Instant::now(); // Reset timer 
                    }
                }
            }  
        }
    }
}


pub async fn connect_to_binance(url: &str) -> Result<FragmentCollector<TokioIo<Upgraded>>> {
 
    let uri = Uri::from_str(&url)?; // Parse the URL into a URI 
    let host = uri.host().unwrap();  
    let domain = ServerName::try_from(host)
        .map_err(|_| anyhow::anyhow!("Invalid domain"))?; // Convert host to ServerName for TLS 
    let port = uri.port_u16().unwrap_or(443); 

    // TCP + TLS connection 
    let tcp_stream = TcpStream::connect((host, port)).await?;
    let tls_stream = tls_connect(tcp_stream, domain).await?; 

    // Build WebSocket upgrade request
    let request = Request::builder()
        .method("GET")
        .uri(uri.clone()) // reuse the parsed URI
        .header("Host", host)
        .header(UPGRADE, "websocket")
        .header(CONNECTION, "Upgrade")
        .header("Sec-WebSocket-Version", "13")
        .header("Sec-WebSocket-Key", handshake::generate_key())
        .body(Empty::<Bytes>::new())?; // empty body for handshake

    // Perform WebSocket handshake 
    let (ws, _) = handshake::client(&SpawnExecutor, request, tls_stream).await?;
    let collector = FragmentCollector::new(ws); 

    println!{"WebSocket handshake complete"}; 
    Ok(collector)  

}