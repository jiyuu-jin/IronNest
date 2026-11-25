//! A blocking P2P session with one device (or the station it hangs off).
//!
//! **Connect**: local lookups (broadcast and the device's LAN address, to
//! port 32108) and cloud lookups (`LOOKUP_WITH_KEY` with this host's own
//! address, or the variant-2 form until that is known, to every lookup
//! server the record's `p2p_conn` names, from the session's socket and
//! seven more source ports) are repeated every second. A `LOOKUP_ADDR` or a
//! local answer is hole-punched with `CHECK_CAM` (the port and three either
//! side); the device's `CAM_ID` completes the connection. Once a lookup
//! server has acknowledged the P2P request for 24 rounds, the relay
//! handshake starts as the vendor client runs it — `ListReq_DSK`, then
//! `RLY_HELLO` to each listed relay, `RLY_PORT` to the first that answers,
//! `RLY_REQ` naming its port and mark — and the relay's `RLY_RDY` completes
//! the connection with the relay as the peer. A `RLY_TO` is registered
//! with (four `RLY_PKT`s) rather than punched. 45 s bounds it all.
//!
//! **Connected**: `CMD_GATEWAYINFO` is sent to prompt the device, a PING
//! every 5 s keeps the path, every DATA packet is acknowledged, and the
//! device's ACKs of ours are reported as [`Event::Acked`]. The gateway
//! reply carries the level-2 key, wrapped for the cipher its `cipher_id`
//! names; [`Session::establish_level2`] has the caller fetch that cipher's
//! private keys from the cloud and unwraps it. A device that offers no
//! level-2 key — a reply too short to hold one (the Smart Lock C220,
//! T8506, answers a result code, -103), or one naming a cipher the cloud
//! has no keys for — has its `SET_PAYLOAD` commands sealed at level 1
//! (AES-128-ECB under the key derived from serial and P2P id, sign code
//! 1), as eufy-security-client (MIT) does: its `CMD_GATEWAYINFO` handler
//! sets level 1 whenever `getCipher` finds no such cipher.
//!
//! Ported from mega-yfue/eufy-sdk (Apache-2.0),
//! `src/transport/p2p/p2p-session.ts`. Left out: media, talkback, the
//! database and image reads, and the many-camera fan-out; inbound
//! reordering is kept, simplified.

use std::collections::{BTreeMap, HashMap, VecDeque};
use std::io::ErrorKind;
use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4, UdpSocket};
use std::time::{Duration, Instant};

use serde_json::{Value, json};

use super::codec::{self, data_type, request, response};

/// How long reaching a device may take (the relay starts about 25 s in).
pub const CONNECT_TIMEOUT: Duration = Duration::from_secs(45);
/// How long a command waits for the level-2 key before asking once more.
pub const LEVEL2_GRACE: Duration = Duration::from_secs(25);
const HEARTBEAT: Duration = Duration::from_secs(5);
const LOOKUP_RETRY: Duration = Duration::from_secs(1);
const RELAY_AFTER_ACKED_ROUNDS: u32 = 24;
const RELAY_PACKET_COPIES: usize = 4;
const PUNCH_PROBE_SOCKETS: usize = 7;
const REORDER_WAIT: Duration = Duration::from_millis(250);
const REORDER_MAX_DATAGRAMS: usize = 128;
const SEQUENCE_LOOKBACK: u16 = 0x8000;
const STALE_RETRANSMIT_DEPTH: u16 = 1024;

/// `CMD_GATEWAYINFO`: prompts the device; its reply carries the level-2 key.
pub const CMD_GATEWAYINFO: u16 = 1100;
/// `CMD_SET_PAYLOAD`: a JSON command envelope.
pub const CMD_SET_PAYLOAD: u16 = 1350;
/// `CMD_NOTIFY_PAYLOAD`: the device's own JSON notification.
pub const CMD_NOTIFY_PAYLOAD: u16 = 1351;
/// The channel that addresses the station itself.
pub const STATION_CHANNEL: u8 = 255;
/// A `CMD_SET_PAYLOAD` answer of this many bytes on a sign-code-0 frame
/// carries its result code in the first four.
const PADDED_RESULT_BYTES: usize = 132;

/// Where and how to reach a device.
#[derive(Debug, Clone)]
pub struct Config {
    /// The serial of the device the session is with (a level-1 key input).
    pub serial: String,
    /// Its P2P id (also a key input: treat it as a credential).
    pub p2p_did: String,
    /// The cloud lookup servers (its `p2p_conn`, decoded and resolved).
    pub lookup_servers: Vec<SocketAddrV4>,
    /// The DSK key the cloud lookup needs; without it only local lookup runs.
    pub dsk_key: Option<String>,
    /// The device's LAN address, when known, for a direct local lookup.
    pub local_address: Option<Ipv4Addr>,
    /// Whether to broadcast the local lookup.
    pub broadcast: bool,
}

/// A cipher's private keys, as the cloud's `get_ciphers` gives them.
#[derive(Clone, Default, PartialEq, Eq)]
pub struct CipherKeys {
    /// `ecc_private_key`: hex P-256 scalar.
    pub ecc: Option<String>,
    /// `private_key`: base64 PKCS#8 RSA key.
    pub rsa: Option<String>,
}

impl std::fmt::Debug for CipherKeys {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CipherKeys")
            .field("ecc", &self.ecc.is_some())
            .field("rsa", &self.rsa.is_some())
            .finish()
    }
}

/// What arrived from the device.
#[derive(Debug, Clone, PartialEq)]
pub enum Event {
    /// The device acknowledged our DATA packet with this sequence number.
    Acked(u16),
    /// A reply whose whole body is an int32 LE result (negative = refused).
    CommandResult { code: i32, channel: u8 },
    /// A frame carrying a JSON document.
    Json {
        command: u16,
        channel: u8,
        json: Value,
    },
}

/// The level-2 key, by kind.
#[derive(Clone, PartialEq, Eq)]
enum Level2 {
    None,
    /// AES-256-GCM, from a cipher's ECC key.
    Gcm([u8; 32]),
    /// AES-128-ECB in the level-1 layout, from a cipher's RSA key; it seals
    /// `SET_PAYLOAD` commands only.
    Ecb([u8; 16]),
    /// The device offers no level-2 key (its gateway reply is too short to
    /// hold one, or names a cipher the cloud has no keys for): `SET_PAYLOAD`
    /// is sealed under the level-1 key at sign code 1, as
    /// eufy-security-client falls back to level 1 there.
    Level1,
}

/// Per-data-type inbound sequencing and reassembly.
#[derive(Default)]
struct Inbound {
    last: Option<u16>,
    held: BTreeMap<u16, Vec<u8>>,
    hole_since: Option<Instant>,
    /// Bytes of a frame cut by a datagram boundary, from its magic on.
    pending: Vec<u8>,
}

/// A connected session.
pub struct Session {
    socket: UdpSocket,
    peer: SocketAddrV4,
    seq: u16,
    level1_key: [u8; 16],
    level2: Level2,
    level2_counter: u32,
    gateway: Option<Vec<u8>>,
    gateway_settled: bool,
    inbound: HashMap<u8, Inbound>,
    events: VecDeque<Event>,
    last_ping: Instant,
    last_pong: Option<Vec<u8>>,
    ended: bool,
}

