use futures::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tokio::sync::broadcast;
use tokio_util::codec::{Framed, LinesCodec};
use std::sync::{atomic::{AtomicBool, Ordering}, Arc};
use tokio_serial::{SerialPort, SerialPortBuilderExt, SerialStream};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Passing {
    pub passing_number: String,
    pub transponder: String,
    pub date: String, // ISO 8601 format
    pub time: String,
    pub event_id: String,
    pub hits: String,
    pub max_rssi: String,
    pub internal_data: String, // hex
    pub is_active: String, // 1/0
    pub channel: String,
    pub loop_id: String,
    pub loop_id_wakeup: String,
    pub battery: String,
    pub temperature: String,
    pub internal_active_data: String, // hex
    pub box_temp: String,
    pub box_reader_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum WsMessage {
    Passing(Passing),
    Status { event: String },
}

pub struct UsbBox {
    port_name: String,
    poll_interval: u64,
    ref_computer_time: Option<i64>,
    ref_internal_time: Option<u64>,
}

impl UsbBox {
    pub fn new(port_name: String, poll_interval: u64) -> Self {
        Self { 
            port_name,
            poll_interval,
            ref_computer_time: None,
            ref_internal_time: None,
        }
    }

    pub async fn run(mut self, tx: broadcast::Sender<WsMessage>, is_connected: Arc<AtomicBool>) {
        println!("Opening serial port {}", self.port_name);
        
        let mut port = match tokio_serial::new(&self.port_name, 19200).open_native_async() {
            Ok(p) => p,
            Err(e) => {
                eprintln!("Failed to open serial port: {}", e);
                return;
            }
        };

        #[cfg(unix)]
        port.set_exclusive(false).expect("Unable to set serial port exclusive to false");

        // DTR Low to start (avoid reset)
        port.write_data_terminal_ready(false).expect("Failed to set DTR low");

        println!("Connected to serial port");
        
        // Status: Connected
        is_connected.store(true, Ordering::SeqCst);
        let _ = tx.send(WsMessage::Status { event: "connected".to_string() });

        if let Err(e) = self.handle_connection(port, &tx).await {
            eprintln!("Connection error: {}", e);
        }

        // Status: Disconnected
        if is_connected.load(Ordering::SeqCst) {
            is_connected.store(false, Ordering::SeqCst);
            let _ = tx.send(WsMessage::Status { event: "disconnected".to_string() });
        }
    }

    async fn handle_connection(
        &mut self,
        mut port: SerialStream,
        tx: &broadcast::Sender<WsMessage>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Wait 3 seconds for bootloader
        println!("Waiting 3s for bootloader...");
        tokio::time::sleep(Duration::from_secs(3)).await;

        let mut framed = Framed::new(port, LinesCodec::new());

        // Step 1: Switch to ASCII-Timing Protocol (just in case FW 2.4)
        framed.send("ASCII").await?;
        tokio::time::sleep(Duration::from_millis(500)).await;

        // Step 2: Pair & Sync computer time
        let now = chrono::Utc::now().timestamp();
        let hex_time = format!("{:x}", now);
        
        println!("Syncing time with timestamp: {} ({})", now, hex_time);
        framed.send(format!("EPOCHREFSET;{}", hex_time)).await?;

        // Toggle DTR line to confirm sync
        framed.get_mut().write_data_terminal_ready(true)?;
        tokio::time::sleep(Duration::from_millis(100)).await;
        framed.get_mut().write_data_terminal_ready(false)?;

        // Step 3: Enable Push Passings (FW 2.6+)
        println!("Enabling push passings (SETCONF;b2;1)...");
        framed.send("SETCONF;b2;1").await?;
        
        // Give it a moment
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Step 4: Get existing passings (just in case)
        framed.send("PASSINGGET;00000000").await?;

        let mut interval = tokio::time::interval(Duration::from_millis(self.poll_interval));

        loop {
            tokio::select! {
                _ = interval.tick() => {
                    // Request passings periodically
                    if let Err(e) = framed.send("PASSINGGET;00000000").await {
                        return Err(Box::new(e));
                    }
                }
                msg = framed.next() => {
                    match msg {
                        Some(Ok(msg)) => {
                            self.process_message(&msg, tx);
                        }
                        Some(Err(e)) => return Err(Box::new(e)),
                        None => return Err("Connection closed".into()),
                    }
                }
            }
        }
    }

    fn process_message(&mut self, msg: &str, tx: &broadcast::Sender<WsMessage>) {
        // println!("Received: {}", msg);
        let parts: Vec<&str> = msg.split(';').collect();
        if parts.is_empty() {
            return;
        }

        // Handle EPOCHREFSET response
        // Format: 4a3caa45;0151bcf5 (ComputerTime;InternalTime)
        if parts.len() == 2 && parts[0].len() == 8 && parts[1].len() == 8 {
             if let (Ok(comp_time), Ok(int_time)) = (
                i64::from_str_radix(parts[0], 16),
                u64::from_str_radix(parts[1], 16)
            ) {
                // Check if this looks like a time sync response (heuristic: comp_time is recent)
                let now = chrono::Utc::now().timestamp();
                if (comp_time - now).abs() < 3600 { // Within an hour
                    println!("Time sync established: Comp={}, Int={}", comp_time, int_time);
                    self.ref_computer_time = Some(comp_time);
                    self.ref_internal_time = Some(int_time);
                    return;
                }
            }
        }

        // Standard Passing Format:
        // [TranspCode];[WakeupCounter];[TimeStamp];[Hits];[RSSI];[Battery];[Temperature];[LoopOnly];[LoopId];[Channel];[InternalActiveData];[InternalData]
        // Example: IKNWZ06;a153;093a9eb4;fe;71;1d;15;0;1;7;00;0
        
        if parts.len() >= 12 {
            // Filter out command responses
            if parts[0] == "PASSINGGET" || parts[0] == "EPOCHREFSET" || parts[0] == "ASCII" || parts[0] == "SETCONF" {
                return;
            }
            
            // Check if parts[0] is likely a transponder (alphanumeric)
            // and parts[2] is a timestamp (hex)
            
            let transponder = parts[0].to_string();
            let timestamp_hex = parts[2];
            
            let mut date_str = "".to_string();
            let mut time_str = "".to_string();
            
            if let Ok(ts_ticks) = u64::from_str_radix(timestamp_hex, 16) {
                // Calculate real time
                // Computer_Time = ref_computer_time + ((pass_time_stamp - ref_time_stamp) / 256.0)
                if let (Some(ref_comp), Some(ref_int)) = (self.ref_computer_time, self.ref_internal_time) {
                    // Use signed arithmetic to handle passings before the sync point (stored passings)
                    let diff_ticks = (ts_ticks as i64) - (ref_int as i64);
                    
                    // Ticks are 1/256 sec (Standard Format)
                    let diff_seconds = diff_ticks as f64 / 256.0;
                    let passing_time_unix = ref_comp as f64 + diff_seconds;
                    
                    let secs = passing_time_unix as i64;
                    let nsecs = ((passing_time_unix - secs as f64) * 1_000_000_000.0) as u32;
                    
                    if let Some(dt) = chrono::DateTime::from_timestamp(secs, nsecs) {
                        let local_dt: chrono::DateTime<chrono::Local> = chrono::DateTime::from(dt);
                        date_str = local_dt.format("%Y-%m-%dT%H:%M:%S.%3f").to_string(); // ISO-ish
                        time_str = local_dt.format("%H:%M:%S.%3f").to_string();
                    }
                } else {
                    // Fallback to current time if sync not yet established
                    let now = chrono::Local::now();
                    date_str = now.format("%Y-%m-%dT%H:%M:%S.%3f").to_string();
                    time_str = now.format("%H:%M:%S.%3f").to_string();
                }
            }

            let passing = Passing {
                passing_number: "".to_string(), // Not in standard format
                transponder,
                date: date_str,
                time: time_str,
                event_id: "".to_string(),
                hits: parts[3].to_string(), // Hex
                max_rssi: parts[4].to_string(), // Hex
                internal_data: parts[11].to_string(),
                is_active: "".to_string(), // Could derive from InternalActiveData
                channel: parts[9].to_string(),
                loop_id: parts[8].to_string(),
                loop_id_wakeup: "".to_string(),
                battery: parts[5].to_string(),
                temperature: parts[6].to_string(),
                internal_active_data: parts[10].to_string(),
                box_temp: "".to_string(),
                box_reader_id: "".to_string(),
            };
            
            println!("Passing: {:?}", passing);
            if let Err(e) = tx.send(WsMessage::Passing(passing)) {
                eprintln!("Error broadcasting passing: {}", e);
            }
        }
    }
}
