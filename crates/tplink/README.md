# TP-Link Smart Device Client Library

A pure Rust library for communicating with TP-Link smart devices including smart plugs, smart lights, smart dimmers, and power strips.

## Features

- **🔍 Device Discovery**: Automatic network discovery via UDP broadcast
- **🔧 Device Objects**: Type-safe device objects with embedded client functionality
- **🏠 Smart Home Control**: Support for plugs, lights, dimmers, and power strips
- **🎨 Color Control**: Full HSL/RGB/CSS color support for smart lights
- **⚡ Async/Await**: Built on Tokio for high-performance async operations
- **🛡️ Type Safety**: Compile-time prevention of calling wrong methods on wrong device types
- **📦 Zero Dependencies**: Pure Rust implementation with minimal external dependencies

## 🚀 Quick Start

Add this to your `Cargo.toml`:

```toml
[dependencies]
tplink = "0.1.0"
tokio = { version = "1", features = ["full"] }
```

**For local development or monorepo usage:**
```toml
[dependencies]
tplink = { path = "path/to/tplink" }
tokio = { version = "1", features = ["full"] }
```

## 📖 Basic Usage

### Device Discovery & Control

The new API provides device objects with embedded clients for type-safe operations:

```rust
use tplink::{discover, Device};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Discover all TP-Link devices on the network
    let devices = discover().await?;
    
    for device in devices {
        println!("Found device: {} at {}", device.alias(), device.ip_string().unwrap());
        
        match device {
            Device::SmartPlug(plug) => {
                println!("  Type: Smart Plug");
                plug.turn_on().await?;
                println!("  ✅ Turned on!");
            }
            Device::SmartLight(light) => {
                println!(" ✅ Type: Smart Light (Dimmable: {}, Color: {})", 
                    light.is_dimmable(), light.is_color());
                light.turn_on().await?;
                light.set_brightness(75).await?;
                light.set_color("#FF6B35").await?; // Set to orange
                println!("  Configured light!");
            }
            Device::SmartDimmer(dimmer) => {
                println!("  Type: Smart Dimmer");
                dimmer.set_brightness(50).await?;
                println!("  🔆 Set brightness!");
            }
            Device::PowerStrip(strip) => {
                println!("  Type: Power Strip ({} sockets)", strip.sockets().len());
                for socket in strip.sockets() {
                    strip.turn_socket_on(&socket.id).await?;
                    println!("    🔌 Socket '{}' turned on", socket.alias);
                }
            }
        }
    }
    Ok(())
}
```

### Using the Device Registry

For applications that need persistent device management:

```rust
use tplink::{create_registry, DeviceRegistry};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let registry = create_registry();
    
    // Discover and register devices
    {
        let mut reg = registry.write().await;
        let devices = reg.discover_and_register().await?;
        println!("Discovered {} devices", devices.len());
    }
    
    // Use devices by IP
    {
        let reg = registry.read().await;
        if let Some(plug) = reg.get_smart_plug("192.168.1.100") {
            plug.turn_on().await?;
            println!("Turned on plug at 192.168.1.100");
        }
        
        if let Some(light) = reg.get_smart_light("192.168.1.101") {
            light.set_brightness(100).await?;
            light.set_color("hsl(240, 100%, 50%)").await?; // Blue
            println!("Set light to bright blue");
        }
    }
    
    Ok(())
}
```

## 🎯 Device-Specific Operations

### Smart Plugs

```rust
use tplink::Device;

// Assuming you have a smart plug device
if let Device::SmartPlug(plug) = device {
    // Basic control
    plug.turn_on().await?;
    plug.turn_off().await?;
    
    // Device management
    plug.set_alias("Living Room Lamp").await?;
    plug.reboot().await?;
    
    // Get device info
    println!("Plug: {} ({})", plug.alias(), plug.device_id());
}
```

### Smart Lights

```rust
use tplink::Device;

if let Device::SmartLight(light) = device {
    // Power control
    light.turn_on().await?;
    light.turn_off().await?;
    
    // Brightness control (0-100)
    light.set_brightness(75).await?;
    
    // Color control - supports multiple formats
    light.set_color("#FF0000").await?;              // Hex
    light.set_color("red").await?;                  // Named colors
    light.set_color("rgb(255, 0, 0)").await?;      // RGB
    light.set_color("hsl(0, 100%, 50%)").await?;   // HSL
    
    // Check capabilities
    if light.is_color() {
        println!("This light supports color!");
    }
    if light.is_dimmable() {
        println!("This light is dimmable!");
    }
}
```