fn now_plus(duration: Duration) -> Instant {
    Instant::now() + duration
}

fn bind() -> Result<UdpSocket, String> {
    let socket =
        UdpSocket::bind((Ipv4Addr::UNSPECIFIED, 0)).map_err(|e| format!("binding UDP: {e}"))?;
    socket.set_nonblocking(true).map_err(|e| e.to_string())?;
    Ok(socket)
}

fn send(socket: &UdpSocket, to: SocketAddrV4, kind: [u8; 2], payload: &[u8]) {
    let _ = socket.send_to(&codec::frame_message(kind, payload), to);
}

/// This host's outbound IPv4 (no packet is sent: a UDP connect only routes).
fn own_address() -> Option<Ipv4Addr> {
    let probe = UdpSocket::bind((Ipv4Addr::UNSPECIFIED, 0)).ok()?;
    probe.connect((Ipv4Addr::new(8, 8, 8, 8), 53)).ok()?;
    match probe.local_addr().ok()? {
        SocketAddr::V4(addr) if !addr.ip().is_unspecified() => Some(*addr.ip()),
        _ => None,
    }
}

fn port_of(socket: &UdpSocket) -> u16 {
    socket.local_addr().map_or(0, |addr| addr.port())
}

/// Where a connect's relay fallback stands.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Relay {
    List(Vec<SocketAddrV4>),
    Hello(Vec<SocketAddrV4>),
    Port(SocketAddrV4),
    Request { via: SocketAddrV4, mark: u32 },
}

/// The connect handshake's state, apart from its sockets.
struct Connecting<'a> {
    cfg: &'a Config,
    did: [u8; 20],
    own: Option<Ipv4Addr>,
    acked_rounds: Option<u32>,
    relay: Option<Relay>,
}

impl Connecting<'_> {
    fn cloud(&self) -> Option<&str> {
        self.cfg
            .dsk_key
            .as_deref()
            .filter(|_| !self.cfg.lookup_servers.is_empty())
    }

    fn send_lookups(&mut self, sockets: &[UdpSocket]) {
        let main = &sockets[0];
        let local = codec::local_lookup_payload();
        if self.cfg.broadcast {
            send(
                main,
                SocketAddrV4::new(Ipv4Addr::BROADCAST, codec::LOCAL_LOOKUP_PORT),
                request::LOCAL_LOOKUP,
                &local,
            );
        }
        if let Some(ip) = self.cfg.local_address {
            send(
                main,
                SocketAddrV4::new(ip, codec::LOCAL_LOOKUP_PORT),
                request::LOCAL_LOOKUP,
                &local,
            );
        }
        if let Some(key) = self.cloud() {
            for socket in sockets {
                let (kind, payload) = match self.own {
                    Some(ip) => (
                        request::LOOKUP_WITH_KEY,
                        codec::lookup_with_key_payload(
                            &self.did,
                            SocketAddrV4::new(ip, port_of(socket)),
                            key,
                        ),
                    ),
                    None => (
                        request::LOOKUP_WITH_KEY2,
                        codec::lookup_with_key2_payload(&self.did, key),
                    ),
                };
                for server in &self.cfg.lookup_servers {
                    send(socket, *server, kind, &payload);
                }
            }
            if let Some(rounds) = self.acked_rounds.as_mut()
                && self.relay.is_none()
            {
                *rounds += 1;
                if *rounds > RELAY_AFTER_ACKED_ROUNDS {
                    self.relay = Some(Relay::List(Vec::new()));
                }
            }
        }
        self.send_relay(main);
    }

    fn send_relay(&self, main: &UdpSocket) {
        let Some(key) = self.cloud() else { return };
        match &self.relay {
            Some(Relay::List(_)) => {
                let payload = codec::lookup_with_key2_payload(&self.did, key);
                for server in &self.cfg.lookup_servers {
                    send(main, *server, request::LOOKUP_WITH_KEY2, &payload);
                }
            }
            Some(Relay::Hello(relays)) => {
                for relay in relays {
                    send(main, *relay, request::RLY_HELLO, &[]);
                }
            }
            Some(Relay::Port(via)) => send(main, *via, request::RLY_PORT, &[]),
            Some(Relay::Request { via, mark }) => {
                let payload = codec::relay_request_payload(&self.did, *via, *mark);
                for server in &self.cfg.lookup_servers {
                    send(main, *server, request::RLY_REQ, &payload);
                }
            }
            None => {}
        }
    }

    /// Advance the relay handshake on one of its answers (main socket only).
    fn on_relay_message(&mut self, msg: &[u8], from: SocketAddrV4, main: &UdpSocket) {
        let next = match (&self.relay, msg) {
            (Some(Relay::List(known) | Relay::Hello(known)), m)
                if codec::has_header(m, response::LIST_REQ_ACK) =>
            {
                let mut relays = known.clone();
                for listed in codec::parse_relay_list(m) {
                    if !relays.contains(&listed) {
                        relays.push(listed);
                    }
                }
                if relays.is_empty() {
                    Relay::List(relays)
                } else {
                    Relay::Hello(relays)
                }
            }
            (Some(Relay::Hello(relays)), m)
                if codec::has_header(m, response::RLY_HELLO_ACK) && relays.contains(&from) =>
            {
                Relay::Port(from)
            }
            (Some(Relay::Port(via)), m) if codec::has_header(m, response::RLY_PORT_ACK) => {
                let Some((mark, port)) = codec::parse_relay_port_ack(m) else {
                    return;
                };
                Relay::Request {
                    via: SocketAddrV4::new(*via.ip(), port),
                    mark,
                }
            }
            _ => return,
        };
        self.relay = Some(next);
        self.send_relay(main);
    }

    fn check_cam(&self, socket: &UdpSocket, addr: SocketAddrV4) {
        let payload = codec::check_cam_payload(&self.did);
        let port = addr.port() as i32;
        for p in (port - 3..=port + 3).filter(|p| *p > 0 && *p <= u16::MAX as i32) {
            send(
                socket,
                SocketAddrV4::new(*addr.ip(), p as u16),
                request::CHECK_CAM,
                &payload,
            );
        }
    }

    /// Handle one pre-connect packet; the peer when it completes the connect.
    fn on_message(
        &mut self,
        msg: &[u8],
        from: SocketAddrV4,
        socket: &UdpSocket,
        is_main: bool,
    ) -> Option<SocketAddrV4> {
        use codec::has_header as is;
        if is(msg, response::LOCAL_LOOKUP_RESP) {
            self.check_cam(socket, from);
        } else if is(msg, response::LOOKUP_ADDR) {
            if let Some(addr) = codec::read_address(msg, 4).filter(|a| !a.ip().is_unspecified()) {
                self.check_cam(socket, addr);
            }
        } else if is(msg, response::RLY_TO) {
            if let Some((addr, handle)) =
                codec::parse_relay_to(msg).filter(|(a, _)| !a.ip().is_unspecified())
            {
                let payload = codec::relay_packet_payload(handle, &self.did);
                for _ in 0..RELAY_PACKET_COPIES {
                    send(socket, addr, request::RLY_PKT, &payload);
                }
            }
        } else if is(msg, response::RLY_RDY) {
            if msg.get(4..24) == Some(&self.did[..]) {
                return Some(from);
            }
        } else if is(msg, response::P2P_REQ_ACK) {
            if self.acked_rounds.is_none() && msg.get(4) == Some(&0) {
                self.acked_rounds = Some(0);
            }
        } else if is(msg, response::LIST_REQ_ACK)
            || is(msg, response::RLY_HELLO_ACK)
            || is(msg, response::RLY_PORT_ACK)
            || is(msg, response::RLY_REQ_ACK)
        {
            if is_main {
                self.on_relay_message(msg, from, socket);
            }
        } else if is(msg, response::CAM_ID) {
            return Some(from);
        } else if is(msg, response::PING) {
            send(socket, from, request::PONG, &[]);
        }
        None
    }
}

