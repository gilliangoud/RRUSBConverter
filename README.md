# RRConverter

RRConverter is a tool for converting RaceReady passing data to a format that can be used by simple javascript web applications.

It's written in rust for a small footprint and fast performance.

## Features

- Connects automatically to a Rase|Result decoder on the local network
- hosts a simple websocket server that emits passing data in real time at /ws
- emits passing data in JSON format

## Websocket Data Format

```json
{
    "passing_number": "1",
    "transponder": "12345",
    "date": "2023-10-27T10:00:00.000",
    "time": "10:00:00.000",
    "event_id": "123456",
    "hits": "10",
    "max_rssi": "-50",
    "internal_data": "0000",
    "is_active": "1",
    "channel": "1",
    "loop_id": "1",
    "loop_id_wakeup": "1",
    "battery": "3.0",
    "temperature": "25",
    "internal_active_data": "0000",
    "box_temp": "30",
    "box_reader_id": "0"
}
```

## Running the sofware
Download the latest release from the [releases](https://github.com/gillian/rrconverter/releases) page and run it with:

```sh
./rrconverter [OPTIONS]
```

### Options

- `--ip-range <PREFIX>`: Manually specify the subnet to scan (e.g., `192.168.1.`).
- `--decoder-ip <IP>`: Connect directly to a specific IP, skipping the scan.
- `--decoder-port <PORT>`: Use a different port (default: 3601).

## Built on

- [tokio](https://tokio.rs)
- [warp](https://warp.rs)
- [serde](https://serde.rs)
- [tokio-util](https://docs.rs/tokio-util)
- [timing](https://github.com/critrace/timing)

## Dev Building instructions

```sh
cargo build --release
```

## Dev Usage

```sh
cargo run
```

# RRUSBConverter
