# RRUSBConverter

RRUSBConverter is a tool for connecting to a Race Result USB Timing Box via a serial port and broadcasting passing data to a WebSocket server for use by web applications.

It is designed to be lightweight and fast, written in Rust.

## Features

- Connects to a Race Result USB Timing Box via serial port (e.g., `/dev/tty.usbserial-XXXX`).
- Handles protocol negotiation and time synchronization.
- Hosts a simple WebSocket server that emits passing data in real-time at `/ws`.
- Emits passing data in JSON format compatible with `rrclivelaps`.

## WebSocket Data Format

The WebSocket server emits JSON objects with the following structure:

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

## Running the Software

Download the latest release from the [releases](https://github.com/gillian/rrusbconverter/releases) page and run it with:

```sh
./rrusbconverter --serial-port <PORT>
```

### Options

- `--serial-port <PORT>`: **Required**. The serial port path to connect to (e.g., `/dev/tty.usbserial-10`, `/dev/ttyUSB0`, `COM3`).

## Built on

- [tokio](https://tokio.rs)
- [tokio-serial](https://docs.rs/tokio-serial)
- [warp](https://warp.rs)
- [serde](https://serde.rs)

## Dev Building Instructions

```sh
cargo build --release
```

## Dev Usage

```sh
cargo run -- --serial-port /dev/tty.usbserial-XXXX
```