impl Session {
    /// Reach the device, or fail after [`CONNECT_TIMEOUT`].
    pub fn connect(cfg: Config) -> Result<Session, String> {
        let did = codec::p2p_did_bytes(&cfg.p2p_did)?;
        let level1_key = codec::level1_key(&cfg.serial, &cfg.p2p_did)?;
        let main = bind()?;
        let _ = main.set_broadcast(true);
        let mut state = Connecting {
            cfg: &cfg,
            did,
            own: own_address(),
            acked_rounds: None,
            relay: None,
        };
        let mut sockets = vec![main];
        if state.cloud().is_some() {
            for _ in 0..PUNCH_PROBE_SOCKETS {
                if let Ok(probe) = bind() {
                    sockets.push(probe);
                }
            }
        }
        if !cfg.broadcast && cfg.local_address.is_none() && state.cloud().is_none() {
            return Err(
                "no way to look the device up: no DSK key or lookup servers, and no LAN address"
                    .into(),
            );
        }
        let deadline = now_plus(CONNECT_TIMEOUT);
        let mut next_lookup = Instant::now();
        let mut buf = [0u8; 2048];
        loop {
            if Instant::now() >= deadline {
                let relay = match &state.relay {
                    None if state.cloud().is_some() => "not started",
                    None => "no cloud lookup",
                    Some(Relay::List(_)) => "list",
                    Some(Relay::Hello(_)) => "hello",
                    Some(Relay::Port(_)) => "port",
                    Some(Relay::Request { .. }) => "request",
                };
                return Err(format!("P2P connect timed out (relay: {relay})"));
            }
            if Instant::now() >= next_lookup {
                state.send_lookups(&sockets);
                next_lookup = now_plus(LOOKUP_RETRY);
            }
            let mut idle = true;
            let mut winner = None;
            for (index, socket) in sockets.iter().enumerate() {
                match socket.recv_from(&mut buf) {
                    Ok((len, SocketAddr::V4(from))) => {
                        idle = false;
                        if let Some(peer) = state.on_message(&buf[..len], from, socket, index == 0)
                        {
                            winner = Some((index, peer));
                            break;
                        }
                    }
                    Ok(_) => idle = false,
                    Err(e) if e.kind() == ErrorKind::WouldBlock => {}
                    Err(_) => {}
                }
            }
            if let Some((index, peer)) = winner {
                let socket = sockets.swap_remove(index);
                drop(sockets);
                socket.set_nonblocking(false).map_err(|e| e.to_string())?;
                socket
                    .set_read_timeout(Some(Duration::from_millis(50)))
                    .map_err(|e| e.to_string())?;
                let mut session = Session {
                    socket,
                    peer,
                    seq: 0,
                    level1_key,
                    level2: Level2::None,
                    level2_counter: 0,
                    gateway: None,
                    gateway_settled: false,
                    inbound: HashMap::new(),
                    events: VecDeque::new(),
                    last_ping: Instant::now(),
                    last_pong: None,
                    ended: false,
                };
                session.send_command(CMD_GATEWAYINFO, STATION_CHANNEL);
                session.ping();
                return Ok(session);
            }
            if idle {
                std::thread::sleep(Duration::from_millis(5));
            }
        }
    }

    /// The address the session talks to (the device, or a relay).
    pub fn peer(&self) -> SocketAddrV4 {
        self.peer
    }

    /// Whether a level-2 key is held that seals `SET_PAYLOAD` commands.
    pub fn has_level2(&self) -> bool {
        !matches!(self.level2, Level2::None | Level2::Level1)
    }

    fn ping(&mut self) {
        let pong = self.last_pong.clone().unwrap_or_default();
        send(&self.socket, self.peer, request::PING, &pong);
        self.last_ping = Instant::now();
    }

    fn next_seq(&mut self) -> u16 {
        let seq = self.seq;
        self.seq = self.seq.wrapping_add(1);
        seq
    }

    fn send_data(&mut self, command: u16, body: &[u8]) -> u16 {
        let seq = self.next_seq();
        let mut data = codec::command_header(seq, command).to_vec();
        data.extend_from_slice(body);
        send(&self.socket, self.peer, request::DATA, &data);
        seq
    }

    fn send_command(&mut self, command: u16, channel: u8) -> u16 {
        self.send_data(command, &codec::void_body(channel))
    }

    /// The `SET_PAYLOAD` body of a level-2 command:
    /// `{account_id, cmd, mChannel, mValue3, payload}`, sealed under the
    /// level-2 key — GCM at sign code 8 (header magic `08 00`), or, with
    /// an RSA cipher's key, ECB at sign code 2 in the level-1 layout; to a
    /// device that offers no level-2 key, ECB under the level-1 key at sign
    /// code 1.
    fn control_body(
        &mut self,
        cmd: u32,
        channel: u8,
        account_id: &str,
        payload: &Value,
        m_value3: u32,
        nonce: [u8; 12],
    ) -> Option<Vec<u8>> {
        let value = json!({
            "account_id": account_id,
            "cmd": cmd,
            "mChannel": channel,
            "mValue3": m_value3,
            "payload": payload,
        })
        .to_string();
        match self.level2.clone() {
            Level2::None => None,
            Level2::Level1 => Some(codec::command_body(
                &codec::ecb_encrypt(&codec::zero_pad(value.as_bytes()), &self.level1_key),
                channel,
                1,
                [0x01, 0x00],
            )),
            Level2::Gcm(key) => {
                let body = codec::seal_level2(&key, nonce, self.level2_counter, value.as_bytes());
                self.level2_counter = self.level2_counter.wrapping_add(1);
                Some(codec::command_body(&body, channel, 8, [0x08, 0x00]))
            }
            Level2::Ecb(key) => Some(codec::command_body(
                &codec::ecb_encrypt(&codec::zero_pad(value.as_bytes()), &key),
                channel,
                2,
                [0x01, 0x00],
            )),
        }
    }

    /// Send a level-2 `SET_PAYLOAD` command; answers the DATA sequence
    /// number it went under, which the device's ACK names.
    pub fn send_control_level2(
        &mut self,
        cmd: u32,
        channel: u8,
        account_id: &str,
        payload: &Value,
        m_value3: u32,
    ) -> Result<u16, String> {
        use rand_core::RngCore;
        let mut nonce = [0u8; 12];
        rand_core::OsRng.fill_bytes(&mut nonce);
        let body = self
            .control_body(cmd, channel, account_id, payload, m_value3, nonce)
            .ok_or("no level-2 key negotiated with the device")?;
        Ok(self.send_data(CMD_SET_PAYLOAD, &body))
    }