### Smart Dimmers

```rust
use tplink::Device;

if let Device::SmartDimmer(dimmer) = device {
    // Brightness control (0-100)
    dimmer.set_brightness(25).await?;
    
    // Set auto-off timeout (in minutes)
    dimmer.set_inactivity_timeout(30).await?;
}
```

### Power Strips

```rust
use tplink::Device;

if let Device::PowerStrip(strip) = device {
    // Get available sockets
    let sockets = strip.sockets();
    println!("Power strip has {} sockets", sockets.len());
    
    for socket in sockets {
        println!("Socket: {} (ID: {})", socket.alias, socket.id);
        
        // Control individual sockets
        strip.turn_socket_on(&socket.id).await?;
        strip.turn_socket_off(&socket.id).await?;
        
        // Energy monitoring
        let usage = strip.get_energy_usage(&socket.id).await?;
        let emeter = strip.get_emeter_data(&socket.id).await?;
        println!("Energy data: {:?}", usage);
    }
}
```

## 🔄 Migrating from Function-Based API

**Old way (function-based):**
```rust
// ❌ Old API - not type safe, easy to make mistakes
use tplink::{turn_plug_on, set_light_brightness};

turn_plug_on("192.168.1.100").await?;
set_light_brightness("192.168.1.100", 75).await?; // Wrong! This is a plug, not a light
```

**New way (device-based):**
```rust
// ✅ New API - type safe, self-documenting
use tplink::{discover, Device};

let devices = discover().await?;
for device in devices {
    match device {
        Device::SmartPlug(plug) => {
            plug.turn_on().await?;
            // plug.set_brightness(75).await?; // ❌ Compile error - plugs don't have brightness!
        }
        Device::SmartLight(light) => {
            light.turn_on().await?;
            light.set_brightness(75).await?; // ✅ Type safe!
        }
        _ => {}
    }
}
```

## 🔧 Advanced Usage

### Custom Client Configuration

```rust
use tplink::{TpLinkClient, DeviceRegistry};
use std::sync::Arc;

let client = Arc::new(TpLinkClient::new());
let registry = DeviceRegistry::with_client(client.clone());

// Use the same client across multiple registries or operations
let devices = tplink::discovery::discover_devices_with_client(client).await?;
```

### Error Handling

```rust
use tplink::{TpLinkError, Result};

match plug.turn_on().await {
    Ok(()) => println!("Success!"),
    Err(TpLinkError::Network(e)) => eprintln!("Network error: {}", e),
    Err(TpLinkError::InvalidIpAddress(ip)) => eprintln!("Invalid IP: {}", ip),
    Err(TpLinkError::DeviceCommunication { message }) => eprintln!("Device error: {}", message),
    Err(e) => eprintln!("Other error: {}", e),
}
```

## 📋 Supported Devices

| Device Type | Models | Features |
|-------------|---------|----------|
| **Smart Plugs** | HS100, HS110, KP100+ | On/Off, Scheduling, Energy Monitoring* |
| **Smart Lights** | LB100, LB110, LB120, LB130+ | On/Off, Dimming*, Color* |
| **Smart Dimmers** | ES20M(US), KS230(US) | Dimming, Inactivity Timer |
| **Power Strips** | HS300, KP303+ | Individual Socket Control, Energy Monitoring |

*Feature availability depends on specific model

## 🛠️ Protocol Details

TP-Link smart devices use:
- **Discovery**: UDP broadcast on port 9999
- **Communication**: TCP connections on port 9999  
- **Encryption**: XOR cipher with rotating key (0xAB initial)
- **Format**: JSON messages with 4-byte length prefix

## 🔍 Troubleshooting

**Device not found:**
- Ensure device is on the same network
- Check firewall settings for UDP port 9999
- Verify device is powered on and connected to WiFi

**Connection timeouts:**
- Device may be temporarily offline
- Check IP address is correct
- Network congestion can cause delays

**Color commands not working:**
- Verify the device supports color (`light.is_color()`)
- Use valid CSS color formats: `#RRGGBB`, `rgb()`, `hsl()`, named colors

## 🤝 Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## 📄 License

This project is licensed under either of

 * Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
 * MIT License ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.