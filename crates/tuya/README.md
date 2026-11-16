# tuya

A blocking client for Tuya (Smart Life) devices over their local protocol
(3.3, 3.4, 3.5): switch a data point, set a light's brightness, read state,
and find addresses listening on the Tuya port.

- One kept session per device, with a heartbeat; a command on a quiet session
  goes out with no status query first.
- The protocol version is found by trying, newest first, and remembered.
- The local key is handed in by the caller and kept in memory only.

```rust
let mut tuya = tuya::Tuya::new(std::time::Duration::from_secs(5))?;
tuya.set_key("device-id", Ok("0123456789abcdef".to_string()));
tuya.switch("device-id", None, "192.168.1.50".parse()?, true)?;
tuya.brightness("device-id", "192.168.1.50".parse()?, 40)?;
```
