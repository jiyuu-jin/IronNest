# govee

A blocking client for Govee lights over Govee's LAN API: JSON over UDP on the
home network, no account and no cloud.

- `scan` finds devices (a multicast to 239.255.255.250:4001; each answers on
  4002 with its address, device id and model); `scan_hosts` also sends it
  to each address given, for where multicast does not reach (a bridged
  container);
- `turn`, `brightness` (1–100), `color` (RGB), `color_temperature` (kelvin),
  sent to the device on 4003;
- `status` (`devStatus`): on/off, brightness, colour, colour temperature.

The device must have **LAN Control** switched on in the Govee app (the
device's settings page). Answers come back to port 4002 on the asking
machine, so one process at a time on that machine can ask.

```rust
let govee = govee::Govee::new(std::time::Duration::from_secs(2));
for found in govee.scan()? {
    println!("{} {} {}", found.ip, found.sku, found.device);
}
let light = "192.168.1.60".parse()?;
govee.turn(light, true)?;
govee.brightness(light, 40)?;
println!("{:?}", govee.status(light)?);
```
