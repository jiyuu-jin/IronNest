# eufy

A blocking client for the eufy security cloud (the "mega" `algo_ecdh` API):

- sign in with email and password, including the emailed two-factor code
  and the captcha challenge, and keep/resume the session (`Session`);
- list the account's devices with their reported parameters;
- read a lock's state (`Device::locked`, parameter 6000) and battery;
- lock and unlock a smart lock over eufy's P2P protocol
  (`Cloud::set_locked`).

```rust
let mut cloud = eufy::Cloud::new(eufy::Config::new("you@example.com", "password"));
match cloud.login()? {
    eufy::Login::Done => {}
    eufy::Login::CodeSent => { cloud.submit_code("123456")?; }
    eufy::Login::Captcha { .. } => return Err("answer the captcha".into()),
}
for device in cloud.devices()? {
    println!("{} locked: {:?}", device.serial, device.locked());
}
```

## Locking

```rust
let lock = cloud.devices()?.into_iter().find(|d| d.serial == serial).unwrap();
let outcome: eufy::Actuation = cloud.set_locked(&lock, true)?; // false unlocks
```

`set_locked(&mut self, device: &Device, locked: bool) -> Result<Actuation, String>`
blocks (up to about a minute and a half) while it:

1. picks the lock's wire by family (`eufy::lock::lock_wire`): the shared
   `ff09` frame for every lock except the Video Smart Lock S330 and the
   classic `T8520` Wi-Fi lock, whose own commands are **not ported** and
   are refused; a lock with no P2P id (driven over the cloud's MQTT broker)
   is refused too;
2. fetches the station's DSK key and reaches the device over P2P
   (`eufy::p2p::Session`): local and cloud lookups, hole punch, or the
   PPCS relay;
3. negotiates the level-2 session key from the device's gateway info,
   fetching the cipher's private key from the cloud (`Cloud::cipher_keys`,
   ECC or RSA);
4. sends the `ff09` lock/unlock frame (`eufy::ff09`) in a level-2 sealed
   `SET_PAYLOAD` envelope (`cmd` 1940, `apiCommand` 6018), five times
   200 ms apart;
5. answers `Ok` once the device has acknowledged delivery, carrying any
   bare result code or `ff09` reply it sent; a negative result code is an
   error.

The acting name in the frame is `Config::account_name` when set, else the
name the sign-in gave the account — the email's local part — which the
`Session` keeps, so a resumed session acts with no email in its `Config`.
A session kept before it carried the name falls back to the lock's own
`member` record (its `email` local part, else its `nick_name`). The
identity fields come from that `member` record.

**What is verified.** The SDK this is ported from verifies the `ff09` frame
byte-exact against captures from the T8531 video lock and the T85D0
garage door, and drives it over P2P on the T8531. For the Smart Lock C220 /
Touch & Wi-Fi (`T8506`) the `ff09` wire is that SDK's family fall-through,
not a capture; the SDK also reads no reply to an actuation, so what a
reply's status byte means is not established. Every pure piece here is
unit-tested (the `ff09` vector is the SDK's own); nothing in this crate's
tests talks to the network or to a lock. `examples/lock.rs` moves a real
lock when run.

The protocol is as documented, and verified against live traffic, by
[mega-yfue/eufy-sdk](https://github.com/mega-yfue/eufy-sdk) (Apache-2.0).
