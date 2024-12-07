use {
    super::types::{DeviceData, GetSysInfo, TPLinkDiscoveryRes, TPLinkDiscoverySysInfo},
    log::{info, trace, warn},
    serde_json::{json, Value},
    std::{
        error::Error,
        io,
        net::{IpAddr, Ipv4Addr},
    },
    tokio::{
        io::{AsyncReadExt, AsyncWriteExt},
        net::{TcpStream, UdpSocket},
        time::{timeout, Duration},
    },
};

const KEY: u8 = 0xAB;

pub async fn discover_devices() -> Result<Vec<DeviceData>, Box<dyn Error + Send>> {
    let port = 9999;
    let socket = UdpSocket::bind((Ipv4Addr::UNSPECIFIED, port))
        .await
        .unwrap();
    socket.set_broadcast(true).unwrap();
    let request = TPLinkDiscoveryRes {
        system: TPLinkDiscoverySysInfo {
            get_sysinfo: GetSysInfo::Empty(()),
        },
    };
    let msg_bytes =
        serde_json::to_vec(&request).expect("Should be able to serialize hardcoded data w/o error");
    let discover_msg = encrypt(&msg_bytes, KEY);

    let broadcast_addr = (Ipv4Addr::BROADCAST, port);
    socket.send_to(&discover_msg, broadcast_addr).await.unwrap();

    let mut buf = [0; 2048];
    let timeout_duration = Duration::from_millis(2500);

    let mut devices = Vec::with_capacity(20);
    loop {
        match timeout(timeout_duration, socket.recv_from(&mut buf)).await {
            Ok(Ok((num_bytes, src_addr))) => {
                let incoming_data = decrypt(&buf, KEY);
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
                            std::str::from_utf8(valid_data).expect("Failed to convert to UTF-8");

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

pub async fn send(ip: &str, json: serde_json::Value) -> io::Result<()> {
    let _ip: IpAddr = match ip.parse() {
        Ok(addr) => addr,
        Err(e) => {
            eprintln!("Failed to parse IP address '{}': {}", ip, e);
            return Err(io::Error::new(io::ErrorKind::InvalidInput, e));
        }
    };

    let mut stream = TcpStream::connect((_ip, 9999)).await?;

    let msg_bytes =
        serde_json::to_vec(&json).expect("Should be able to serialize hardcoded data w/o error");
    let discover_msg = encrypt_with_header(&msg_bytes, KEY);

    stream.write_all(&discover_msg).await.unwrap();

    let mut buf = [0u8; 1024];
    let bytes_read = stream.read(&mut buf).await?;
    println!("bytes_read: {bytes_read}");
    let decrypted_msg = decrypt_with_header(&buf, KEY);
    let msg = serde_json::from_slice::<Value>(&decrypted_msg).unwrap();
    println!("msg: {msg:?}");
    Ok(())
}

pub async fn tplink_set_alias(ip: &str, alias: &str) {
    send(ip, json!({"system":{"set_dev_alias":{"alias": alias}}}))
        .await
        .unwrap();
}

pub async fn tplink_reboot(ip: &str) {
    send(ip, json!({"system":{"reboot":{"delay": 1}}}))
        .await
        .unwrap();
}

pub async fn tplink_turn_plug_on(ip: &str) {
    send(ip, json!({"system":{"set_relay_state":{"state": 1}}}))
        .await
        .unwrap();
}

pub async fn tplink_turn_plug_off(ip: &str) {
    send(ip, json!({"system":{"set_relay_state":{"state": 0}}}))
        .await
        .unwrap();
}

pub async fn tplink_turn_smart_strip_socket_off(ip: &str, id: &str) {
    send(
        ip,
        json!(
        {
            "context": {
                "child_ids": [id]
            },
            "system":{"set_relay_state":{"state": 0}}
        }),
    )
    .await
    .unwrap();
}

pub async fn tplink_turn_smart_strip_socket_on(ip: &str, id: &str) {
    send(
        ip,
        json!(
        {
            "context": {
                "child_ids": [id]
            },
            "system":{"set_relay_state":{"state": 1}}
        }),
    )
    .await
    .unwrap();
}

pub async fn tplink_set_dimmer_brightness(ip: &str, brightness: &u8) {
    send(
        ip,
        json!({"smartlife.iot.dimmer":{"set_dimmer_transition":{"brightness": brightness, "duration": 1}}}),
    )
    .await
    .unwrap();
}

pub async fn tplink_set_dimmer_inactivity_timeout(ip: &str, timeout: &u8) {
    send(
        ip,
        json!({"smartlife.iot.dimmer":{"set_cold_time": {"cold_time": timeout}}}),
    )
    .await
    .unwrap();
}

// https://github.com/python-kasa/python-kasa/blob/123ea107b1e7536bc5dfc8b93111cc5c7e8d066b/tests/fakeprotocol_iot.py#L445
const LIGHT_SERVICE: &str = "smartlife.iot.smartbulb.lightingservice";

pub async fn tplink_turn_light_on_off(ip: &str, state: u8) {
    send(
        ip,
        json!({LIGHT_SERVICE:{"transition_light_state":{"on_off":state,"transition_period":0}}}),
    )
    .await
    .unwrap();
}

pub async fn tplink_set_light_brightness(ip: &str, brightness: u8) {
    send(ip, json!({LIGHT_SERVICE:{"transition_light_state":{"brightness":brightness,"transition_period":0}}}))
        .await
        .unwrap();
}

pub async fn tplink_set_light_hsl(ip: &str, color: String) {
    match csscolorparser::parse(&color) {
        Ok(color) => {
            let [h, s, v, _a] = color.to_hsva();
            let hue = h as u8;
            let saturation = (s * 100.) as u8;
            let value = (v * 100.) as u8;
            let color_temp = 0u8;
            // https://github.com/python-kasa/python-kasa/blob/123ea107b1e7536bc5dfc8b93111cc5c7e8d066b/kasa/iot/iotbulb.py#L407
            send(
                ip,
                json!({
                        LIGHT_SERVICE:{
                        "transition_light_state":{
                            "hue": hue,
                            "saturation": saturation,
                            "brightness": value,
                            "color_temp": color_temp,
                            "transition_period": 0
                        }
                    }
                }),
            )
            .await
            .unwrap();
        }
        Err(e) => {
            leptos::logging::log!("Failed to parse color '{}': {}", color, e);
        }
    }
}

fn encrypt_with_header(input: &[u8], first_key: u8) -> Vec<u8> {
    (input.len() as u32)
        .to_be_bytes()
        .into_iter()
        .chain(input.iter().scan(first_key, |key, byte| {
            let result = *byte ^ *key;
            *key = result;
            Some(result)
        }))
        .collect()
}

fn decrypt(input: &[u8], first_key: u8) -> Vec<u8> {
    let mut buf = input.to_vec();
    let mut key = first_key;
    for item in &mut buf {
        let next_key = *item;
        *item ^= key;
        key = next_key;
    }
    buf
}

fn encrypt(input: &[u8], first_key: u8) -> Vec<u8> {
    let mut buf = input.to_vec();
    let mut key = first_key;
    for byte in &mut buf {
        *byte ^= key;
        key = *byte;
    }
    buf
}

fn decrypt_with_header(input: &[u8], first_key: u8) -> Vec<u8> {
    let len = u32::from_be_bytes(input[0..4].try_into().unwrap());
    let mut msg = decrypt(&input[4..], first_key);
    msg.truncate(len as usize);
    msg
}
