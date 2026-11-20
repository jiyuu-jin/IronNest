//! Govee lights over Govee's LAN API: JSON over UDP on the home network, no
//! account and no cloud. The device must have "LAN Control" switched on in
//! the Govee app.
//!
//! Three ports:
//!
//! - a scan goes to the multicast group [`MULTICAST`] on [`SCAN_PORT`];
//! - commands go to the device on [`CONTROL_PORT`];
//! - **every answer** — to a scan or to `devStatus` — comes back to
//!   [`LISTEN_PORT`] on the asking machine, whatever port the question left
//!   from. So each call binds that port for its duration (and sends from it),
//!   and only one caller on a machine can be asking at once.
//!
//! Every call is one short exchange with a timeout; nothing is kept between
//! calls, so a device that changes address is reached at its new one as soon
//! as the caller asks there.

use std::net::{IpAddr, Ipv4Addr, SocketAddr, UdpSocket};
use std::time::{Duration, Instant};

use serde_json::{Value, json};

/// The multicast group a scan is sent to.
pub const MULTICAST: Ipv4Addr = Ipv4Addr::new(239, 255, 255, 250);
/// The port a scan is sent to.
pub const SCAN_PORT: u16 = 4001;
/// The port every answer comes back to, on the asking machine.
pub const LISTEN_PORT: u16 = 4002;
/// The port a device takes commands on.
pub const CONTROL_PORT: u16 = 4003;

/// The largest answer read; a device's are a few hundred bytes.
const DATAGRAM: usize = 2048;

/// A device that answered a scan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Found {
    pub ip: IpAddr,
    /// Its device id as the device says it: eight colon-separated bytes, the
    /// last six of which are its MAC (`7C:0C:5F:…`), or sometimes the six
    /// alone.
    pub device: String,
    /// Its model, like `H6076`.
    pub sku: String,
}

impl Found {
    /// The MAC in its device id, upper-case and colon-separated.
    pub fn mac(&self) -> Option<String> {
        mac_of(&self.device)
    }
}

/// The MAC in a device id: its last six bytes (of six or eight), upper-case
/// and colon-separated. `None` for anything else.
pub fn mac_of(device: &str) -> Option<String> {
    let bytes: Vec<&str> = device.trim().split([':', '-']).collect();
    if !(bytes.len() == 6 || bytes.len() == 8)
        || !bytes
            .iter()
            .all(|byte| byte.len() == 2 && byte.chars().all(|c| c.is_ascii_hexdigit()))
    {
        return None;
    }
    Some(bytes[bytes.len() - 6..].join(":").to_ascii_uppercase())
}

/// A colour.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Rgb {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

/// What a device says about itself (`devStatus`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Status {
    pub on: bool,
    /// In percent, 1–100 (0 when it did not say).
    pub brightness: u8,
    pub color: Rgb,
    /// The white colour temperature in kelvin; 0 while it shows a colour.
    pub kelvin: u32,
}

/// The kelvin range accepted for a white colour temperature.
pub const KELVIN: std::ops::RangeInclusive<u32> = 2000..=9000;

fn message(cmd: &str, data: Value) -> String {
    json!({"msg": {"cmd": cmd, "data": data}}).to_string()
}

/// The scan question.
pub fn scan_message() -> String {
    message("scan", json!({"account_topic": "reserve"}))
}

/// Switch on or off.
pub fn turn_message(on: bool) -> String {
    message("turn", json!({"value": u8::from(on)}))
}

/// Set the brightness, in percent from 1 to 100.
pub fn brightness_message(percent: u32) -> Result<String, String> {
    if !(1..=100).contains(&percent) {
        return Err(format!("brightness is 1 to 100 percent, not {percent}"));
    }
    Ok(message("brightness", json!({"value": percent})))
}

/// Show a colour (`colorwc` with no colour temperature).
pub fn color_message(color: Rgb) -> String {
    message(
        "colorwc",
        json!({"color": {"r": color.r, "g": color.g, "b": color.b}, "colorTemInKelvin": 0}),
    )
}

