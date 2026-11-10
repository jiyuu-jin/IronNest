use {
    crate::{
        error::Result,
        protocol::encrypt,
        types::{DeviceData, GetSysInfo, TPLinkDiscoveryRes, TPLinkDiscoverySysInfo},
    },
    log::{info, trace, warn},
    std::net::Ipv4Addr,
    tokio::{
        net::UdpSocket,
        time::{timeout, Duration},
    },
};

/// Discovers TP-Link devices on the local network using UDP broadcast
pub async fn discover_devices() -> Result<Vec<DeviceData>> {
    let port = 9999;
    let socket = UdpSocket::bind((Ipv4Addr::UNSPECIFIED, port)).await?;
    socket.set_broadcast(true)?;

    let request = TPLinkDiscoveryRes {
        system: TPLinkDiscoverySysInfo {
            get_sysinfo: GetSysInfo::Empty(()),
        },
    };

    let msg_bytes = serde_json::to_vec(&request)?;
    let discover_msg = encrypt(&msg_bytes);

    let broadcast_addr = (Ipv4Addr::BROADCAST, port);
    socket.send_to(&discover_msg, broadcast_addr).await?;

    let mut buf = [0; 2048];
    let timeout_duration = Duration::from_millis(2500);
    let mut devices = Vec::with_capacity(20);

    loop {
        match timeout(timeout_duration, socket.recv_from(&mut buf)).await {
            Ok(Ok((num_bytes, src_addr))) => {
                let incoming_data = crate::protocol::decrypt(&buf);
                let incoming_msg_result =
                    serde_json::from_slice::<TPLinkDiscoveryRes>(&incoming_data[..num_bytes]);

                match incoming_msg_result {
                    Ok(msg) => match msg.system.get_sysinfo {
                        GetSysInfo::TPLinkDiscoveryData(mut get_sysinfo) => {
                            info!(
                                "Smart Plug or Dimmer from {}: {}",
                                src_addr, get_sysinfo.alias
                            );
                            get_sysinfo.ip = Some(src_addr.ip());

                            if get_sysinfo.model == "ES20M(US)" || get_sysinfo.model == "KS230(US)"
                            {
                                devices.push(DeviceData::SmartDimmer(get_sysinfo));
                            } else {
                                devices.push(DeviceData::SmartPlug(get_sysinfo));
                            }
                        }
                        GetSysInfo::TPLinkSmartLightData(mut get_sysinfo) => {
                            info!("Smart Light from {}: {}", src_addr, get_sysinfo.alias);
                            get_sysinfo.ip = Some(src_addr.ip());
                            devices.push(DeviceData::SmartLight(get_sysinfo));
                        }
                        GetSysInfo::TPLinkSmartPowerStripData(mut get_sysinfo) => {
                            info!("Smart Power Strip from {}: {}", src_addr, get_sysinfo.alias);
                            get_sysinfo.ip = Some(src_addr.ip());
                            devices.push(DeviceData::SmartPowerStrip(get_sysinfo));
                        }
                        GetSysInfo::Empty(()) => trace!("ignoring GetSysInfo::Empty(())"),
                        GetSysInfo::CatchAll(raw_json) => {
                            warn!("Catch-all variant triggered, raw JSON: {:?}", raw_json);
                        }
                    },
                    Err(e) => {
                        let valid_length = incoming_data
                            .iter()
                            .take_while(|&&byte| std::str::from_utf8(&[byte]).is_ok())
                            .count();

                        let valid_data = &incoming_data[..valid_length];
                        let string_value =
                            std::str::from_utf8(valid_data).unwrap_or("Failed to convert to UTF-8");

                        warn!("Error parsing broadcast response: {e}, {:?}", string_value);
                    }
                }
            }
            Ok(Err(e)) => {
                warn!("Error receiving broadcast response: {}", e);
                break;
            }
            Err(_) => {
                trace!("Timeout reached, no more responses.");
                break;
            }
        }
    }

    Ok(devices)
}

/// A device that answered a [`scan`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Found {
    pub addr: std::net::SocketAddr,
    pub alias: String,
    pub model: String,
    /// The outlets, when it is a power strip.
    pub outlets: Option<Vec<crate::devices::Outlet>>,
}

