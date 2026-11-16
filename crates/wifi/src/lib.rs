//! A Wi-Fi radio on Linux, over nl80211 (generic netlink) on a raw socket:
//! scan, join an open network, leave it, set the interface's address and
//! link, and lend the radio to another network namespace.
//!
//! No `iw`, `wpa_supplicant` or DHCP client. A "full MAC" driver (brcmfmac,
//! for one) does the association itself on `NL80211_CMD_CONNECT`, and an open
//! network needs no supplicant. Message building and parsing are pure
//! functions with tests; the socket calls are the thin part only hardware can
//! exercise.

use std::net::Ipv4Addr;
use std::time::{Duration, Instant};

/// The radio's interface.
pub const INTERFACE: &str = "wlan0";

// --- netlink constants (linux/netlink.h, linux/genetlink.h, nl80211.h) ---

const NETLINK_GENERIC: libc::c_int = 16;
const SOL_NETLINK: libc::c_int = 270;
const NETLINK_ADD_MEMBERSHIP: libc::c_int = 1;

const NLM_F_REQUEST: u16 = 0x1;
const NLM_F_ACK: u16 = 0x4;
const NLM_F_DUMP: u16 = 0x300;
const NLMSG_ERROR: u16 = 2;
const NLMSG_DONE: u16 = 3;
const NLA_TYPE_MASK: u16 = 0x3fff;
const NLA_F_NESTED: u16 = 0x8000;

const GENL_ID_CTRL: u16 = 16;
const CTRL_CMD_GETFAMILY: u8 = 3;
const CTRL_ATTR_FAMILY_ID: u16 = 1;
const CTRL_ATTR_FAMILY_NAME: u16 = 2;
const CTRL_ATTR_MCAST_GROUPS: u16 = 7;
const CTRL_ATTR_MCAST_GRP_NAME: u16 = 1;
const CTRL_ATTR_MCAST_GRP_ID: u16 = 2;

const NL80211_CMD_GET_SCAN: u8 = 32;
const NL80211_CMD_TRIGGER_SCAN: u8 = 33;
const NL80211_CMD_NEW_SCAN_RESULTS: u8 = 34;
const NL80211_CMD_SCAN_ABORTED: u8 = 35;
const NL80211_CMD_CONNECT: u8 = 46;
const NL80211_CMD_DISCONNECT: u8 = 48;
const NL80211_CMD_SET_WIPHY_NETNS: u8 = 49;

const NL80211_ATTR_WIPHY: u16 = 1;
const NL80211_ATTR_IFINDEX: u16 = 3;
const NL80211_ATTR_PID: u16 = 82;
const NL80211_ATTR_MAC: u16 = 6;
const NL80211_ATTR_WIPHY_FREQ: u16 = 38;
const NL80211_ATTR_SCAN_FREQUENCIES: u16 = 44;
const NL80211_ATTR_SCAN_SSIDS: u16 = 45;
const NL80211_ATTR_BSS: u16 = 47;
const NL80211_ATTR_SSID: u16 = 52;
const NL80211_ATTR_AUTH_TYPE: u16 = 53;
const NL80211_ATTR_REASON_CODE: u16 = 54;
const NL80211_ATTR_STATUS_CODE: u16 = 72;

const NL80211_BSS_BSSID: u16 = 1;
const NL80211_BSS_FREQUENCY: u16 = 2;
const NL80211_BSS_CAPABILITY: u16 = 5;
const NL80211_BSS_INFORMATION_ELEMENTS: u16 = 6;
const NL80211_BSS_SIGNAL_MBM: u16 = 7;

/// The "privacy" bit of a BSS's capability field: set on any network that
/// wants a key.
const CAPABILITY_PRIVACY: u16 = 1 << 4;
/// Open System authentication.
const NL80211_AUTHTYPE_OPEN_SYSTEM: u32 = 0;
/// "Deauthenticated because sending station is leaving".
const REASON_LEAVING: u16 = 3;

// --- message building: pure ---

fn align4(n: usize) -> usize {
    (n + 3) & !3
}

/// One attribute: length, type, value, padded to four bytes.
pub fn attr(kind: u16, value: &[u8]) -> Vec<u8> {
    let length = 4 + value.len();
    let mut out = Vec::with_capacity(align4(length));
    out.extend_from_slice(&(length as u16).to_ne_bytes());
    out.extend_from_slice(&kind.to_ne_bytes());
    out.extend_from_slice(value);
    out.resize(align4(length), 0);
    out
}

/// A nested attribute holding `children` (already encoded).
pub fn nested(kind: u16, children: &[Vec<u8>]) -> Vec<u8> {
    attr(kind | NLA_F_NESTED, &children.concat())
}

