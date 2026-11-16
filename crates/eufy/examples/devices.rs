//! Sign in and list the account's devices:
//! `EUFY_EMAIL=… EUFY_PASSWORD=… cargo run --example devices [code]`.

fn main() -> Result<(), String> {
    let email = std::env::var("EUFY_EMAIL").map_err(|_| "set EUFY_EMAIL")?;
    let password = std::env::var("EUFY_PASSWORD").map_err(|_| "set EUFY_PASSWORD")?;
    let mut cloud = eufy::Cloud::new(eufy::Config::new(&email, &password));
    let login = match std::env::args().nth(1) {
        Some(code) => cloud.submit_code(&code)?,
        None => cloud.login()?,
    };
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
