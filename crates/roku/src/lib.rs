//! Roku devices over the External Control Protocol (ECP): HTTP on port
//! [`PORT`] of the device, no account and no cloud.
//!
//! Every call is one short request with a timeout; nothing is kept between
//! calls, so a device that changes address is reached at its new one as soon
//! as the caller asks there.

use std::net::IpAddr;
use std::time::Duration;

/// The port ECP answers on.
pub const PORT: u16 = 8060;

/// What a device says about itself (`query/device-info`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeviceInfo {
    /// The name its owner gave it, or its default name.
    pub name: String,
    pub model: String,
    pub serial: String,
    /// `PowerOn`, `DisplayOff`, `Headless`, `Ready`… as the device says it.
    pub power_mode: String,
    /// Whether it is a TV (which can be switched off) rather than a player.
    pub is_tv: bool,
}

impl DeviceInfo {
    /// Whether the screen is on: a TV in `PowerOn`. A player has no screen of
    /// its own and reports `PowerOn` whenever it is awake.
    pub fn on(&self) -> bool {
        self.power_mode == "PowerOn"
    }
}

/// An installed app (channel).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct App {
    pub id: String,
    pub name: String,
}

/// A client with a timeout per request.
pub struct Roku {
    agent: ureq::Agent,
    port: u16,
}

impl Roku {
    pub fn new(timeout: Duration) -> Self {
        Self {
            agent: ureq::AgentBuilder::new().timeout(timeout).build(),
            port: PORT,
        }
    }

    /// Talk to devices on another port (a simulator).
    #[must_use]
    pub fn with_port(mut self, port: u16) -> Self {
        self.port = port;
        self
    }

    fn url(&self, address: IpAddr, path: &str) -> String {
        match address {
            IpAddr::V6(v6) => format!("http://[{v6}]:{}/{path}", self.port),
            IpAddr::V4(v4) => format!("http://{v4}:{}/{path}", self.port),
        }
    }

    fn get(&self, address: IpAddr, path: &str) -> Result<String, String> {
        self.agent
            .get(&self.url(address, path))
            .call()
            .map_err(|error| describe(address, error))?
            .into_string()
            .map_err(|error| format!("reading {address}'s answer: {error}"))
    }

    fn post(&self, address: IpAddr, path: &str) -> Result<(), String> {
        self.agent
            .post(&self.url(address, path))
            .send_string("")
            .map(|_| ())
            .map_err(|error| describe(address, error))
    }

    pub fn device_info(&self, address: IpAddr) -> Result<DeviceInfo, String> {
        parse_device_info(&self.get(address, "query/device-info")?)
    }

    /// The app on screen, if any (`None` on the home screen).
    pub fn active_app(&self, address: IpAddr) -> Result<Option<App>, String> {
        let xml = self.get(address, "query/active-app")?;
        Ok(parse_apps(&xml)?.into_iter().next())
    }

    pub fn apps(&self, address: IpAddr) -> Result<Vec<App>, String> {
        parse_apps(&self.get(address, "query/apps")?)
    }

    /// Press a remote key (`Home`, `Play`, `VolumeUp`, `PowerOn`, …).
    pub fn keypress(&self, address: IpAddr, key: &str) -> Result<(), String> {
        if key.is_empty() || !key.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
            return Err(format!("'{key}' is not a Roku key name"));
        }
        self.post(address, &format!("keypress/{key}"))
    }

    /// Turn a TV's screen on or off (a player ignores this).
    pub fn power(&self, address: IpAddr, on: bool) -> Result<(), String> {
        self.keypress(address, if on { "PowerOn" } else { "PowerOff" })
    }

    /// Open an app by id.
    pub fn launch(&self, address: IpAddr, app: &str) -> Result<(), String> {
        if app.is_empty() || !app.chars().all(|c| c.is_ascii_alphanumeric()) {
            return Err(format!("'{app}' is not a Roku app id"));
        }
        self.post(address, &format!("launch/{app}"))
    }
}

fn describe(address: IpAddr, error: ureq::Error) -> String {
    match error {
        ureq::Error::Status(403, _) => format!(
            "{address} refused: turn on \"Control by mobile apps\" (network access) in its settings"
        ),
        ureq::Error::Status(code, _) => format!("{address} answered {code}"),
        ureq::Error::Transport(transport) => format!("could not reach {address}: {transport}"),
    }
}

fn text_of(document: &roxmltree::Document, tag: &str) -> String {
    document
        .descendants()
        .find(|node| node.has_tag_name(tag))
        .and_then(|node| node.text())
        .unwrap_or_default()
        .trim()
        .to_string()
}