/// A generic netlink message: the netlink header, the genl header, the
/// attributes.
pub fn genl_message(family: u16, flags: u16, seq: u32, cmd: u8, attrs: &[Vec<u8>]) -> Vec<u8> {
    let body = attrs.concat();
    let length = 16 + 4 + body.len();
    let mut out = Vec::with_capacity(length);
    out.extend_from_slice(&(length as u32).to_ne_bytes());
    out.extend_from_slice(&family.to_ne_bytes());
    out.extend_from_slice(&(NLM_F_REQUEST | flags).to_ne_bytes());
    out.extend_from_slice(&seq.to_ne_bytes());
    out.extend_from_slice(&0u32.to_ne_bytes());
    out.extend_from_slice(&[cmd, 1, 0, 0]);
    out.extend_from_slice(&body);
    out
}

/// Asking the controller for a family's id and multicast groups.
pub fn get_family(seq: u32, name: &str) -> Vec<u8> {
    let mut name = name.as_bytes().to_vec();
    name.push(0);
    genl_message(
        GENL_ID_CTRL,
        NLM_F_ACK,
        seq,
        CTRL_CMD_GETFAMILY,
        &[attr(CTRL_ATTR_FAMILY_NAME, &name)],
    )
}

/// An active scan (the wildcard SSID), so a quiet access point answers too.
pub fn trigger_scan(family: u16, seq: u32, ifindex: u32) -> Vec<u8> {
    trigger_scan_on(family, seq, ifindex, &[])
}

/// The 2.4 GHz channels 1-13, in MHz.
pub const CHANNELS_2_4_GHZ: [u32; 13] = [
    2412, 2417, 2422, 2427, 2432, 2437, 2442, 2447, 2452, 2457, 2462, 2467, 2472,
];

/// A scan of only these frequencies (MHz); every channel when empty. Fewer
/// channels finish sooner, so the same time holds more scans of the ones
/// that matter.
pub fn trigger_scan_on(family: u16, seq: u32, ifindex: u32, frequencies: &[u32]) -> Vec<u8> {
    let mut attributes = vec![
        attr(NL80211_ATTR_IFINDEX, &ifindex.to_ne_bytes()),
        nested(NL80211_ATTR_SCAN_SSIDS, &[attr(1, &[])]),
    ];
    if !frequencies.is_empty() {
        let list: Vec<Vec<u8>> = frequencies
            .iter()
            .enumerate()
            .map(|(index, mhz)| attr(index as u16 + 1, &mhz.to_ne_bytes()))
            .collect();
        attributes.push(nested(NL80211_ATTR_SCAN_FREQUENCIES, &list));
    }
    genl_message(
        family,
        NLM_F_ACK,
        seq,
        NL80211_CMD_TRIGGER_SCAN,
        &attributes,
    )
}

pub fn get_scan(family: u16, seq: u32, ifindex: u32) -> Vec<u8> {
    genl_message(
        family,
        NLM_F_DUMP,
        seq,
        NL80211_CMD_GET_SCAN,
        &[attr(NL80211_ATTR_IFINDEX, &ifindex.to_ne_bytes())],
    )
}

/// Associate with an open network. `pinned` names the scanned access point
/// and its channel; unpinned, only the name, and the firmware picks (it
/// rescans, so a stale scan result cannot be what it is refused for).
pub fn connect(family: u16, seq: u32, ifindex: u32, bss: &Bss, pinned: bool) -> Vec<u8> {
    let mut attributes = vec![
        attr(NL80211_ATTR_IFINDEX, &ifindex.to_ne_bytes()),
        attr(NL80211_ATTR_SSID, bss.ssid.as_bytes()),
    ];
    if pinned {
        attributes.push(attr(NL80211_ATTR_MAC, &bss.bssid));
        attributes.push(attr(NL80211_ATTR_WIPHY_FREQ, &bss.frequency.to_ne_bytes()));
    }
    attributes.push(attr(
        NL80211_ATTR_AUTH_TYPE,
        &NL80211_AUTHTYPE_OPEN_SYSTEM.to_ne_bytes(),
    ));
    genl_message(family, NLM_F_ACK, seq, NL80211_CMD_CONNECT, &attributes)
}

/// Move radio `wiphy` (and its interfaces) into the network namespace of
/// process `pid`.
pub fn set_wiphy_netns(family: u16, seq: u32, wiphy: u32, pid: u32) -> Vec<u8> {
    genl_message(
        family,
        NLM_F_REQUEST | NLM_F_ACK,
        seq,
        NL80211_CMD_SET_WIPHY_NETNS,
        &[
            attr(NL80211_ATTR_WIPHY, &wiphy.to_ne_bytes()),
            attr(NL80211_ATTR_PID, &pid.to_ne_bytes()),
        ],
    )
}

pub fn disconnect(family: u16, seq: u32, ifindex: u32) -> Vec<u8> {
    genl_message(
        family,
        NLM_F_ACK,
        seq,
        NL80211_CMD_DISCONNECT,
        &[
            attr(NL80211_ATTR_IFINDEX, &ifindex.to_ne_bytes()),
            attr(NL80211_ATTR_REASON_CODE, &REASON_LEAVING.to_ne_bytes()),
        ],
    )
}

