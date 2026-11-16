# eufy

A blocking client for the eufy security cloud (the "mega" `algo_ecdh` API):

- sign in with email and password, including the emailed two-factor code
  and the captcha challenge, and keep/resume the session (`Session`);
- list the account's devices with their reported parameters;
- read a lock's state (`Device::locked`, parameter 6000) and battery.

Lock commands are not implemented yet.

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

The protocol is as documented, and verified against live traffic, by
[mega-yfue/eufy-sdk](https://github.com/mega-yfue/eufy-sdk) (Apache-2.0).
