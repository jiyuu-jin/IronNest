# TP-Link Smart Device Client Library

A pure Rust library for communicating with TP-Link smart devices including smart plugs, smart lights, smart dimmers, and power strips.

## Features

- **Device Discovery**: Find TP-Link devices on your network via UDP broadcast
- **Smart Plugs**: Control on/off state, set aliases, reboot
- **Smart Lights**: Control on/off, brightness, and color (HSL/RGB/CSS colors)
- **Smart Dimmers**: Control brightness and inactivity timeout
- **Power Strips**: Control individual sockets and monitor energy usage
- **Async/Await**: Full Tokio async support
- **Error Handling**: Comprehensive error types with proper error propagation
- **Pure Rust**: No external dependencies on system libraries

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
tplink = { path = "../crates/tplink" }
tokio = { version = "1", features = ["full"] }
```

## Quick Start

### Discover Devices

```rust
use tplink::{discover_devices, DeviceData};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let devices = discover_devices().await?;
    
    for device in devices {
        match device {
            DeviceData::SmartPlug(plug) => {
                println!("Smart Plug: {} ({})", plug.alias, plug.ip.unwrap());
            }
            DeviceData::SmartLight(light) => {
                println!("Smart Light: {} ({})", light.alias, light.ip.unwrap());
            }
            DeviceData::SmartDimmer(dimmer) => {
                println!("Smart Dimmer: {} ({})", dimmer.alias, dimmer.ip.unwrap());
            }
            DeviceData::SmartPowerStrip(strip) => {
                println!("Power Strip: {} ({})", strip.alias, strip.ip.unwrap());
            }
        }
    }
    Ok(())
}
```

### Control Devices

```rust
use tplink::TpLinkClient;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = TpLinkClient::new();
    
    // Smart Plug
    client.turn_plug_on("192.168.1.100").await?;
    client.turn_plug_off("192.168.1.100").await?;
    
    // Smart Light
    client.turn_light_on_off("192.168.1.101", true).await?;
    client.set_light_brightness("192.168.1.101", 75).await?;
    client.set_light_color("192.168.1.101", "#FF0000").await?; // Red
    client.set_light_color("192.168.1.101", "hsl(120, 50%, 50%)").await?; // Green
    
    // Smart Dimmer
    client.set_dimmer_brightness("192.168.1.102", 50).await?;
    
    // Power Strip
    client.turn_power_strip_socket_on("192.168.1.103", "socket_0").await?;
    let energy = client.get_power_strip_energy_usage("192.168.1.103", "socket_0").await?;
    println!("Energy usage: {:?}", energy);
    
    Ok(())
}
```

### Standalone Functions

For convenience, all device operations are also available as standalone functions:

```rust
use tplink::{turn_plug_on, set_light_brightness, set_dimmer_brightness};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    turn_plug_on("192.168.1.100").await?;
    set_light_brightness("192.168.1.101", 80).await?;
    set_dimmer_brightness("192.168.1.102", 60).await?;
    Ok(())
}
```

## Supported Devices

- **Smart Plugs**: HS100, HS110, KP100, etc.
- **Smart Lights**: LB100, LB110, LB120, LB130, etc.
- **Smart Dimmers**: ES20M(US), KS230(US), etc.
- **Power Strips**: HS300, KP303, etc.

## Protocol

This library implements the TP-Link smart device protocol which uses:
- UDP broadcasts on port 9999 for device discovery
- TCP connections on port 9999 for device control
- Simple XOR encryption with a rotating key (starting with 0xAB)

## Error Handling

The library provides comprehensive error types:

```rust
use tplink::{TpLinkError, Result};

match some_operation().await {
    Ok(result) => println!("Success: {:?}", result),
    Err(TpLinkError::Network(e)) => eprintln!("Network error: {}", e),
    Err(TpLinkError::InvalidIpAddress(ip)) => eprintln!("Invalid IP: {}", ip),
    Err(TpLinkError::InvalidColor(color)) => eprintln!("Invalid color: {}", color),
    Err(e) => eprintln!("Other error: {}", e),
}
```

## License

This project is licensed under either of
 * MIT License

at your option.


## Setup mode (`setup`)

A Kasa device in setup mode opens an open access point (`TP-LINK_…` or
`Kasa_…`) and answers at 192.168.0.1:9999. `tplink::setup` has the bytes to
read what it is (`read_setup_device`) and hand it a Wi-Fi network
(`hand_over`) once a client has joined that access point, with no account
and no cloud. Joining the access point needs a radio (see the `wifi` crate).