// --- parsing: pure ---

/// One netlink message out of a datagram.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Message {
    pub kind: u16,
    pub seq: u32,
    /// For a generic netlink message, its command; 0 otherwise.
    pub cmd: u8,
    /// The attributes (after the genl header), or for `NLMSG_ERROR` the
    /// error payload.
    pub payload: Vec<u8>,
}

impl Message {
    /// The errno of an `NLMSG_ERROR` (0 is an acknowledgement).
    pub fn error(&self) -> Option<i32> {
        (self.kind == NLMSG_ERROR)
            .then(|| {
                self.payload
                    .get(..4)
                    .map(|b| i32::from_ne_bytes(b.try_into().unwrap()))
            })
            .flatten()
    }
}

/// Every message in one datagram.
pub fn messages(datagram: &[u8]) -> Vec<Message> {
    let mut out = Vec::new();
    let mut at = 0;
    while at + 16 <= datagram.len() {
        let length = u32::from_ne_bytes(datagram[at..at + 4].try_into().unwrap()) as usize;
        if length < 16 || at + length > datagram.len() {
            break;
        }
        let kind = u16::from_ne_bytes(datagram[at + 4..at + 6].try_into().unwrap());
        let seq = u32::from_ne_bytes(datagram[at + 8..at + 12].try_into().unwrap());
        let body = &datagram[at + 16..at + length];
        let (cmd, payload) = if kind == NLMSG_ERROR || kind == NLMSG_DONE || body.len() < 4 {
            (0, body.to_vec())
        } else {
            (body[0], body[4..].to_vec())
        };
        out.push(Message {
            kind,
            seq,
            cmd,
            payload,
        });
        at += align4(length);
    }
    out
}

/// The attributes in `bytes`, as (type without flags, value).
pub fn attrs(bytes: &[u8]) -> Vec<(u16, &[u8])> {
    let mut out = Vec::new();
    let mut at = 0;
    while at + 4 <= bytes.len() {
        let length = u16::from_ne_bytes([bytes[at], bytes[at + 1]]) as usize;
        let kind = u16::from_ne_bytes([bytes[at + 2], bytes[at + 3]]) & NLA_TYPE_MASK;
        if length < 4 || at + length > bytes.len() {
            break;
        }
        out.push((kind, &bytes[at + 4..at + length]));
        at += align4(length);
    }
    out
}

fn find(bytes: &[u8], kind: u16) -> Option<&[u8]> {
    attrs(bytes)
        .into_iter()
        .find(|(k, _)| *k == kind)
        .map(|(_, v)| v)
}

fn u16_of(value: &[u8]) -> Option<u16> {
    Some(u16::from_ne_bytes(value.get(..2)?.try_into().ok()?))
}

fn u32_of(value: &[u8]) -> Option<u32> {
    Some(u32::from_ne_bytes(value.get(..4)?.try_into().ok()?))
}

/// A family's id and its multicast groups by name, from the controller's
/// answer.
pub fn family_of(payload: &[u8]) -> Option<(u16, Vec<(String, u32)>)> {
    let id = u16_of(find(payload, CTRL_ATTR_FAMILY_ID)?)?;
    let groups = find(payload, CTRL_ATTR_MCAST_GROUPS)
        .map(|groups| {
            attrs(groups)
                .into_iter()
                .filter_map(|(_, group)| {
                    let name = find(group, CTRL_ATTR_MCAST_GRP_NAME)?;
                    let name = String::from_utf8_lossy(name)
                        .trim_end_matches('\0')
                        .to_string();
                    Some((name, u32_of(find(group, CTRL_ATTR_MCAST_GRP_ID)?)?))
                })
                .collect()
        })
        .unwrap_or_default();
    Some((id, groups))
}

/// One network the scan saw.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Bss {
    pub bssid: [u8; 6],
    pub ssid: String,
    pub frequency: u32,
    /// Whether it wants a key.
    pub private: bool,
    /// Signal in hundredths of a dBm.
    pub signal_mbm: i32,
}

/// The SSID element (id 0) out of a BSS's information elements.
pub fn ssid_of(elements: &[u8]) -> Option<String> {
    let mut at = 0;
    while at + 2 <= elements.len() {
        let (id, length) = (elements[at], elements[at + 1] as usize);
        let value = elements.get(at + 2..at + 2 + length)?;
        if id == 0 {
            return Some(String::from_utf8_lossy(value).into_owned());
        }
        at += 2 + length;
    }
    None
}