/// Show white at a colour temperature (`colorwc` with the colour ignored).
pub fn color_temperature_message(kelvin: u32) -> Result<String, String> {
    if !KELVIN.contains(&kelvin) {
        return Err(format!(
            "colour temperature is {} to {} kelvin, not {kelvin}",
            KELVIN.start(),
            KELVIN.end()
        ));
    }
    Ok(message(
        "colorwc",
        json!({"color": {"r": 0, "g": 0, "b": 0}, "colorTemInKelvin": kelvin}),
    ))
}

/// The status question.
pub fn status_message() -> String {
    message("devStatus", json!({}))
}

/// An answer's command and data, or why it is not one.
fn read(answer: &[u8]) -> Result<(String, Value), String> {
    let value: Value =
        serde_json::from_slice(answer).map_err(|_| "an answer that is not JSON".to_string())?;
    let msg = value
        .get("msg")
        .ok_or_else(|| "an answer with no msg".to_string())?;
    let cmd = msg
        .get("cmd")
        .and_then(Value::as_str)
        .ok_or_else(|| "an answer with no cmd".to_string())?;
    Ok((
        cmd.to_string(),
        msg.get("data").cloned().unwrap_or(Value::Null),
    ))
}

fn number(data: &Value, key: &str) -> u64 {
    data.get(key).and_then(Value::as_u64).unwrap_or_default()
}

/// Read an answer to a scan.
pub fn parse_scan(answer: &[u8]) -> Result<Found, String> {
    let (cmd, data) = read(answer)?;
    if cmd != "scan" {
        return Err(format!("a '{cmd}' answer, not a scan's"));
    }
    let ip = data
        .get("ip")
        .and_then(Value::as_str)
        .and_then(|ip| ip.parse().ok())
        .ok_or_else(|| "a scan answer with no address".to_string())?;
    let text = |key: &str| {
        data.get(key)
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string()
    };
    Ok(Found {
        ip,
        device: text("device"),
        sku: text("sku"),
    })
}

/// Read an answer to `devStatus`.
pub fn parse_status(answer: &[u8]) -> Result<Status, String> {
    let (cmd, data) = read(answer)?;
    if cmd != "devStatus" {
        return Err(format!("a '{cmd}' answer, not a status"));
    }
    let on = data
        .get("onOff")
        .and_then(Value::as_u64)
        .ok_or_else(|| "a status that does not say whether it is on".to_string())?;
    let color = data.get("color").cloned().unwrap_or(Value::Null);
    let channel = |key: &str| number(&color, key).min(255) as u8;
    Ok(Status {
        on: on == 1,
        brightness: number(&data, "brightness").min(100) as u8,
        color: Rgb {
            r: channel("r"),
            g: channel("g"),
            b: channel("b"),
        },
        kelvin: number(&data, "colorTemInKelvin") as u32,
    })
}

/// A client with a timeout per call.
#[derive(Debug, Clone)]
pub struct Govee {
    timeout: Duration,
    scan_to: SocketAddr,
    listen: SocketAddr,
    control_port: u16,
}

impl Govee {
    pub fn new(timeout: Duration) -> Self {
        Self {
            timeout,
            scan_to: SocketAddr::new(MULTICAST.into(), SCAN_PORT),
            listen: SocketAddr::new(Ipv4Addr::UNSPECIFIED.into(), LISTEN_PORT),
            control_port: CONTROL_PORT,
        }
    }

    /// Scan somewhere other than the multicast group (one device's address
    /// on [`SCAN_PORT`], or a simulator).
    #[must_use]
    pub fn with_scan_to(mut self, scan_to: SocketAddr) -> Self {
        self.scan_to = scan_to;
        self
    }

    /// Listen for answers somewhere else, and send commands to another port
    /// (a simulator).
    #[must_use]
    pub fn with_ports(mut self, listen: SocketAddr, control_port: u16) -> Self {
        self.listen = listen;
        self.control_port = control_port;
        self
    }

