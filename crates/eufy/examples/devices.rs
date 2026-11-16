//! Sign in and list the account's devices.
//!
//! `EUFY_EMAIL=… EUFY_PASSWORD=… EUFY_SESSION=session.json cargo run --example devices`
//! signs in (or resumes the session kept in `EUFY_SESSION`); when the cloud
//! sends a code, run it again with the code as the argument.

fn main() -> Result<(), String> {
    let email = std::env::var("EUFY_EMAIL").map_err(|_| "set EUFY_EMAIL")?;
    let password = std::env::var("EUFY_PASSWORD").map_err(|_| "set EUFY_PASSWORD")?;
    let kept = std::env::var("EUFY_SESSION").ok();
    let config = eufy::Config::new(&email, &password);
    let saved: Option<eufy::Session> = kept
        .as_ref()
        .and_then(|path| std::fs::read_to_string(path).ok())
        .and_then(|text| serde_json::from_str(&text).ok());
    let code = std::env::args().nth(1);
    let mut cloud = match &saved {
        Some(session) => eufy::Cloud::resume(config, session.clone()),
        None => eufy::Cloud::new(config),
    };
    let login = match (&code, &saved) {
        (Some(code), _) => cloud.submit_code(code)?,
        (None, Some(_)) => eufy::Login::Done,
        (None, None) => cloud.login()?,
    };
    if let (Some(path), Some(session)) = (&kept, cloud.session()) {
        let text = serde_json::to_string(&session).map_err(|error| error.to_string())?;
        std::fs::write(path, text).map_err(|error| error.to_string())?;
    }
    println!("sign-in: {login:?}");
    if login != eufy::Login::Done {
        return Ok(());
    }
    for device in cloud.devices()? {
        println!(
            "{}  type {}  {:?}  locked {:?}  battery {:?}  p2p {}",
            device.serial,
            device.device_type,
            device.name,
            device.locked(),
            device.battery(),
            device.p2p_did.is_some()
        );
    }
    Ok(())
}