/// One BSS out of a scan dump message.
pub fn bss_of(payload: &[u8]) -> Option<Bss> {
    let bss = find(payload, NL80211_ATTR_BSS)?;
    Some(Bss {
        bssid: find(bss, NL80211_BSS_BSSID)?.try_into().ok()?,
        ssid: ssid_of(find(bss, NL80211_BSS_INFORMATION_ELEMENTS)?)?,
        frequency: u32_of(find(bss, NL80211_BSS_FREQUENCY)?)?,
        private: find(bss, NL80211_BSS_CAPABILITY)
            .and_then(u16_of)
            .is_some_and(|capability| capability & CAPABILITY_PRIVACY != 0),
        signal_mbm: find(bss, NL80211_BSS_SIGNAL_MBM)
            .and_then(u32_of)
            .map_or(i32::MIN, |raw| raw as i32),
    })
}

// --- the socket: thin ---

/// A generic netlink socket bound to nl80211.
pub struct Radio {
    fd: libc::c_int,
    family: u16,
    ifindex: u32,
    seq: u32,
}

impl Drop for Radio {
    fn drop(&mut self) {
        unsafe { libc::close(self.fd) };
    }
}

fn os_error(what: &str) -> String {
    format!("{what}: {}", std::io::Error::last_os_error())
}

impl Radio {
    /// Open the radio, bring the interface up, and join nl80211's `scan` and
    /// `mlme` groups (where scan results and connect results are announced).
    pub fn open() -> Result<Self, String> {
        Self::open_as(true)
    }

    /// The radio's nl80211 socket; brought up and listening for its scan
    /// and connect events when `listen`, untouched otherwise (to hand it
    /// over: an interface that is up is closed by the move, a firmware round
    /// trip for nothing).
    pub fn open_as(listen: bool) -> Result<Self, String> {
        let name = std::ffi::CString::new(INTERFACE).expect("no NUL");
        let ifindex = unsafe { libc::if_nametoindex(name.as_ptr()) };
        if ifindex == 0 {
            return Err(format!("no {INTERFACE}: the radio's driver is not loaded"));
        }
        if listen {
            set_link(INTERFACE, true)?;
        }
        let fd = unsafe {
            libc::socket(
                libc::AF_NETLINK,
                libc::SOCK_RAW | libc::SOCK_CLOEXEC,
                NETLINK_GENERIC,
            )
        };
        if fd < 0 {
            return Err(os_error("a netlink socket"));
        }
        // A receive timeout, so a radio that never answers is given up on.
        let timeout = libc::timeval {
            tv_sec: 2,
            tv_usec: 0,
        };
        unsafe {
            libc::setsockopt(
                fd,
                libc::SOL_SOCKET,
                libc::SO_RCVTIMEO,
                (&timeout as *const libc::timeval).cast(),
                std::mem::size_of::<libc::timeval>() as libc::socklen_t,
            );
        }
        let mut radio = Self {
            fd,
            family: GENL_ID_CTRL,
            ifindex,
            seq: 1,
        };
        let seq = radio.next_seq();
        radio.send(&get_family(seq, "nl80211"))?;
        let (family, groups) = radio
            .replies(seq)?
            .iter()
            .find_map(|message| family_of(&message.payload))
            .ok_or("this kernel has no nl80211")?;
        radio.family = family;
        for (name, id) in groups.into_iter().filter(|_| listen) {
            if name == "scan" || name == "mlme" {
                let rc = unsafe {
                    libc::setsockopt(
                        fd,
                        SOL_NETLINK,
                        NETLINK_ADD_MEMBERSHIP,
                        (&id as *const u32).cast(),
                        4,
                    )
                };
                if rc != 0 {
                    return Err(os_error("joining nl80211's events"));
                }
            }
        }
        Ok(radio)
    }

    fn next_seq(&mut self) -> u32 {
        self.seq = self.seq.wrapping_add(1);
        self.seq
    }

    fn send(&self, message: &[u8]) -> Result<(), String> {
        let mut to: libc::sockaddr_nl = unsafe { std::mem::zeroed() };
        to.nl_family = libc::AF_NETLINK as libc::sa_family_t;
        let sent = unsafe {
            libc::sendto(
                self.fd,
                message.as_ptr().cast(),
                message.len(),
                0,
                (&to as *const libc::sockaddr_nl).cast(),
                std::mem::size_of::<libc::sockaddr_nl>() as libc::socklen_t,
            )
        };
        if sent < 0 {
            return Err(os_error("sending to the radio"));
        }
        Ok(())
    }

    /// One datagram's messages, or none on the receive timeout.
    fn receive(&self) -> Result<Vec<Message>, String> {
        let mut buffer = vec![0u8; 32 * 1024];
        let read = unsafe { libc::recv(self.fd, buffer.as_mut_ptr().cast(), buffer.len(), 0) };
        if read < 0 {
            let error = std::io::Error::last_os_error();
            return match error.raw_os_error() {
                Some(libc::EAGAIN) | Some(libc::EINTR) => Ok(Vec::new()),
                _ => Err(format!("reading from the radio: {error}")),
            };
        }
        Ok(messages(&buffer[..read as usize]))
    }

