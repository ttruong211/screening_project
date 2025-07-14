Binance Mid Streamer

An async Rust program using tokio that connects to the Binance Futures bookTicker WebSocket API and streams live order book data for given symbols. It calculates and prints mid-price statistics every 5 seconds.

How to run: 
cargo run --release -- -s <SYMBOL1,SYMBOL2,...>

Output: 
- Mid Price, Weighted Mid Price, 1-min max/min of both mid and weighted mid price 