    /// The socket answers come back to: bound for the length of one call.
    fn listener(&self) -> Result<UdpSocket, String> {
        UdpSocket::bind(self.listen).map_err(|error| {
            format!(
                "listening for Govee answers on port {}: {error}",
                self.listen.port()
            )
        })
    }

    fn send(&self, address: IpAddr, text: &str) -> Result<(), String> {
        let socket = self.listener()?;
        socket
            .send_to(text.as_bytes(), SocketAddr::new(address, self.control_port))
            .map(|_| ())
            .map_err(|error| format!("could not reach {address}: {error}"))
    }

    /// Every device that answers a scan within the timeout (the whole timeout
    /// is waited, since any number may answer).
    pub fn scan(&self) -> Result<Vec<Found>, String> {
        self.scan_hosts(&[])
    }

    /// [`Govee::scan`], with the scan also sent to each of `hosts` directly
    /// on [`SCAN_PORT`]: where multicast does not reach (a bridged container,
    /// a network that drops it), a device still answers a scan sent to its
    /// own address. One datagram per host; a host that is not a Govee device
    /// ignores it.
    pub fn scan_hosts(&self, hosts: &[IpAddr]) -> Result<Vec<Found>, String> {
        let socket = self.listener()?;
        let group = socket.send_to(scan_message().as_bytes(), self.scan_to);
        let mut sent_any = group.is_ok();
        for host in hosts {
            sent_any |= socket
                .send_to(
                    scan_message().as_bytes(),
                    SocketAddr::new(*host, self.scan_to.port()),
                )
                .is_ok();
        }
        if !sent_any {
            group.map_err(|error| format!("sending a scan to {}: {error}", self.scan_to))?;
        }
        let mut found: Vec<Found> = Vec::new();
        let deadline = Instant::now() + self.timeout;
        let mut buffer = [0u8; DATAGRAM];
        while let Some(left) = deadline.checked_duration_since(Instant::now()) {
            if left.is_zero() {
                break;
            }
            socket
                .set_read_timeout(Some(left))
                .map_err(|error| error.to_string())?;
            let read = match socket.recv_from(&mut buffer) {
                Ok((read, _)) => read,
                Err(error)
                    if matches!(
                        error.kind(),
                        std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
                    ) =>
                {
                    break;
                }
                // A host's "nothing here" for one of the direct scans.
                Err(_) => continue,
            };
            if let Ok(one) = parse_scan(&buffer[..read])
                && !found
                    .iter()
                    .any(|seen| seen.device == one.device && seen.ip == one.ip)
            {
                found.push(one);
            }
        }
        Ok(found)
    }

    /// Switch on or off. The device does not answer; [`Govee::status`] says
    /// whether it took.
    pub fn turn(&self, address: IpAddr, on: bool) -> Result<(), String> {
        self.send(address, &turn_message(on))
    }

    /// Set the brightness, in percent from 1 to 100.
    pub fn brightness(&self, address: IpAddr, percent: u32) -> Result<(), String> {
        self.send(address, &brightness_message(percent)?)
    }

    /// Show a colour.
    pub fn color(&self, address: IpAddr, color: Rgb) -> Result<(), String> {
        self.send(address, &color_message(color))
    }

    /// Show white at a colour temperature, in kelvin ([`KELVIN`]).
    pub fn color_temperature(&self, address: IpAddr, kelvin: u32) -> Result<(), String> {
        self.send(address, &color_temperature_message(kelvin)?)
    }

    /// Ask the device how it is. Only an answer from `address` counts.
    pub fn status(&self, address: IpAddr) -> Result<Status, String> {
        let socket = self.listener()?;
        socket
            .send_to(
                status_message().as_bytes(),
                SocketAddr::new(address, self.control_port),
            )
            .map_err(|error| format!("could not reach {address}: {error}"))?;
        let deadline = Instant::now() + self.timeout;
        let mut buffer = [0u8; DATAGRAM];
        loop {
            let left = deadline
                .checked_duration_since(Instant::now())
                .filter(|left| !left.is_zero())
                .ok_or_else(|| no_answer(address, self.timeout))?;
            socket
                .set_read_timeout(Some(left))
                .map_err(|error| error.to_string())?;
            let (read, from) = socket
                .recv_from(&mut buffer)
                .map_err(|_| no_answer(address, self.timeout))?;
            if from.ip() != address {
                continue;
            }
            if let Ok(status) = parse_status(&buffer[..read]) {
                return Ok(status);
            }
        }
    }
}