    /// The answers to request `seq`, to its acknowledgement or end of dump;
    /// a kernel error is returned as one. Events in between are dropped.
    fn replies(&self, seq: u32) -> Result<Vec<Message>, String> {
        self.replies_within(seq, Duration::from_secs(5))
    }

    fn replies_within(&self, seq: u32, limit: Duration) -> Result<Vec<Message>, String> {
        let deadline = Instant::now() + limit;
        let mut out = Vec::new();
        while Instant::now() < deadline {
            for message in self.receive()? {
                if message.seq != seq {
                    continue;
                }
                match message.error() {
                    Some(0) => return Ok(out),
                    Some(errno) => {
                        return Err(format!(
                            "the radio refused: {}",
                            std::io::Error::from_raw_os_error(-errno)
                        ));
                    }
                    None if message.kind == NLMSG_DONE => return Ok(out),
                    None => out.push(message),
                }
            }
        }
        Err("the radio did not answer".to_string())
    }

    /// Wait for an event with one of `commands`, up to `limit`.
    fn event(&self, commands: &[u8], limit: Duration) -> Result<Option<Message>, String> {
        let deadline = Instant::now() + limit;
        while Instant::now() < deadline {
            for message in self.receive()? {
                if message.seq == 0 && commands.contains(&message.cmd) {
                    return Ok(Some(message));
                }
            }
        }
        Ok(None)
    }

    /// Scan and return every network seen.
    pub fn scan(&mut self) -> Result<Vec<Bss>, String> {
        self.scan_on(&[])
    }

    /// Scan only these frequencies (MHz); every channel when empty.
    pub fn scan_on(&mut self, frequencies: &[u32]) -> Result<Vec<Bss>, String> {
        let seq = self.next_seq();
        self.send(&trigger_scan_on(
            self.family,
            seq,
            self.ifindex,
            frequencies,
        ))?;
        self.replies(seq)?;
        match self.event(
            &[NL80211_CMD_NEW_SCAN_RESULTS, NL80211_CMD_SCAN_ABORTED],
            Duration::from_secs(15),
        )? {
            Some(message) if message.cmd == NL80211_CMD_SCAN_ABORTED => {
                return Err("the scan was aborted".to_string());
            }
            Some(_) => {}
            None => return Err("the scan did not finish".to_string()),
        }
        let seq = self.next_seq();
        self.send(&get_scan(self.family, seq, self.ifindex))?;
        Ok(self
            .replies(seq)?
            .iter()
            .filter_map(|message| bss_of(&message.payload))
            .collect())
    }

    /// Associate with an open network and wait for the result.
    pub fn connect(&mut self, bss: &Bss, pinned: bool) -> Result<(), String> {
        let seq = self.next_seq();
        self.send(&connect(self.family, seq, self.ifindex, bss, pinned))?;
        self.replies(seq)?;
        let result = self
            .event(&[NL80211_CMD_CONNECT], Duration::from_secs(15))?
            .ok_or("no answer from the access point")?;
        match find(&result.payload, NL80211_ATTR_STATUS_CODE).and_then(u16_of) {
            Some(0) => Ok(()),
            Some(code) => Err(format!("the access point refused us (status {code})")),
            None => Err("the connect result carried no status".to_string()),
        }
    }

    /// Hand the radio to the network namespace of process `pid`: the phy
    /// moves, its interface with it, and both come back to this namespace
    /// when that one ends. cfg80211 interfaces cannot be moved on their own;
    /// the phy can, when its driver allows it (brcmfmac does).
    pub fn lend_to(&mut self, pid: u32) -> Result<Lent, String> {
        let wiphy: u32 =
            std::fs::read_to_string(format!("/sys/class/net/{INTERFACE}/phy80211/index"))
                .map_err(|error| format!("which radio {INTERFACE} is: {error}"))?
                .trim()
                .parse()
                .map_err(|_| "an unreadable radio index".to_string())?;
        let started = Instant::now();
        let seq = self.next_seq();
        self.send(&set_wiphy_netns(self.family, seq, wiphy, pid))?;
        // The kernel's answer, or failing that the interface gone from this
        // namespace: some kernels have been seen not to answer within five
        // seconds, leaving whether the radio moved untold.
        let answered = self.replies_within(seq, Duration::from_secs(30));
        let gone = !std::path::Path::new(&format!("/sys/class/net/{INTERFACE}")).exists();
        match (answered, gone) {
            (Ok(_), _) | (Err(_), true) => Ok(Lent {
                wiphy,
                took: started.elapsed(),
                still_visible: !gone,
            }),
            (Err(error), false) => Err(format!(
                "{error} after {} ms, and {INTERFACE} is still here",
                started.elapsed().as_millis()
            )),
        }
    }

    pub fn disconnect(&mut self) -> Result<(), String> {
        let seq = self.next_seq();
        self.send(&disconnect(self.family, seq, self.ifindex))?;
        // Not connected is fine: leaving is the point.
        let _ = self.replies(seq);
        Ok(())
    }
}

