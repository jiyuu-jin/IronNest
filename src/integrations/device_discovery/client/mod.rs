use mdns_sd::{ServiceDaemon, ServiceEvent};
use std::collections::HashMap;
use uuid::Uuid;
use std::{env, time::Duration};

pub mod types;
pub use types::lib::*;

fn get_service(service_type : &str) -> Devices {
    if env::var_os("lame").is_some() {
        println!("yeet the sumbitch");
    }
    // Create a daemon
    let mdns = ServiceDaemon::new().expect("Failed to create daemon");
    let receiver = mdns.browse(service_type).expect("Failed to browse");

    let mut found_devices = Devices {
        uuid: Uuid::new_v4().to_string().to_owned(),
        name: "Home".to_owned(),
        location: "Earf".to_owned(),
        list: Vec::with_capacity(50)
    };
    
    while let Ok(event) = receiver.recv_timeout(Duration::from_millis(1000)) {
        match event {
            ServiceEvent::ServiceResolved(info) => {
                let addresses = info
                    .get_addresses()
                    .iter()
                    .map(|address| address.to_string())
                    .collect::<Vec<_>>();
                println!("Resolved a new service: {}", info.get_fullname());
                println!("IP: {:?}", addresses);
                println!("[Debug]: {:?}", info);
                let service = NetworkService {
                    service: info.get_type().to_owned(),
                    service_name: Some("".to_owned()),
                    product_name: Some("".to_owned()),
                    port: Some(info.get_port().to_owned()),
                };
                let network_info = NetworkInfo {
                    ipv4: addresses.join(".").to_string().to_owned(),
                    port: info.get_port().to_owned(),
                    mac: "".to_owned(),
                };
                let device = Device {
                    uuid:  Some(info.get_fullname().to_owned()),
                    name:  Some(info.get_fullname().to_owned()),
                    network: network_info,
                    services:  service,
                    serial: Some("".to_owned()),
                    location: Some("".to_owned()),
                    local_key: Some("".to_owned()),
                    uid: Some("".to_owned()),
                    product_name: Some("".to_owned()),
                };
                println!("device: {:?}", device);

                found_devices.add(device);
                println!("found_devices: {:?}", found_devices);
            }
            ServiceEvent::SearchStarted(_) => {},
            ServiceEvent::ServiceFound(_, _) => {},
            ServiceEvent::ServiceRemoved(_, _) => {},
            ServiceEvent::SearchStopped(_) => {},
        }
    }

    // Gracefully shutdown the daemon.
    std::thread::sleep(std::time::Duration::from_secs(1));
    let nul = mdns.shutdown().unwrap();
    return found_devices;
}