    /// Wait for the level-2 key, fetching the cipher's private keys with
    /// `resolve` when the gateway reply names it (`Ok(None)`: the cloud has
    /// no keys for that cipher, and commands are sealed at level 1). Asks
    /// the device once more when the first ask goes unanswered within
    /// `grace` or settles with no key.
    pub fn establish_level2(
        &mut self,
        resolve: &mut dyn FnMut(u16) -> Result<Option<CipherKeys>, String>,
        grace: Duration,
    ) -> Result<(), String> {
        let mut last_error = String::from("the device sent no gateway info");
        for attempt in 0..2 {
            if attempt == 1 {
                self.gateway_settled = false;
                self.send_command(CMD_GATEWAYINFO, STATION_CHANNEL);
            }
            let deadline = now_plus(grace);
            while Instant::now() < deadline && !self.gateway_settled {
                self.pump(now_plus(Duration::from_millis(100)))?;
                if let Some(payload) = self.gateway.take() {
                    self.gateway_settled = true;
                    match self.negotiate(&payload, resolve) {
                        Ok(()) => return Ok(()),
                        Err(error) => last_error = error,
                    }
                }
            }
        }
        Err(format!("no level-2 key: {last_error}"))
    }

    fn negotiate(
        &mut self,
        payload: &[u8],
        resolve: &mut dyn FnMut(u16) -> Result<Option<CipherKeys>, String>,
    ) -> Result<(), String> {
        if codec::gateway_holds_no_key(payload) {
            self.level2 = Level2::Level1;
            return Ok(());
        }
        let cipher_id = codec::gateway_cipher_id(payload);
        let Some(keys) = resolve(cipher_id)? else {
            self.level2 = Level2::Level1;
            return Ok(());
        };
        if let Some(ecc) = &keys.ecc {
            let key = codec::level2_key_from_gateway_ecc(payload, ecc).ok_or(format!(
                "the level-2 key did not unwrap (cipher {cipher_id})"
            ))?;
            self.level2 = Level2::Gcm(key);
            return Ok(());
        }
        if let Some(rsa) = &keys.rsa {
            let key = codec::level2_key_from_gateway_rsa(payload, rsa).ok_or(format!(
                "the level-2 key did not unwrap (cipher {cipher_id}, RSA)"
            ))?;
            self.level2 = Level2::Ecb(key);
            return Ok(());
        }
        Err(format!(
            "the cloud has no private key for cipher {cipher_id}"
        ))
    }

    /// The next event, if one has arrived.
    pub fn next_event(&mut self) -> Option<Event> {
        self.events.pop_front()
    }

    /// Read and handle packets until `until`, keeping the heartbeat.
    pub fn pump(&mut self, until: Instant) -> Result<(), String> {
        let mut buf = [0u8; 4096];
        loop {
            if self.ended {
                return Err("the device ended the session".into());
            }
            if self.last_ping.elapsed() >= HEARTBEAT {
                self.ping();
            }
            self.expire_holes();
            if Instant::now() >= until {
                return Ok(());
            }
            match self.socket.recv_from(&mut buf) {
                Ok((len, SocketAddr::V4(from))) => {
                    let msg = buf[..len].to_vec();
                    self.on_message(&msg, from);
                }
                Ok(_) => {}
                Err(e) if matches!(e.kind(), ErrorKind::WouldBlock | ErrorKind::TimedOut) => {}
                Err(e) => return Err(format!("reading the P2P socket: {e}")),
            }
        }
    }

    fn on_message(&mut self, msg: &[u8], from: SocketAddrV4) {
        use codec::has_header as is;
        let from_peer = from == self.peer;
        if is(msg, response::PING) {
            send(&self.socket, from, request::PONG, &[]);
        } else if !from_peer {
        } else if is(msg, response::PONG) {
            self.last_pong = msg
                .get(4..)
                .filter(|rest| !rest.is_empty())
                .map(<[u8]>::to_vec);
        } else if is(msg, response::END) {
            self.ended = true;
        } else if is(msg, response::ACK) {
            for seq in codec::parse_ack(msg, data_type::DATA) {
                self.events.push_back(Event::Acked(seq));
            }
        } else if is(msg, response::DATA) {
            self.on_data(msg);
        }
    }

    /// Acknowledge a DATA packet and deliver its body in sequence: a
    /// repeat is dropped, a packet past a hole is held until the hole fills
    /// or [`REORDER_WAIT`] passes, and a number far behind the last one is
    /// read as the device having restarted its numbering.
    fn on_data(&mut self, msg: &[u8]) {
        if msg.len() < 8 {
            return;
        }
        let kind = msg[5];
        let seq = u16::from_be_bytes([msg[6], msg[7]]);
        send(
            &self.socket,
            self.peer,
            request::ACK,
            &codec::ack_payload(kind, seq),
        );
        let body = msg[8..].to_vec();
        let state = self.inbound.entry(kind).or_default();
        let advance = state.last.map_or(1, |last| seq.wrapping_sub(last));
        if advance == 0 {
            return;
        }
        if advance > SEQUENCE_LOOKBACK {
            if 0u16.wrapping_sub(advance) <= STALE_RETRANSMIT_DEPTH {
                return;
            }
            *state = Inbound {
                last: Some(seq),
                ..Inbound::default()
            };
            self.reassemble(kind, &body);
            return;
        }
        if advance == 1 {
            state.last = Some(seq);
            self.reassemble(kind, &body);
            self.drain(kind);
            return;
        }
        state.held.entry(seq).or_insert(body);
        state.hole_since.get_or_insert_with(Instant::now);
        if state.held.len() > REORDER_MAX_DATAGRAMS {
            self.abandon_hole(kind);
        }
    }

    fn drain(&mut self, kind: u8) {
        loop {
            let state = self.inbound.get_mut(&kind).expect("present");
            let next = state.last.expect("delivered").wrapping_add(1);
            let Some(body) = state.held.remove(&next) else {
                state.hole_since = if state.held.is_empty() {
                    None
                } else {
                    Some(Instant::now())
                };
                return;
            };
            state.last = Some(next);
            self.reassemble(kind, &body);
        }
    }

    fn abandon_hole(&mut self, kind: u8) {
        let state = self.inbound.get_mut(&kind).expect("present");
        let last = state.last.unwrap_or(0);
        let Some(earliest) = state
            .held
            .keys()
            .copied()
            .min_by_key(|seq| seq.wrapping_sub(last))
        else {
            return;
        };
        state.pending.clear();
        state.last = Some(earliest.wrapping_sub(1));
        self.drain(kind);
    }

    fn expire_holes(&mut self) {
        let expired: Vec<u8> = self
            .inbound
            .iter()
            .filter(|(_, state)| {
                state
                    .hole_since
                    .is_some_and(|since| since.elapsed() >= REORDER_WAIT)
            })
            .map(|(kind, _)| *kind)
            .collect();
        for kind in expired {
            self.abandon_hole(kind);
        }
    }

