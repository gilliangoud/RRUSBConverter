mod decoder;
mod server;

use std::sync::{atomic::{AtomicBool}, Arc};
use tokio::sync::broadcast;
use crate::decoder::WsMessage;

use clap::Parser;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Serial port to connect to (e.g. "/dev/tty.usbserial-XXXX")
    #[arg(long)]
    serial_port: String,
}

#[tokio::main]
async fn main() {
    let args = Args::parse();
    println!("Starting rrusbconverter...");

    let (tx, _rx) = broadcast::channel::<WsMessage>(100);
    let is_connected = Arc::new(AtomicBool::new(false));

    // Start WebSocket server in the background
    let tx_clone = tx.clone();
    let is_connected_clone = is_connected.clone();
    tokio::spawn(async move {
        server::start_server(tx_clone, 8080, is_connected_clone).await;
    });

    loop {
        println!("Connecting to USB box at {}...", args.serial_port);
        let decoder = decoder::UsbBox::new(args.serial_port.clone());
        
        // Run decoder. If it returns, it means it disconnected.
        decoder.run(tx.clone(), is_connected.clone()).await;
        
        // Disconnected
        println!("USB box disconnected, retrying in 5 seconds...");
        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
    }
}
