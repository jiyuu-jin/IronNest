//! Lock or unlock a smart lock. THIS MOVES A REAL LOCK.
//!
//! `EUFY_EMAIL=… EUFY_PASSWORD=… EUFY_SESSION=session.json \
//!  cargo run --example lock -- <serial> lock|unlock`
//!
//! Uses the session kept by the `devices` example (sign in with that
//! first). Prints what the lock answered.

fn main() -> Result<(), String> {
    let mut args = std::env::args().skip(1);
    let (Some(serial), Some(verb)) = (args.next(), args.next()) else {
        return Err("usage: lock <serial> lock|unlock".into());
    };
    let locked = match verb.as_str() {
        "lock" => true,
        "unlock" => false,
        _ => return Err("the second argument is lock or unlock".into()),
    };
    let email = std::env::var("EUFY_EMAIL").unwrap_or_default();
    let password = std::env::var("EUFY_PASSWORD").unwrap_or_default();
    let path = std::env::var("EUFY_SESSION").map_err(|_| "set EUFY_SESSION")?;
    let text = std::fs::read_to_string(&path).map_err(|error| error.to_string())?;
    let session: eufy::Session = serde_json::from_str(&text).map_err(|error| error.to_string())?;
    let mut cloud = eufy::Cloud::resume(eufy::Config::new(&email, &password), session);
    let device = cloud
        .devices()?
        .into_iter()
        .find(|device| device.serial == serial)
        .ok_or("no such device on the account")?;
    let outcome = cloud.set_locked(&device, locked)?;
    println!("{outcome:?}");
    Ok(())
}