fn run_mdns_discovery() -> Devices {
    let devices = HashMap::from([
        ("_airgradient._tcp.local.", "airgradient"),
        ("_androidtvremote2._tcp.local.", "androidtv_remote"),
        ("_appletv-v2._tcp.local.", "apple_tv"),
        ("_mediaremotetv._tcp.local.", "apple_tv"),
        ("_hscp._tcp.local.", "apple_tv"),
        ("_airport._tcp.local.", "apple_tv"),
        ("_companion-link._tcp.local.", "apple_tv"),
        ("_sleep-proxy._udp.local.", "apple_tv"),
        ("_touch-able._tcp.local.", "apple_tv"),
        ("_raop._tcp.local.", "apple_tv"),
        ("_api._tcp.local.", "baf"),
        ("_bangolufsen._tcp.local.", "bang_olufsen"),
        ("_bbxsrv._tcp.local.", "blebox"),
        ("_musc._tcp.local.", "bluesound"),
        ("_bond._tcp.local.", "bond"),
        ("_printer._tcp.local.", "brother"),
        ("_googlecast._tcp.local.", "cast"),
        ("_dkapi._tcp.local.", "daikin"),
        ("_deako._tcp.local.", "deako"),
        ("_devialet-http._tcp.local.", "devialet"),
        ("_dvl-deviceapi._tcp.local.", "devolo_home_network"),
        ("_axis-video._tcp.local.", "doorbird"),
        ("_ecobee._tcp.local.", "ecobee"),
        ("_sideplay._tcp.local.", "ecobee"),
        ("_elg._tcp.local.", "elgato"),
        ("_elmax-ssl._tcp.local.", "elmax"),
        ("_enphase-envoy._tcp.local.", "enphase_envoy"),
        ("_daap._tcp.local.", "forked_daapd"),
        ("_fbx-api._tcp.local.", "freebox"),
        ("_api._udp.local.", "guardian"),
        ("_homekit._tcp.local.", "homekit"),
        ("_hap._udp.local.", "homekit_controller"),
        ("_hwenergy._tcp.local.", "homewizard"),
        ("_hue._tcp.local.", "hue"),
        ("_powerview-g3._tcp.local.", "hunterdouglas_powerview"),
        ("_powerview._tcp.local.", "hunterdouglas_powerview"),
        ("_ipp._tcp.local.", "ipp"),
        ("_ipps._tcp.local.", "ipp"),
        ("_xbmc-jsonrpc-h._tcp.local.", "kodi"),
        ("_http._tcp.local.", "lektrico"),
        ("_linkplay._tcp.local.", "linkplay"),
        ("_lookin._tcp.local.", "lookin"),
        ("_lutron._tcp.local.", "lutron_caseta"),
        ("_matter._tcp.local.", "matter"),
        ("_matterc._udp.local.", "matter"),
        ("_minecraft._tcp.local.", "minecraft"),
        ("_tvm._tcp.local.", "motionmount"),
        ("_nanoleafms._tcp.local.", "nanoleaf"),
        ("_nanoleafapi._tcp.local.", "nanoleaf"),
        ("_nut._tcp.local.", "nut"),
        ("_octoprint._tcp.local.", "octoprint"),
        ("_kizbox._tcp.local.", "overkiz"),
        ("_kizboxdev._tcp.local.", "overkiz"),
        ("_plexmediasvr._tcp.local.", "plex"),
        ("_plugwise._tcp.local.", "plugwise"),
        ("_rabbitair._udp.local.", "rabbitair"),
        ("_aicu-http._tcp.local.", "romy"),
        ("_amzn-alexa._tcp.local.", "roomba"),
        ("_airplay._tcp.local.", "samsungtv"),
        ("_smartview._tcp.local.", "samsungtv"),
        ("_ssh._tcp.local.", "smappee"),
        ("_sonos._tcp.local.", "sonos"),
        ("_soundtouch._tcp.local.", "soundtouch"),
        ("_spotify-connect._tcp.local.", "spotify"),
        ("_system-bridge._tcp.local.", "system_bridge"),
        ("_technove-stations._tcp.local.", "technove"),
        ("_meshcop._udp.local.", "thread"),
        ("_tivo-device._tcp.local.", "tivo"),
        ("_tivo-mindrpc._tcp.local.", "tivo"),
        ("_viziocast._tcp.local.", "vizio"),
        ("_Volumio._tcp.local.", "volumio"),
        ("_wled._tcp.local.", "wled"),
        ("_wyoming._tcp.local.", "wyoming"),
        ("_miio._udp.local.", "yeelight"),
        ("_uzg-01._tcp.local.", "zha"),
        ("_slzb-06._tcp.local.", "zha"),
        ("_czc._tcp.local.", "zha"),
        ("_esphomelib._tcp.local.", "zha"),
        ("_zigate-zigbee-gateway._tcp.local.", "zha"),
        ("_xzg._tcp.local.", "zha"),
        ("_zigstar_gw._tcp.local.", "zha"),
        ("_zwave-js-server._tcp.local.", "zwave_js"),
        ("_hap._tcp.local.", "zwave_me"),
    ]);
    let mut output = Vec::with_capacity(50);

    for (key, value) in devices.clone().into_iter() {
        for device in get_service(key).list {
            output.push(device);
        }
    }

    let found_devices = Devices {
        uuid: Uuid::new_v4().to_string().to_owned(),
        name: "Home".to_owned(),
        location: "Earf".to_owned(),
        list: output
    };
    println!("found_devices: {:?}", found_devices);
    return found_devices;
}
