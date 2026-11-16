//! Sign in and list the account's cameras with their latest events.
//!
//! `RING_EMAIL=… RING_PASSWORD=… RING_SESSION=session.json cargo run --example cameras`
//! signs in (or resumes the session kept in `RING_SESSION`, which is
//! rewritten after every run since Ring rotates it); when Ring sends a code,
//! run it again with the code as the argument.

fn main() -> Result<(), String> {
    let path = std::env::var("RING_SESSION").map_err(|_| "set RING_SESSION to a file")?;
    let saved: Option<ring_cloud::Session> = std::fs::read_to_string(&path)
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok());
    let session = match saved {
        Some(session) if !session.refresh_token.is_empty() => session,
        kept => {
            let email = std::env::var("RING_EMAIL").map_err(|_| "set RING_EMAIL")?;
            let password = std::env::var("RING_PASSWORD").map_err(|_| "set RING_PASSWORD")?;
            // The hardware id must be the same for the call that asks for a
            // code and the one that sends it, so it is kept between them.
            let hardware_id = kept
                .map(|session| session.hardware_id)
                .unwrap_or_else(ring_cloud::new_hardware_id);
            let pending = ring_cloud::Session {
                refresh_token: String::new(),
                hardware_id: hardware_id.clone(),
            };
            let write = |session: &ring_cloud::Session| {
                let text = serde_json::to_string(session).map_err(|error| error.to_string())?;
                std::fs::write(&path, text).map_err(|error| error.to_string())
            };
            write(&pending)?;
            let code = std::env::args().nth(1);
            match ring_cloud::sign_in(&email, &password, code.as_deref(), &hardware_id)? {
                ring_cloud::SignIn::NeedsCode(prompt) => {
                    println!("{prompt} Run again with the code.");
                    return Ok(());
                }
                ring_cloud::SignIn::Done(session) => {
                    write(&session)?;
                    session
                }
            }
        }
    };
    let mut ring = ring_cloud::Ring::resume(session);
    let cameras = ring.cameras();
    let text = serde_json::to_string(&ring.session()).map_err(|error| error.to_string())?;
    std::fs::write(&path, text).map_err(|error| error.to_string())?;
    for camera in cameras? {
        println!("{}  {}  {}", camera.id, camera.kind, camera.name);
        for event in ring.events(&camera, 4)? {
            println!(
                "    {}  {}  person {}  {:?}",
                event.created_at, event.kind, event.person, event.detection
            );
        }
    }
    let text = serde_json::to_string(&ring.session()).map_err(|error| error.to_string())?;
    std::fs::write(&path, text).map_err(|error| error.to_string())
}