    /// Split an in-sequence body into frames, carrying a frame (or a
    /// header) cut by the datagram boundary into the next one.
    fn reassemble(&mut self, kind: u8, body: &[u8]) {
        let state = self.inbound.get_mut(&kind).expect("present");
        let mut buf = std::mem::take(&mut state.pending);
        buf.extend_from_slice(body);
        let mut at = 0;
        let mut frames = Vec::new();
        while buf.len() - at >= 4 && &buf[at..at + 4] == codec::MAGIC {
            let Some(header) = codec::parse_frame_header(&buf[at..]) else {
                break;
            };
            let end = at + codec::FRAME_HEADER_BYTES + header.length as usize;
            if buf.len() < end {
                break;
            }
            frames.push((header, buf[at + codec::FRAME_HEADER_BYTES..end].to_vec()));
            at = end;
        }
        let rest = &buf[at..];
        let keep = (rest.len() >= 4 && &rest[..4] == codec::MAGIC)
            || (!rest.is_empty() && rest.len() < 4 && codec::MAGIC.starts_with(rest));
        state.pending = if keep { rest.to_vec() } else { Vec::new() };
        for (header, payload) in frames {
            self.handle_frame(header, payload);
        }
    }

    fn handle_frame(&mut self, header: codec::FrameHeader, payload: Vec<u8>) {
        let media = header.command == 1300 || header.command == 1301;
        let opened = match (media, header.sign_code, &self.level2) {
            (true, _, _) => None,
            (false, 2 | 8, Level2::Gcm(key)) => codec::open_level2(key, &payload, header.sign_code),
            (false, 1.., _) if !payload.is_empty() => {
                codec::ecb_decrypt(&payload, &self.level1_key)
            }
            _ => None,
        };
        let data = opened.unwrap_or(payload);
        if header.command == CMD_GATEWAYINFO
            && header.sign_code <= 1
            && self.level2 == Level2::None
            && !self.gateway_settled
        {
            self.gateway = Some(data.clone());
        }
        let text = codec::nul_terminated(&data);
        let json = text
            .starts_with('{')
            .then(|| serde_json::from_str::<Value>(&text).ok())
            .flatten();
        let padded = header.command == CMD_SET_PAYLOAD
            && match header.sign_code {
                0 => data.len() == PADDED_RESULT_BYTES,
                // A result sealed at level 1: the code, zero-padded to a block.
                1 => data.len() == 16 && data[4..].iter().all(|&byte| byte == 0),
                _ => false,
            };
        if let Some(json) = json {
            self.events.push_back(Event::Json {
                command: header.command,
                channel: header.channel,
                json,
            });
        } else if !media && (data.len() == 4 || padded) {
            let code = i32::from_le_bytes(data[..4].try_into().expect("4 bytes"));
            self.events.push_back(Event::CommandResult {
                code,
                channel: header.channel,
            });
        }
    }

    /// End the session (sends `END`).
    pub fn close(self) {
        send(&self.socket, self.peer, request::END, &[]);
    }