// --- addresses, by ioctl ---

fn ifreq(name: &str) -> libc::ifreq {
    let mut request: libc::ifreq = unsafe { std::mem::zeroed() };
    for (slot, byte) in request.ifr_name.iter_mut().zip(name.bytes()) {
        *slot = byte as libc::c_char;
    }
    request
}

fn ioctl_socket() -> Result<libc::c_int, String> {
    let fd = unsafe { libc::socket(libc::AF_INET, libc::SOCK_DGRAM | libc::SOCK_CLOEXEC, 0) };
    if fd < 0 {
        return Err(os_error("an address socket"));
    }
    Ok(fd)
}

pub fn set_link(name: &str, up: bool) -> Result<(), String> {
    let fd = ioctl_socket()?;
    let mut request = ifreq(name);
    let result = unsafe {
        if libc::ioctl(fd, libc::SIOCGIFFLAGS as _, &mut request) < 0 {
            Err(os_error("reading the radio's flags"))
        } else {
            let flags = request.ifr_ifru.ifru_flags;
            request.ifr_ifru.ifru_flags = if up {
                flags | libc::IFF_UP as libc::c_short
            } else {
                flags & !(libc::IFF_UP as libc::c_short)
            };
            if libc::ioctl(fd, libc::SIOCSIFFLAGS as _, &request) < 0 {
                Err(os_error("bringing the radio up or down"))
            } else {
                Ok(())
            }
        }
    };
    unsafe { libc::close(fd) };
    result
}

fn sockaddr(address: Ipv4Addr) -> libc::sockaddr {
    let mut inet: libc::sockaddr_in = unsafe { std::mem::zeroed() };
    inet.sin_family = libc::AF_INET as libc::sa_family_t;
    inet.sin_addr.s_addr = u32::from_ne_bytes(address.octets());
    unsafe { std::mem::transmute(inet) }
}

/// Give the interface an address and a /24 (the kernel adds the route), or
/// take it away with 0.0.0.0.
pub fn set_address(name: &str, address: Ipv4Addr) -> Result<(), String> {
    let fd = ioctl_socket()?;
    let mut request = ifreq(name);
    request.ifr_ifru.ifru_addr = sockaddr(address);
    let mut result = if unsafe { libc::ioctl(fd, libc::SIOCSIFADDR as _, &request) } < 0 {
        Err(os_error("setting the radio's address"))
    } else {
        Ok(())
    };
    if result.is_ok() && !address.is_unspecified() {
        request.ifr_ifru.ifru_netmask = sockaddr(Ipv4Addr::new(255, 255, 255, 0));
        if unsafe { libc::ioctl(fd, libc::SIOCSIFNETMASK as _, &request) } < 0 {
            result = Err(os_error("setting the radio's netmask"));
        }
    }
    unsafe { libc::close(fd) };
    result
}

// --- composed ---

/// What lending the radio did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Lent {
    /// The phy that moved.
    pub wiphy: u32,
    /// How long the move took.
    pub took: Duration,
    /// Whether this namespace can still see the interface (the kernel did
    /// not answer, but it moved).
    pub still_visible: bool,
}

/// Lend this machine's radio to the network namespace of process `pid`.
pub fn lend_radio(pid: u32) -> Result<Lent, String> {
    Radio::open_as(false)?.lend_to(pid)
}