fn no_answer(address: IpAddr, timeout: Duration) -> String {
    format!(
        "{address} did not answer within {} ms (is LAN Control on in the Govee app?)",
        timeout.as_millis()
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parsed(text: &str) -> Value {
        serde_json::from_str(text).expect("json")
    }

    #[test]
    fn commands_are_the_lan_api_messages() {
        assert_eq!(
            parsed(&turn_message(true)),
            json!({"msg": {"cmd": "turn", "data": {"value": 1}}})
        );
        assert_eq!(
            parsed(&turn_message(false)),
            json!({"msg": {"cmd": "turn", "data": {"value": 0}}})
        );
        assert_eq!(
            parsed(&brightness_message(40).expect("in range")),
            json!({"msg": {"cmd": "brightness", "data": {"value": 40}}})
        );
        assert_eq!(
            parsed(&color_message(Rgb {
                r: 255,
                g: 10,
                b: 0
            })),
            json!({"msg": {"cmd": "colorwc", "data": {
                "color": {"r": 255, "g": 10, "b": 0}, "colorTemInKelvin": 0}}})
        );
        assert_eq!(
            parsed(&color_temperature_message(2700).expect("in range")),
            json!({"msg": {"cmd": "colorwc", "data": {
                "color": {"r": 0, "g": 0, "b": 0}, "colorTemInKelvin": 2700}}})
        );
        assert_eq!(
            parsed(&status_message()),
            json!({"msg": {"cmd": "devStatus", "data": {}}})
        );
        assert_eq!(
            parsed(&scan_message()),
            json!({"msg": {"cmd": "scan", "data": {"account_topic": "reserve"}}})
        );
    }

    #[test]
    fn out_of_range_values_are_refused_before_sending() {
        assert!(brightness_message(0).is_err());
        assert!(brightness_message(101).is_err());
        assert!(brightness_message(1).is_ok() && brightness_message(100).is_ok());
        assert!(color_temperature_message(1999).is_err());
        assert!(color_temperature_message(9001).is_err());
        let govee = Govee::new(Duration::from_millis(100));
        let here: IpAddr = "127.0.0.1".parse().expect("ip");
        assert!(govee.brightness(here, 0).is_err());
    }

    #[test]
    fn a_scan_answer_says_where_and_what_and_its_mac() {
        let found = parse_scan(
            br#"{"msg":{"cmd":"scan","data":{"ip":"192.168.1.23","device":"1f:80:aa:bb:cc:dd:ee:01","sku":"H6076","bleVersionHard":"3.01.01","wifiVersionSoft":"1.02.03"}}}"#,
        )
        .expect("found");
        assert_eq!(found.ip.to_string(), "192.168.1.23");
        assert_eq!(found.sku, "H6076");
        assert_eq!(found.mac().as_deref(), Some("AA:BB:CC:DD:EE:01"));
        assert!(parse_scan(br#"{"msg":{"cmd":"scan","data":{"device":"x"}}}"#).is_err());
        assert!(parse_scan(br#"{"msg":{"cmd":"devStatus","data":{}}}"#).is_err());
        assert!(parse_scan(b"not json").is_err());
    }

    #[test]
    fn a_mac_is_the_last_six_bytes_of_a_device_id() {
        assert_eq!(
            mac_of("00:11:aa:bb:cc:dd:ee:ff").as_deref(),
            Some("AA:BB:CC:DD:EE:FF")
        );
        assert_eq!(
            mac_of("aa-bb-cc-dd-ee-ff").as_deref(),
            Some("AA:BB:CC:DD:EE:FF")
        );
        assert_eq!(mac_of("aa:bb:cc:dd:ee"), None);
        assert_eq!(mac_of("zz:bb:cc:dd:ee:ff"), None);
        assert_eq!(mac_of(""), None);
    }

    #[test]
    fn a_status_says_on_brightness_and_colour() {
        let status = parse_status(
            br#"{"msg":{"cmd":"devStatus","data":{"onOff":1,"brightness":64,"color":{"r":255,"g":128,"b":0},"colorTemInKelvin":0}}}"#,
        )
        .expect("status");
        assert_eq!(
            status,
            Status {
                on: true,
                brightness: 64,
                color: Rgb {
                    r: 255,
                    g: 128,
                    b: 0
                },
                kelvin: 0
            }
        );
        let off = parse_status(
            br#"{"msg":{"cmd":"devStatus","data":{"onOff":0,"brightness":10,"colorTemInKelvin":3000}}}"#,
        )
        .expect("status");
        assert!(!off.on);
        assert_eq!(off.kelvin, 3000);
        assert!(
            parse_status(br#"{"msg":{"cmd":"devStatus","data":{"brightness":10}}}"#).is_err(),
            "no onOff: not a status to trust"
        );
        assert!(parse_status(br#"{"msg":{"cmd":"scan","data":{"onOff":1}}}"#).is_err());
    }

    /// A free UDP port on loopback (bound and released).
    fn free_port() -> u16 {
        UdpSocket::bind("127.0.0.1:0")
            .expect("bind")
            .local_addr()
            .expect("addr")
            .port()
    }

    /// A light on loopback: answers scans on one socket and commands on
    /// another, sending every answer to `listen` on the asker — as a real one
    /// does, whatever port the question came from. Returns its two ports and
    /// what it was sent.
    fn light(listen: u16, questions: usize) -> (u16, u16, std::thread::JoinHandle<Vec<Value>>) {
        let scan = UdpSocket::bind("127.0.0.1:0").expect("bind");
        let control = UdpSocket::bind("127.0.0.1:0").expect("bind");
        let scan_port = scan.local_addr().expect("addr").port();
        let control_port = control.local_addr().expect("addr").port();
        scan.set_nonblocking(true).expect("nonblocking");
        control.set_nonblocking(true).expect("nonblocking");
        let handle = std::thread::spawn(move || {
            let mut heard = Vec::new();
            let mut on = false;
            let mut brightness = 50u64;
            let mut buffer = [0u8; DATAGRAM];
            let deadline = Instant::now() + Duration::from_secs(10);
            while heard.len() < questions && Instant::now() < deadline {
                for socket in [&scan, &control] {
                    let Ok((read, from)) = socket.recv_from(&mut buffer) else {
                        continue;
                    };
                    let question: Value =
                        serde_json::from_slice(&buffer[..read]).expect("json question");
                    let reply_to = SocketAddr::new(from.ip(), listen);
                    let data = &question["msg"]["data"];
                    let answer = match question["msg"]["cmd"].as_str().expect("cmd") {
                        "scan" => Some(json!({"msg": {"cmd": "scan", "data": {
                            "ip": "127.0.0.1", "device": "12:34:aa:bb:cc:dd:ee:01", "sku": "H6076"}}})),
                        "turn" => {
                            on = data["value"] == 1;
                            None
                        }
                        "brightness" => {
                            brightness = data["value"].as_u64().expect("value");
                            None
                        }
                        "devStatus" => Some(json!({"msg": {"cmd": "devStatus", "data": {
                            "onOff": u8::from(on), "brightness": brightness,
                            "color": {"r": 1, "g": 2, "b": 3}, "colorTemInKelvin": 0}}})),
                        _ => None,
                    };
                    if let Some(answer) = answer {
                        socket
                            .send_to(answer.to_string().as_bytes(), reply_to)
                            .expect("answer");
                    }
                    heard.push(question);
                }
                std::thread::sleep(Duration::from_millis(5));
            }
            heard
        });
        (scan_port, control_port, handle)
    }

    #[test]
    fn a_light_is_found_switched_dimmed_and_asked() {
        let listen = free_port();
        let (scan_port, control_port, light) = light(listen, 5);
        let here: IpAddr = "127.0.0.1".parse().expect("ip");
        let govee = Govee::new(Duration::from_millis(500))
            .with_scan_to(SocketAddr::new(here, scan_port))
            .with_ports(SocketAddr::new(here, listen), control_port);
        let found = govee.scan().expect("scanned");
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].mac().as_deref(), Some("AA:BB:CC:DD:EE:01"));
        assert_eq!(found[0].sku, "H6076");
        govee.turn(here, true).expect("sent");
        govee.brightness(here, 30).expect("sent");
        // UDP has no order guarantee across sockets in principle; on
        // loopback the light reads them in turn.
        std::thread::sleep(Duration::from_millis(50));
        let status = govee.status(here).expect("answered");
        assert!(status.on);
        assert_eq!(status.brightness, 30);
        assert_eq!(status.color, Rgb { r: 1, g: 2, b: 3 });
        govee.turn(here, false).expect("sent");
        let heard = light.join().expect("light");
        let commands: Vec<&str> = heard
            .iter()
            .map(|question| question["msg"]["cmd"].as_str().expect("cmd"))
            .collect();
        assert_eq!(
            commands,
            ["scan", "turn", "brightness", "devStatus", "turn"]
        );
    }

    #[test]
    fn a_scan_sent_to_each_host_finds_a_light_multicast_does_not_reach() {
        let listen = free_port();
        let (scan_port, _, light) = light(listen, 1);
        let here: IpAddr = "127.0.0.1".parse().expect("ip");
        // The "group" is somewhere nothing answers.
        let govee = Govee::new(Duration::from_millis(400))
            .with_scan_to(SocketAddr::new("127.0.0.2".parse().expect("ip"), scan_port))
            .with_ports(SocketAddr::new(here, listen), CONTROL_PORT);
        let found = govee
            .scan_hosts(&["127.0.0.3".parse().expect("ip"), here])
            .expect("scanned");
        assert_eq!(found.len(), 1, "{found:?}");
        assert_eq!(found[0].ip, here);
        light.join().expect("light");
    }

    #[test]
    fn a_light_that_never_answers_is_given_up_on_at_the_timeout() {
        let listen = free_port();
        // Bound and silent.
        let silent = UdpSocket::bind("127.0.0.1:0").expect("bind");
        let here: IpAddr = "127.0.0.1".parse().expect("ip");
        let govee = Govee::new(Duration::from_millis(200)).with_ports(
            SocketAddr::new(here, listen),
            silent.local_addr().expect("addr").port(),
        );
        let started = Instant::now();
        let error = govee.status(here).expect_err("no answer");
        assert!(error.contains("did not answer"), "{error}");
        assert!(started.elapsed() < Duration::from_secs(2));
        // Nobody to answer a scan either: an empty list, not an error.
        let govee = govee.with_scan_to(SocketAddr::new(here, free_port()));
        assert_eq!(govee.scan().expect("scanned"), Vec::new());
    }

    #[test]
    fn an_answer_from_another_address_is_not_taken_for_the_lights() {
        let listen = free_port();
        let here: IpAddr = "127.0.0.1".parse().expect("ip");
        let impostor = UdpSocket::bind("127.0.0.2:0");
        let Ok(impostor) = impostor else {
            // No 127.0.0.2 on this machine.
            return;
        };
        let silent = UdpSocket::bind("127.0.0.1:0").expect("bind");
        let control_port = silent.local_addr().expect("addr").port();
        let sender = std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(50));
            let answer = br#"{"msg":{"cmd":"devStatus","data":{"onOff":1,"brightness":5}}}"#;
            impostor
                .send_to(
                    answer,
                    SocketAddr::new("127.0.0.1".parse().expect("ip"), listen),
                )
                .expect("sent");
        });
        let govee = Govee::new(Duration::from_millis(300))
            .with_ports(SocketAddr::new(here, listen), control_port);
        assert!(govee.status(here).is_err());
        sender.join().expect("sender");
    }
}