    #[cfg(test)]
    fn for_test(peer: SocketAddrV4, socket: UdpSocket) -> Session {
        socket
            .set_read_timeout(Some(Duration::from_millis(20)))
            .unwrap();
        Session {
            socket,
            peer,
            seq: 0,
            level1_key: codec::level1_key("T8000P0000000000", "XXXXXXX-000000-XXXXX").unwrap(),
            level2: Level2::None,
            level2_counter: 0,
            gateway: None,
            gateway_settled: false,
            inbound: HashMap::new(),
            events: VecDeque::new(),
            last_ping: Instant::now(),
            last_pong: None,
            ended: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn loopback() -> (UdpSocket, SocketAddrV4) {
        let socket = UdpSocket::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
        let SocketAddr::V4(addr) = socket.local_addr().unwrap() else {
            unreachable!()
        };
        (socket, addr)
    }

    /// A session wired to a fake device on loopback.
    fn pair() -> (Session, UdpSocket) {
        let (device, device_addr) = loopback();
        let (ours, _) = loopback();
        device
            .set_read_timeout(Some(Duration::from_millis(200)))
            .unwrap();
        (Session::for_test(device_addr, ours), device)
    }

    fn frame(command: u16, channel: u8, sign_code: u8, payload: &[u8]) -> Vec<u8> {
        let mut out = codec::MAGIC.to_vec();
        out.extend_from_slice(&command.to_le_bytes());
        out.extend(codec::command_body(
            payload,
            channel,
            sign_code,
            [0x01, 0x00],
        ));
        out
    }

    fn data_packet(seq: u16, body: &[u8]) -> Vec<u8> {
        let mut payload = vec![0xd1, 0, 0, 0];
        payload[2..4].copy_from_slice(&seq.to_be_bytes());
        payload.extend_from_slice(body);
        codec::frame_message(response::DATA, &payload)
    }

    fn deliver(session: &mut Session, packet: &[u8]) {
        let peer = session.peer;
        session.on_message(packet, peer);
    }

    fn events(session: &mut Session) -> Vec<Event> {
        std::iter::from_fn(|| session.next_event()).collect()
    }

    #[test]
    fn a_data_packet_is_acknowledged_and_its_json_reported() {
        let (mut session, device) = pair();
        let json = br#"{"cmd":1940,"payload":{"time":"1a"}}"#;
        deliver(
            &mut session,
            &data_packet(7, &frame(CMD_NOTIFY_PAYLOAD, 0, 0, json)),
        );
        let mut buf = [0u8; 64];
        let (len, _) = device.recv_from(&mut buf).unwrap();
        assert_eq!(
            &buf[..len],
            &codec::frame_message(request::ACK, &codec::ack_payload(0, 7))[..]
        );
        assert_eq!(
            events(&mut session),
            vec![Event::Json {
                command: CMD_NOTIFY_PAYLOAD,
                channel: 0,
                json: serde_json::from_slice(json).unwrap()
            }]
        );
    }

    #[test]
    fn a_bare_int32_reply_is_a_command_result() {
        let (mut session, _device) = pair();
        deliver(
            &mut session,
            &data_packet(0, &frame(CMD_SET_PAYLOAD, 3, 0, &(-108i32).to_le_bytes())),
        );
        let mut padded = 0i32.to_le_bytes().to_vec();
        padded.resize(PADDED_RESULT_BYTES, 0xaa);
        deliver(
            &mut session,
            &data_packet(1, &frame(CMD_SET_PAYLOAD, 3, 0, &padded)),
        );
        // 132 bytes on another command is not a padded result.
        deliver(&mut session, &data_packet(2, &frame(1700, 3, 0, &padded)));
        assert_eq!(
            events(&mut session),
            vec![
                Event::CommandResult {
                    code: -108,
                    channel: 3
                },
                Event::CommandResult {
                    code: 0,
                    channel: 3
                }
            ]
        );
    }

    #[test]
    fn level1_frames_decrypt_with_the_derived_key() {
        let (mut session, _device) = pair();
        let json = br#"{"cmd":1351,"payload":{"x":1}}"#;
        let sealed = codec::ecb_encrypt(&codec::zero_pad(json), &session.level1_key);
        deliver(
            &mut session,
            &data_packet(0, &frame(CMD_NOTIFY_PAYLOAD, 0, 1, &sealed)),
        );
        assert!(
            matches!(&events(&mut session)[..], [Event::Json { json, .. }] if json["payload"]["x"] == 1)
        );
    }

    #[test]
    fn level2_frames_decrypt_once_the_key_is_held() {
        let (mut session, _device) = pair();
        session.level2 = Level2::Gcm([7; 32]);
        let sealed = codec::seal_level2(&[7; 32], [1; 12], 0, &5i32.to_le_bytes());
        let mut sign2 = sealed[..28].to_vec();
        sign2.extend_from_slice(&sealed[32..]);
        deliver(
            &mut session,
            &data_packet(0, &frame(CMD_SET_PAYLOAD, 0, 2, &sign2)),
        );
        assert_eq!(
            events(&mut session),
            vec![Event::CommandResult {
                code: 5,
                channel: 0
            }]
        );
    }

    #[test]
    fn frames_are_reassembled_across_datagrams_and_reordered() {
        let (mut session, _device) = pair();
        let a = frame(CMD_NOTIFY_PAYLOAD, 0, 0, br#"{"n":1}"#);
        let b = frame(
            CMD_NOTIFY_PAYLOAD,
            0,
            0,
            br#"{"n":2,"pad":"xxxxxxxxxxxxxxxxxxxxxxxx"}"#,
        );
        let mut stream = a.clone();
        stream.extend_from_slice(&b);
        // Cut inside b's header, and deliver the second part first.
        let cut = a.len() + 6;
        let first = frame(CMD_NOTIFY_PAYLOAD, 0, 0, br#"{"n":0}"#);
        deliver(&mut session, &data_packet(0, &first));
        deliver(&mut session, &data_packet(2, &stream[cut..]));
        assert_eq!(events(&mut session).len(), 1);
        deliver(&mut session, &data_packet(1, &stream[..cut]));
        deliver(&mut session, &data_packet(1, &stream[..cut]));
        let numbers: Vec<_> = events(&mut session)
            .into_iter()
            .map(|event| match event {
                Event::Json { json, .. } => json["n"].as_i64().unwrap(),
                other => panic!("{other:?}"),
            })
            .collect();
        assert_eq!(numbers, vec![1, 2]);
        assert!(session.inbound[&0].pending.is_empty());
    }

    #[test]
    fn a_hole_is_given_up_on_and_delivery_resumes() {
        let (mut session, _device) = pair();
        deliver(
            &mut session,
            &data_packet(0, &frame(CMD_NOTIFY_PAYLOAD, 0, 0, br#"{"n":0}"#)),
        );
        deliver(
            &mut session,
            &data_packet(2, &frame(CMD_NOTIFY_PAYLOAD, 0, 0, br#"{"n":2}"#)),
        );
        assert_eq!(events(&mut session).len(), 1);
        std::thread::sleep(REORDER_WAIT + Duration::from_millis(20));
        session.pump(Instant::now()).unwrap();
        assert!(matches!(&events(&mut session)[..], [Event::Json { json, .. }] if json["n"] == 2));
        // A restart of the numbering far behind is taken as fresh.
        deliver(
            &mut session,
            &data_packet(40000, &frame(CMD_NOTIFY_PAYLOAD, 0, 0, br#"{"n":3}"#)),
        );
        assert_eq!(events(&mut session).len(), 1);
        // A retransmission just behind is dropped.
        deliver(
            &mut session,
            &data_packet(39990, &frame(CMD_NOTIFY_PAYLOAD, 0, 0, br#"{"n":4}"#)),
        );
        assert!(events(&mut session).is_empty());
    }

    #[test]
    fn acks_of_our_packets_are_reported_and_strangers_ignored() {
        let (mut session, _device) = pair();
        deliver(
            &mut session,
            &codec::frame_message(response::ACK, &[0xd1, 0, 0, 2, 0, 4, 0, 5]),
        );
        deliver(
            &mut session,
            &codec::frame_message(response::ACK, &[0xd1, 1, 0, 1, 0, 9]),
        );
        let stranger = SocketAddrV4::new(Ipv4Addr::LOCALHOST, 1);
        session.on_message(
            &codec::frame_message(response::ACK, &[0xd1, 0, 0, 1, 0, 6]),
            stranger,
        );
        assert_eq!(events(&mut session), vec![Event::Acked(4), Event::Acked(5)]);
        deliver(&mut session, &codec::frame_message(response::END, &[]));
        assert!(session.pump(Instant::now()).is_err());
    }

    #[test]
    fn the_gateway_reply_is_kept_for_negotiation_and_the_key_set() {
        let (mut session, _device) = pair();
        let cipher = p256::SecretKey::from_slice(&[0x07; 32]).unwrap();
        // Reuse the codec test's construction through the public pieces.
        let reply = {
            use cbc::cipher::{BlockEncryptMut, KeyIvInit};
            use p256::elliptic_curve::sec1::ToEncodedPoint;
            let ephemeral = p256::SecretKey::from_slice(&[0x42; 32]).unwrap();
            let shared = p256::ecdh::diffie_hellman(
                ephemeral.to_nonzero_scalar(),
                cipher.public_key().as_affine(),
            );
            let kdf = codec::eufy_kdf(shared.raw_secret_bytes(), 48);
            let key: [u8; 16] = kdf[..16].try_into().unwrap();
            let mut plain = vec![0x5a; 32];
            plain.extend_from_slice(&[0x10; 16]);
            let mut encryptor = cbc::Encryptor::<aes::Aes128>::new(&key.into(), &[0x11; 16].into());
            for block in plain.chunks_exact_mut(16) {
                encryptor.encrypt_block_mut(block.into());
            }
            let mut out = vec![9, 0, 0, 0];
            out.extend_from_slice(ephemeral.public_key().to_encoded_point(true).as_bytes());
            out.extend_from_slice(&[0x11; 16]);
            out.extend(plain);
            out
        };
        let sealed = codec::ecb_encrypt(&codec::zero_pad(&reply), &session.level1_key);
        deliver(
            &mut session,
            &data_packet(0, &frame(CMD_GATEWAYINFO, 255, 1, &sealed)),
        );
        let mut asked = Vec::new();
        let private = hex::encode(cipher.to_bytes());
        session
            .establish_level2(
                &mut |id| {
                    asked.push(id);
                    Ok(Some(CipherKeys {
                        ecc: Some(private.clone()),
                        rsa: None,
                    }))
                },
                Duration::from_millis(200),
            )
            .unwrap();
        assert_eq!(asked, vec![9]);
        assert!(session.level2 == Level2::Gcm([0x5a; 32]));
    }

    /// The `SET_PAYLOAD` frame the device receives next, opened: its
    /// header and its JSON, unsealed with the level-1 key when it was
    /// sealed at sign code 1.
    fn next_set_payload(session: &Session, device: &UdpSocket) -> (codec::FrameHeader, Value) {
        let mut buf = [0u8; 1024];
        let msg = loop {
            let (len, _) = device.recv_from(&mut buf).unwrap();
            if codec::has_header(&buf[..len], request::DATA) {
                break buf[..len].to_vec();
            }
        };
        let header = codec::parse_frame_header(&msg[8..]).unwrap();
        let body = &msg[8 + 16..];
        let plain = match header.sign_code {
            0 => body.to_vec(),
            1 => codec::ecb_decrypt(body, &session.level1_key).unwrap(),
            other => panic!("sealed at sign code {other}"),
        };
        let value = serde_json::from_str(&codec::nul_terminated(&plain)).unwrap();
        (header, value)
    }

    /// Answer the gateway prompt with `reply`, sealed at level 1 as the
    /// device seals it.
    fn answer_gateway(session: &mut Session, reply: &[u8]) {
        let sealed = codec::ecb_encrypt(&codec::zero_pad(reply), &session.level1_key);
        deliver(
            session,
            &data_packet(0, &frame(CMD_GATEWAYINFO, 255, 1, &sealed)),
        );
    }

    fn assert_sealed_at_level_one(session: &mut Session, device: &UdpSocket) {
        assert!(!session.has_level2());
        session
            .send_control_level2(1940, 0, "acct", &json!({"apiCommand": 6018}), 0)
            .unwrap();
        let (header, value) = next_set_payload(session, device);
        assert_eq!(
            (header.command, header.channel, header.sign_code),
            (CMD_SET_PAYLOAD, 0, 1)
        );
        assert_eq!(
            value,
            json!({"account_id": "acct", "cmd": 1940, "mChannel": 0, "mValue3": 0, "payload": {"apiCommand": 6018}})
        );
    }

    /// A Smart Lock C220 (T8506) answers the gateway prompt with a bare
    /// result, -103 (invalid command), sealed at level 1 and padded to a
    /// block — not a key envelope. Its first two bytes read as a cipher id
    /// (0xff99 = 65433) that the cloud does not have, which refused every
    /// lock command. The device offers no level-2 key, and its commands are
    /// sealed at level 1, as eufy-security-client seals them to it: that
    /// client sets level 1 whenever the cloud has no keys for the cipher a
    /// gateway reply names.
    #[test]
    fn a_gateway_answer_that_is_a_result_code_means_commands_go_at_level_one() {
        let (mut session, device) = pair();
        answer_gateway(&mut session, &(-103i32).to_le_bytes());
        session
            .establish_level2(
                &mut |id| panic!("a result code names no cipher (asked for {id})"),
                Duration::from_millis(200),
            )
            .unwrap();
        assert_sealed_at_level_one(&mut session, &device);
    }

    /// The live C220's answer was not the bare or zero-padded code: it
    /// still read as cipher 65433 and asked the cloud, which has no keys
    /// for it ("no level-2 key: the cloud has no keys for cipher 65433").
    /// Anything too short to hold a key envelope offers no key, whatever
    /// follows the code.
    #[test]
    fn a_gateway_answer_too_short_for_a_key_means_commands_go_at_level_one() {
        for reply in [
            vec![0x99, 0xff, 0xff, 0xff, 0x01, 0x00, 0x00, 0x00],
            vec![0x99, 0xff, 0xff, 0xff, 0x00, 0x00, 0x00, 0x00, 0x7f, 0x3c],
            [0x99, 0xff, 0x00, 0x00]
                .iter()
                .copied()
                .chain([0x55; 40])
                .collect(),
        ] {
            let (mut session, device) = pair();
            answer_gateway(&mut session, &reply);
            session
                .establish_level2(
                    &mut |id| panic!("{reply:02x?} holds no key (asked for {id})"),
                    Duration::from_millis(200),
                )
                .unwrap();
            assert_sealed_at_level_one(&mut session, &device);
        }
    }

    /// A reply long enough to be an envelope, naming a cipher the cloud
    /// answers with no keys for: eufy-security-client's `getCipher` finds
    /// nothing and it sets level 1, and so does this.
    #[test]
    fn a_cipher_the_cloud_has_no_keys_for_means_commands_go_at_level_one() {
        let (mut session, device) = pair();
        let mut reply = vec![0x99, 0xff, 0, 0];
        reply.resize(codec::SHORTEST_KEY_ENVELOPE, 0x33);
        answer_gateway(&mut session, &reply);
        let mut asked = Vec::new();
        session
            .establish_level2(
                &mut |id| {
                    asked.push(id);
                    Ok(None)
                },
                Duration::from_millis(200),
            )
            .unwrap();
        assert_eq!(asked, vec![0xff99]);
        assert_sealed_at_level_one(&mut session, &device);
    }

    /// A cloud that cannot be asked is not a cloud with no keys: the
    /// command is refused rather than sent under a guessed seal.
    #[test]
    fn a_cloud_that_fails_to_answer_refuses_the_command() {
        let (mut session, _device) = pair();
        let mut reply = vec![9, 0, 0, 0];
        reply.resize(codec::SHORTEST_KEY_ENVELOPE, 0x33);
        answer_gateway(&mut session, &reply);
        let error = session
            .establish_level2(
                &mut |_| Err("the cloud timed out".into()),
                Duration::from_millis(100),
            )
            .unwrap_err();
        assert!(error.contains("the cloud timed out"), "{error}");
        assert!(
            session
                .send_control_level2(1940, 0, "acct", &json!({}), 0)
                .is_err()
        );
    }

    /// A lock commanded at level 1 answers at level 1: its result is the
    /// code sealed and zero-padded to one block.
    #[test]
    fn a_result_sealed_at_level_one_is_read() {
        let (mut session, _device) = pair();
        let sealed = codec::ecb_encrypt(
            &codec::zero_pad(&(-108i32).to_le_bytes()),
            &session.level1_key,
        );
        deliver(
            &mut session,
            &data_packet(0, &frame(CMD_SET_PAYLOAD, 0, 1, &sealed)),
        );
        assert_eq!(
            events(&mut session),
            vec![Event::CommandResult {
                code: -108,
                channel: 0
            }]
        );
    }

    #[test]
    fn with_no_gateway_reply_the_device_is_asked_again_then_refused() {
        let (mut session, device) = pair();
        let error = session
            .establish_level2(
                &mut |_| panic!("no cipher is named"),
                Duration::from_millis(100),
            )
            .unwrap_err();
        assert!(error.contains("no gateway info"));
        let mut buf = [0u8; 64];
        let (len, _) = device.recv_from(&mut buf).unwrap();
        let msg = &buf[..len];
        assert!(codec::has_header(msg, request::DATA));
        assert_eq!(
            codec::parse_frame_header(&msg[8..]).unwrap().command,
            CMD_GATEWAYINFO
        );
    }

    #[test]
    fn a_control_command_is_a_sealed_set_payload_envelope() {
        let (mut session, device) = pair();
        assert!(
            session
                .send_control_level2(1940, 0, "acct", &json!({}), 0)
                .is_err()
        );
        session.level2 = Level2::Gcm([7; 32]);
        let seq = session
            .send_control_level2(1940, 2, "acct", &json!({"apiCommand": 6018}), 0)
            .unwrap();
        let mut buf = [0u8; 512];
        let (len, _) = device.recv_from(&mut buf).unwrap();
        let msg = &buf[..len];
        assert!(codec::has_header(msg, request::DATA));
        assert_eq!(u16::from_be_bytes([msg[6], msg[7]]), seq);
        let header = codec::parse_frame_header(&msg[8..]).unwrap();
        assert_eq!(
            (header.command, header.channel, header.sign_code),
            (CMD_SET_PAYLOAD, 2, 8)
        );
        assert_eq!(&msg[8 + 10..8 + 12], &[0x08, 0x00]);
        let body = &msg[8 + 16..];
        assert_eq!(body.len(), header.length as usize);
        let plain = codec::open_level2(&[7; 32], body, 8).unwrap();
        let value: Value = serde_json::from_slice(&plain).unwrap();
        assert_eq!(
            value,
            json!({"account_id": "acct", "cmd": 1940, "mChannel": 2, "mValue3": 0, "payload": {"apiCommand": 6018}})
        );
        // The ECB form an RSA cipher's key gives: sign code 2, level-1 layout.
        session.level2 = Level2::Ecb([3; 16]);
        session
            .send_control_level2(1940, 2, "acct", &json!({}), 0)
            .unwrap();
        let (len, _) = device.recv_from(&mut buf).unwrap();
        let msg = &buf[..len];
        let header = codec::parse_frame_header(&msg[8..]).unwrap();
        assert_eq!(
            (header.sign_code, &msg[8 + 10..8 + 12]),
            (2, &[0x01u8, 0x00][..])
        );
        let plain = codec::ecb_decrypt(&msg[8 + 16..], &[3; 16]).unwrap();
        assert!(codec::nul_terminated(&plain).starts_with("{\"account_id\":\"acct\""));
    }

    /// The whole connect against fake lookup servers and a fake device on
    /// loopback: the lookup names the device, the punch reaches it, and its
    /// CAM_ID completes the connect, after which it is prompted for its
    /// gateway info.
    #[test]
    fn connects_through_a_lookup_and_hole_punch() {
        let (server, server_addr) = loopback();
        let (device, device_addr) = loopback();
        server
            .set_read_timeout(Some(Duration::from_secs(5)))
            .unwrap();
        device
            .set_read_timeout(Some(Duration::from_secs(5)))
            .unwrap();
        let did = "XXXXXXX-000000-XXXXX";
        let fake = std::thread::spawn(move || {
            let mut buf = [0u8; 512];
            let (len, client) = server.recv_from(&mut buf).unwrap();
            assert!(
                codec::has_header(&buf[..len], request::LOOKUP_WITH_KEY)
                    || codec::has_header(&buf[..len], request::LOOKUP_WITH_KEY2)
            );
            assert_eq!(&buf[4..24], &codec::p2p_did_bytes(did).unwrap());
            let mut addr = codec::encode_address(device_addr).to_vec();
            addr.extend_from_slice(&[0; 4]);
            server
                .send_to(&codec::frame_message(response::LOOKUP_ADDR, &addr), client)
                .unwrap();
            loop {
                let (len, client) = device.recv_from(&mut buf).unwrap();
                if codec::has_header(&buf[..len], request::CHECK_CAM) {
                    assert_eq!(&buf[4..24], &codec::p2p_did_bytes(did).unwrap());
                    device
                        .send_to(&codec::frame_message(response::CAM_ID, &[0; 24]), client)
                        .unwrap();
                    break;
                }
            }
            loop {
                let (len, _) = device.recv_from(&mut buf).unwrap();
                if codec::has_header(&buf[..len], request::DATA) {
                    return codec::parse_frame_header(&buf[8..len]).unwrap().command;
                }
            }
        });
        let session = Session::connect(Config {
            serial: "T8000P0000000000".into(),
            p2p_did: did.into(),
            lookup_servers: vec![server_addr],
            dsk_key: Some("XXXXXXXXXXXXXXXXXXXX".into()),
            local_address: None,
            broadcast: false,
        })
        .unwrap();
        assert_eq!(session.peer(), device_addr);
        assert_eq!(fake.join().unwrap(), CMD_GATEWAYINFO);
        session.close();
    }

    #[test]
    fn relay_answers_advance_in_the_vendor_order() {
        let cfg = Config {
            serial: "T8000P0000000000".into(),
            p2p_did: "XXXXXXX-000000-XXXXX".into(),
            lookup_servers: vec![SocketAddrV4::new(Ipv4Addr::LOCALHOST, 9)],
            dsk_key: Some("K".into()),
            local_address: None,
            broadcast: false,
        };
        let did = codec::p2p_did_bytes(&cfg.p2p_did).unwrap();
        let mut state = Connecting {
            cfg: &cfg,
            did,
            own: None,
            acked_rounds: None,
            relay: None,
        };
        let (main, _) = loopback();
        let sockets = [main];
        state.on_message(
            &codec::frame_message(response::P2P_REQ_ACK, &[0]),
            SocketAddrV4::new(Ipv4Addr::LOCALHOST, 9),
            &sockets[0],
            true,
        );
        for _ in 0..=RELAY_AFTER_ACKED_ROUNDS {
            state.send_lookups(&sockets);
        }
        assert_eq!(state.relay, Some(Relay::List(Vec::new())));
        let relay = SocketAddrV4::new(Ipv4Addr::new(192, 0, 2, 7), 32100);
        let mut list = codec::frame_message(response::LIST_REQ_ACK, &[1, 0, 0, 0]);
        list.extend_from_slice(&codec::encode_address(relay));
        state.on_message(
            &list,
            SocketAddrV4::new(Ipv4Addr::LOCALHOST, 9),
            &sockets[0],
            true,
        );
        assert_eq!(state.relay, Some(Relay::Hello(vec![relay])));
        // A hello answer from a relay the list did not name is ignored.
        let other = SocketAddrV4::new(Ipv4Addr::new(192, 0, 2, 8), 32100);
        state.on_message(
            &codec::frame_message(response::RLY_HELLO_ACK, &[]),
            other,
            &sockets[0],
            true,
        );
        assert_eq!(state.relay, Some(Relay::Hello(vec![relay])));
        state.on_message(
            &codec::frame_message(response::RLY_HELLO_ACK, &[]),
            relay,
            &sockets[0],
            true,
        );
        assert_eq!(state.relay, Some(Relay::Port(relay)));
        state.on_message(
            &codec::frame_message(response::RLY_PORT_ACK, &[0, 0, 0, 5, 0x9c, 0x41, 0, 0]),
            relay,
            &sockets[0],
            true,
        );
        let via = SocketAddrV4::new(*relay.ip(), 40001);
        assert_eq!(state.relay, Some(Relay::Request { via, mark: 5 }));
        // Only the RLY_RDY naming our own device completes the connect.
        let mut other_did = did;
        other_did[19] ^= 1;
        assert_eq!(
            state.on_message(
                &codec::frame_message(response::RLY_RDY, &other_did),
                via,
                &sockets[0],
                true
            ),
            None
        );
        assert_eq!(
            state.on_message(
                &codec::frame_message(response::RLY_RDY, &did),
                via,
                &sockets[0],
                true
            ),
            Some(via)
        );
    }
}