/// Join an open network, with retries. brcmfmac answers every failed join
/// with status 16 whatever the cause, so it is tried three times, two seconds
/// apart, the last two letting the firmware choose the access point; and a
/// failure says the channel and the signal, the two things it most often is.
pub fn join(radio: &mut Radio, bss: &Bss) -> Result<(), String> {
    let mut failures = Vec::new();
    for (attempt, pinned) in [true, false, false].into_iter().enumerate() {
        if attempt > 0 {
            let _ = radio.disconnect();
            std::thread::sleep(Duration::from_secs(2));
        }
        match radio.connect(bss, pinned) {
            Ok(()) => return Ok(()),
            Err(error) => failures.push(error),
        }
    }
    Err(format!(
        "could not join it on {} MHz at {} dBm, three tries: {}",
        bss.frequency,
        bss.signal_mbm / 100,
        failures.join("; ")
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A scan dump message as the kernel sends it: header, genl header, a
    /// nested BSS.
    fn scan_message(ssid: &str, capability: u16, signal: i32) -> Vec<u8> {
        let mut elements = vec![0, ssid.len() as u8];
        elements.extend_from_slice(ssid.as_bytes());
        elements.extend_from_slice(&[1, 2, 0x82, 0x84]); // supported rates
        genl_message(
            30,
            0,
            0,
            NL80211_CMD_NEW_SCAN_RESULTS,
            &[
                attr(NL80211_ATTR_IFINDEX, &3u32.to_ne_bytes()),
                nested(
                    NL80211_ATTR_BSS,
                    &[
                        attr(NL80211_BSS_BSSID, &[0xfc, 0x67, 0x1f, 0, 0, 1]),
                        attr(NL80211_BSS_FREQUENCY, &2437u32.to_ne_bytes()),
                        attr(NL80211_BSS_CAPABILITY, &capability.to_ne_bytes()),
                        attr(NL80211_BSS_INFORMATION_ELEMENTS, &elements),
                        attr(NL80211_BSS_SIGNAL_MBM, &(signal as u32).to_ne_bytes()),
                    ],
                ),
            ],
        )
    }

    /// `replies` ends on the kernel's acknowledgement or a dump's end, so
    /// every request it waits on must ask for one or be a dump. The family
    /// lookup asked for neither: the kernel answered once, `replies` waited
    /// out its five seconds for an acknowledgement that never came, and
    /// every `Radio::open` failed with "the radio did not answer".
    #[test]
    fn every_request_the_radio_waits_on_ends_in_an_answer() {
        let flags = |message: &[u8]| u16::from_ne_bytes([message[6], message[7]]);
        let bss = Bss {
            ssid: "x".into(),
            bssid: [0; 6],
            frequency: 2437,
            private: false,
            signal_mbm: 0,
        };
        for (what, message) in [
            ("get_family", get_family(1, "nl80211")),
            ("trigger_scan", trigger_scan(30, 1, 3)),
            ("get_scan", get_scan(30, 1, 3)),
            ("connect", connect(30, 1, 3, &bss, true)),
            ("connect by name", connect(30, 1, 3, &bss, false)),
            ("disconnect", disconnect(30, 1, 3)),
            ("set_wiphy_netns", set_wiphy_netns(30, 1, 0, 42)),
        ] {
            let flags = flags(&message);
            assert!(
                flags & NLM_F_ACK != 0 || flags & NLM_F_DUMP == NLM_F_DUMP,
                "{what} asks for no answer that ends it ({flags:#x})"
            );
        }
    }

    #[test]
    fn a_scan_can_be_held_to_some_channels() {
        let all = messages(&trigger_scan(30, 1, 3));
        assert!(
            !attrs(&all[0].payload)
                .iter()
                .any(|(kind, _)| *kind == NL80211_ATTR_SCAN_FREQUENCIES),
            "an unrestricted scan names no frequencies"
        );
        let some = messages(&trigger_scan_on(30, 1, 3, &CHANNELS_2_4_GHZ));
        let fields = attrs(&some[0].payload);
        let list = fields
            .iter()
            .find(|(kind, _)| *kind == NL80211_ATTR_SCAN_FREQUENCIES)
            .map(|(_, value)| attrs(value))
            .expect("the frequencies");
        let mhz: Vec<u32> = list
            .iter()
            .map(|(_, value)| u32::from_ne_bytes((*value).try_into().expect("4 bytes")))
            .collect();
        assert_eq!(mhz, CHANNELS_2_4_GHZ.to_vec());
        assert!(
            fields
                .iter()
                .any(|(kind, _)| *kind == NL80211_ATTR_SCAN_SSIDS)
        );
    }

    #[test]
    fn attributes_are_padded_to_four_bytes_and_read_back() {
        let one = attr(52, b"abcde");
        assert_eq!(one.len(), 12);
        assert_eq!(u16::from_ne_bytes([one[0], one[1]]), 9);
        let both = [one, attr(3, &7u32.to_ne_bytes())].concat();
        let read = attrs(&both);
        assert_eq!(
            read,
            vec![(52, &b"abcde"[..]), (3, &7u32.to_ne_bytes()[..])]
        );
        // The nested flag is not part of the type.
        let n = nested(45, &[attr(1, &[])]);
        assert_eq!(attrs(&n)[0].0, 45);
        assert_eq!(attrs(attrs(&n)[0].1), vec![(1, &[][..])]);
    }

    #[test]
    fn a_request_has_the_netlink_and_genl_headers() {
        let message = connect(
            30,
            9,
            3,
            &Bss {
                bssid: [1, 2, 3, 4, 5, 6],
                ssid: "Example Network".to_string(),
                frequency: 2437,
                private: false,
                signal_mbm: -4000,
            },
            true,
        );
        assert_eq!(
            u32::from_ne_bytes(message[..4].try_into().unwrap()) as usize,
            message.len()
        );
        assert_eq!(u16::from_ne_bytes([message[4], message[5]]), 30);
        assert_eq!(
            u16::from_ne_bytes([message[6], message[7]]),
            NLM_F_REQUEST | NLM_F_ACK
        );
        let parsed = messages(&message);
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].seq, 9);
        assert_eq!(parsed[0].cmd, NL80211_CMD_CONNECT);
        let fields = attrs(&parsed[0].payload);
        assert!(fields.contains(&(NL80211_ATTR_SSID, &b"Example Network"[..])));
        assert!(fields.contains(&(NL80211_ATTR_MAC, &[1, 2, 3, 4, 5, 6][..])));
        assert!(fields.contains(&(NL80211_ATTR_AUTH_TYPE, &0u32.to_ne_bytes()[..])));
    }

    #[test]
    fn a_join_by_name_leaves_the_access_point_and_channel_to_the_firmware() {
        let bss = Bss {
            bssid: [1, 2, 3, 4, 5, 6],
            ssid: "Example Network 2".to_string(),
            frequency: 2462,
            private: false,
            signal_mbm: -6000,
        };
        let fields_of = |message: Vec<u8>| -> Vec<u16> {
            let parsed = messages(&message);
            attrs(&parsed[0].payload)
                .iter()
                .map(|(kind, _)| *kind)
                .collect()
        };
        let pinned = fields_of(connect(30, 1, 3, &bss, true));
        assert!(pinned.contains(&NL80211_ATTR_MAC) && pinned.contains(&NL80211_ATTR_WIPHY_FREQ));
        let by_name = fields_of(connect(30, 1, 3, &bss, false));
        assert!(by_name.contains(&NL80211_ATTR_SSID));
        assert!(by_name.contains(&NL80211_ATTR_AUTH_TYPE));
        assert!(!by_name.contains(&NL80211_ATTR_MAC), "{by_name:?}");
        assert!(!by_name.contains(&NL80211_ATTR_WIPHY_FREQ), "{by_name:?}");
    }

    #[test]
    fn a_scan_result_is_read_into_a_network() {
        let message = scan_message("Example Network", 0x0401, -4200);
        let parsed = messages(&message);
        let bss = bss_of(&parsed[0].payload).expect("a network");
        assert_eq!(bss.ssid, "Example Network");
        assert_eq!(bss.frequency, 2437);
        assert_eq!(bss.bssid, [0xfc, 0x67, 0x1f, 0, 0, 1]);
        assert!(!bss.private);
        assert_eq!(bss.signal_mbm, -4200);
        let locked = messages(&scan_message("Home", 0x0411, -3000));
        assert!(bss_of(&locked[0].payload).expect("a network").private);
    }

    #[test]
    fn several_messages_in_one_datagram_and_an_ack_are_told_apart() {
        let mut ack = Vec::new();
        ack.extend_from_slice(&36u32.to_ne_bytes());
        ack.extend_from_slice(&NLMSG_ERROR.to_ne_bytes());
        ack.extend_from_slice(&0u16.to_ne_bytes());
        ack.extend_from_slice(&5u32.to_ne_bytes());
        ack.extend_from_slice(&0u32.to_ne_bytes());
        ack.extend_from_slice(&0i32.to_ne_bytes());
        ack.extend_from_slice(&[0u8; 16]); // the echoed request header
        let mut refused = ack.clone();
        refused[16..20].copy_from_slice(&(-16i32).to_ne_bytes());
        let datagram = [scan_message("A", 0, 0), ack, refused].concat();
        let parsed = messages(&datagram);
        assert_eq!(parsed.len(), 3);
        assert_eq!(parsed[0].error(), None);
        assert_eq!(parsed[1].error(), Some(0));
        assert_eq!(parsed[2].error(), Some(-16));
        // A truncated datagram yields what is whole and stops.
        assert_eq!(messages(&datagram[..datagram.len() - 3]).len(), 2);
    }

    #[test]
    fn the_family_answer_gives_the_id_and_groups() {
        let groups = nested(
            CTRL_ATTR_MCAST_GROUPS,
            &[
                nested(
                    1,
                    &[
                        attr(CTRL_ATTR_MCAST_GRP_ID, &5u32.to_ne_bytes()),
                        attr(CTRL_ATTR_MCAST_GRP_NAME, b"scan\0"),
                    ],
                ),
                nested(
                    2,
                    &[
                        attr(CTRL_ATTR_MCAST_GRP_ID, &7u32.to_ne_bytes()),
                        attr(CTRL_ATTR_MCAST_GRP_NAME, b"mlme\0"),
                    ],
                ),
            ],
        );
        let payload = [attr(CTRL_ATTR_FAMILY_ID, &30u16.to_ne_bytes()), groups].concat();
        let (id, groups) = family_of(&payload).expect("a family");
        assert_eq!(id, 30);
        assert_eq!(
            groups,
            vec![("scan".to_string(), 5), ("mlme".to_string(), 7)]
        );
    }

    #[test]
    fn a_hidden_or_malformed_ssid_element_is_not_a_name() {
        assert_eq!(ssid_of(&[0, 3, b'a', b'b', b'c']), Some("abc".to_string()));
        assert_eq!(
            ssid_of(&[1, 1, 0x82, 0, 2, b'h', b'i']),
            Some("hi".to_string())
        );
        assert_eq!(ssid_of(&[0, 9, b'a']), None);
        assert_eq!(ssid_of(&[]), None);
    }
}
