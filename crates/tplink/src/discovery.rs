use {
    crate::{
        client::TpLinkClient,
        devices::{Device, PowerStrip, SmartDimmer, SmartLight, SmartPlug},
        error::Result,
        protocol::encrypt,
        types::{DeviceData, GetSysInfo, TPLinkDiscoveryRes, TPLinkDiscoverySysInfo},
    },
    log::{info, trace, warn},
    std::{net::Ipv4Addr, sync::Arc},
    tokio::{
        net::UdpSocket,
        time::{timeout, Duration},
    },
};

/// Discovers TP-Link devices on the local network using UDP broadcast (legacy API)
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

/// Discovers TP-Link devices and returns Device objects with embedded client
pub async fn discover_devices_with_client(client: Arc<TpLinkClient>) -> Result<Vec<Device>> {
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
                                devices.push(Device::SmartDimmer(SmartDimmer::new(
                                    *get_sysinfo,
                                    client.clone(),
                                )));
                            } else {
                                devices.push(Device::SmartPlug(SmartPlug::new(
                                    *get_sysinfo,
                                    client.clone(),
                                )));
                            }
                        }
                        GetSysInfo::TPLinkSmartLightData(mut get_sysinfo) => {
                            info!("Smart Light from {}: {}", src_addr, get_sysinfo.alias);
                            get_sysinfo.ip = Some(src_addr.ip());
                            devices.push(Device::SmartLight(SmartLight::new(
                                get_sysinfo,
                                client.clone(),
                            )));
                        }
                        GetSysInfo::TPLinkSmartPowerStripData(mut get_sysinfo) => {
                            info!("Smart Power Strip from {}: {}", src_addr, get_sysinfo.alias);
                            get_sysinfo.ip = Some(src_addr.ip());
                            devices.push(Device::PowerStrip(PowerStrip::new(
                                *get_sysinfo,
                                client.clone(),
                            )));
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

/// Discovers TP-Link devices and returns Device objects (convenience function)
pub async fn discover() -> Result<Vec<Device>> {
    let client = Arc::new(TpLinkClient::new());
    discover_devices_with_client(client).await
}