/// Read `query/device-info`.
pub fn parse_device_info(xml: &str) -> Result<DeviceInfo, String> {
    let document =
        roxmltree::Document::parse(xml).map_err(|_| "device info that is not XML".to_string())?;
    let user_name = text_of(&document, "user-device-name");
    let default_name = text_of(&document, "default-device-name");
    Ok(DeviceInfo {
        name: if user_name.is_empty() {
            default_name
        } else {
            user_name
        },
        model: text_of(&document, "model-name"),
        serial: text_of(&document, "serial-number"),
        power_mode: text_of(&document, "power-mode"),
        is_tv: text_of(&document, "is-tv") == "true",
    })
}

/// Read a list of `<app id="…">name</app>` (`query/apps`, `query/active-app`).
pub fn parse_apps(xml: &str) -> Result<Vec<App>, String> {
    let document =
        roxmltree::Document::parse(xml).map_err(|_| "an app list that is not XML".to_string())?;
    Ok(document
        .descendants()
        .filter(|node| node.has_tag_name("app"))
        .filter_map(|node| {
            Some(App {
                id: node.attribute("id")?.to_string(),
                name: node.text().unwrap_or_default().trim().to_string(),
            })
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{BufRead, BufReader, Write};

    #[test]
    fn device_info_says_its_name_and_whether_it_is_on() {
        let info = parse_device_info(
            "<device-info><serial-number>X00000000000</serial-number><model-name>Example TV</model-name>\
             <user-device-name>Living room</user-device-name><default-device-name>Roku TV</default-device-name>\
             <is-tv>true</is-tv><power-mode>PowerOn</power-mode></device-info>",
        )
        .expect("info");
        assert_eq!(info.name, "Living room");
        assert!(info.is_tv && info.on());
        let standby = parse_device_info(
            "<device-info><default-device-name>Roku TV</default-device-name><power-mode>DisplayOff</power-mode></device-info>",
        )
        .expect("info");
        assert_eq!(standby.name, "Roku TV", "no name given: the default");
        assert!(!standby.on());
        assert!(parse_device_info("not xml").is_err());
    }

    #[test]
    fn apps_and_the_active_one_are_read_by_id() {
        let apps = parse_apps(
            r#"<apps><app id="12" type="appl" version="1">Example</app><app id="837" type="appl" version="2">Other</app></apps>"#,
        )
        .expect("apps");
        assert_eq!(apps.len(), 2);
        assert_eq!(
            apps[1],
            App {
                id: "837".into(),
                name: "Other".into()
            }
        );
        let home = parse_apps(r#"<active-app><app>Roku</app></active-app>"#).expect("home");
        assert!(home.is_empty(), "the home screen names no app id");
    }

    #[test]
    fn a_key_or_app_that_could_reach_another_path_is_refused_before_sending() {
        let roku = Roku::new(Duration::from_millis(200));
        let here: IpAddr = "127.0.0.1".parse().expect("ip");
        assert!(roku.keypress(here, "Home/../launch").is_err());
        assert!(roku.keypress(here, "").is_err());
        assert!(roku.launch(here, "12?x=1").is_err());
    }

    /// A device answering one request: the path asked is sent back.
    fn device(answer: &'static str) -> (u16, std::thread::JoinHandle<String>) {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind");
        let port = listener.local_addr().expect("addr").port();
        let handle = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("accept");
            let mut reader = BufReader::new(stream.try_clone().expect("clone"));
            let mut request = String::new();
            reader.read_line(&mut request).expect("request line");
            loop {
                let mut line = String::new();
                reader.read_line(&mut line).expect("header");
                if line == "\r\n" || line.is_empty() {
                    break;
                }
            }
            write!(
                stream,
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{answer}",
                answer.len()
            )
            .expect("answer");
            request
        });
        (port, handle)
    }

    #[test]
    fn a_press_and_a_query_go_to_the_ecp_paths() {
        let here: IpAddr = "127.0.0.1".parse().expect("ip");
        let (port, asked) = device("");
        Roku::new(Duration::from_secs(2))
            .with_port(port)
            .power(here, false)
            .expect("pressed");
        assert!(
            asked
                .join()
                .expect("device")
                .starts_with("POST /keypress/PowerOff ")
        );
        let (port, asked) = device(
            "<device-info><power-mode>PowerOn</power-mode><is-tv>true</is-tv></device-info>",
        );
        let info = Roku::new(Duration::from_secs(2))
            .with_port(port)
            .device_info(here)
            .expect("info");
        assert!(info.on());
        assert!(
            asked
                .join()
                .expect("device")
                .starts_with("GET /query/device-info ")
        );
    }
}