/// Every address of an IPv4 network but its network and broadcast addresses
/// (`10.0.0.0/24` gives 10.0.0.1 to 10.0.0.254). Refuses anything wider than
/// a /16, which would be 65 thousand connections.
pub fn hosts_in(network: Ipv4Addr, prefix: u8) -> Result<Vec<std::net::IpAddr>> {
    if !(16..=30).contains(&prefix) {
        return Err(crate::error::TpLinkError::DeviceCommunication {
            message: format!("scan a /16 to a /30, not a /{prefix}"),
        });
    }
    let mask = u32::MAX << (32 - prefix);
    let base = u32::from(network) & mask;
    let size = 1u32 << (32 - prefix);
    Ok((1..size - 1)
        .map(|offset| std::net::IpAddr::V4(Ipv4Addr::from(base + offset)))
        .collect())
}

/// Find devices by asking each address for its system information over TCP,
/// at most `parallel` at a time, giving each `wait` to answer.
///
/// [`discover_devices`] asks by UDP broadcast, which is quicker but does not
/// leave a container on a bridged network or cross a router; this works from
/// anywhere that can reach the addresses.
pub async fn scan(addrs: Vec<std::net::SocketAddr>, wait: Duration, parallel: usize) -> Vec<Found> {
    use {std::sync::Arc, tokio::sync::Semaphore};
    let permits = Arc::new(Semaphore::new(parallel.max(1)));
    let mut tasks = Vec::with_capacity(addrs.len());
    for addr in addrs {
        let permits = Arc::clone(&permits);
        tasks.push(tokio::spawn(async move {
            let _permit = permits.acquire_owned().await.ok()?;
            let client = crate::client::TpLinkClient::new();
            let ask = client.send_to(addr, serde_json::json!({"system": {"get_sysinfo": {}}}));
            let answer = timeout(wait, ask).await.ok()?.ok()?;
            let info = answer.pointer("/system/get_sysinfo")?;
            let text = |key: &str| {
                info.get(key)
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or("")
                    .to_string()
            };
            Some(Found {
                addr,
                alias: text("alias"),
                model: text("model"),
                outlets: crate::devices::parse_strip_info(&answer)
                    .ok()
                    .map(|strip| strip.outlets),
            })
        }));
    }
    let mut found = Vec::new();
    for task in tasks {
        if let Ok(Some(device)) = task.await {
            found.push(device);
        }
    }
    found.sort_by_key(|device| device.addr);
    found
}

#[cfg(test)]
mod tests {
    use {super::*, crate::client::fake, serde_json::json};

    #[test]
    fn a_slash_24_is_its_254_hosts() {
        let hosts = hosts_in(Ipv4Addr::new(10, 0, 0, 77), 24).unwrap();
        assert_eq!(hosts.len(), 254);
        assert_eq!(hosts[0].to_string(), "10.0.0.1");
        assert_eq!(hosts[253].to_string(), "10.0.0.254");
        assert!(hosts_in(Ipv4Addr::new(10, 0, 0, 0), 8).is_err());
    }

    #[tokio::test]
    async fn a_scan_reports_what_answered_and_skips_what_did_not() {
        let (strip, _) = fake::device(|_| {
            json!({"system": {"get_sysinfo": {
                "alias": "Server rack", "model": "HS300(US)", "deviceId": "AB",
                "children": [{"id": "AB00", "alias": "lemon", "state": 1}]
            }}})
        })
        .await;
        let (plug, _) = fake::device(
            |_| json!({"system": {"get_sysinfo": {"alias": "White light", "model": "HS103(US)"}}}),
        )
        .await;
        // A port nothing listens on.
        let closed = {
            let socket = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
            socket.local_addr().unwrap()
        };
        let found = scan(vec![strip, closed, plug], Duration::from_secs(2), 8).await;
        assert_eq!(found.len(), 2);
        let rack = found.iter().find(|d| d.addr == strip).unwrap();
        assert_eq!(rack.model, "HS300(US)");
        assert_eq!(rack.outlets.as_ref().unwrap()[0].alias, "lemon");
        let light = found.iter().find(|d| d.addr == plug).unwrap();
        assert_eq!(light.alias, "White light");
        assert!(light.outlets.is_none());
    }
}
