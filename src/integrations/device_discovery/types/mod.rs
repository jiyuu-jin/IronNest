use serde::{Deserialize, Serialize};
use serde_yaml::{self};

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct NetworkInfo {
    pub ipv4: String,
    pub port: u16,
    pub mac: String,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct NetworkService {
    pub service: String,
    pub service_name: Option<String>,
    pub product_name: Option<String>,
    pub port: Option<u16>,
    //pub backend: Option<Vec<String>>,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct Device {
    pub uuid: Option<String>,
    pub name: Option<String>,
    pub serial: Option<String>,
    pub location: Option<String>,

    pub network: NetworkInfo,
    pub services: NetworkService,

    pub local_key: Option<String>,
    pub uid: Option<String>,
    pub product_name: Option<String>,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct Devices {
    pub uuid: String,
    pub name: String,
    pub location: String,
    pub list: Vec<Device>,
}

impl Devices {
    pub fn add(&mut self, device: Device) {
        self.list.push(device);
    }

    pub fn load_devices(&mut self) {
        let f = std::fs::File::open("devices.yml").expect("Could not open file.");
        let all_devices: Devices = serde_yaml::from_reader(f).expect("Could not read values.");
        println!("{:?}", all_devices);

        println!(
            "[Debug] all devices: {:?}",
            all_devices.list
        );

        for device in all_devices.list.iter() {
            println!("{:?}", device.name);
        }
    }

    pub fn dump_devices(&mut self) {
        let f = std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .open("devices.yml")
            .expect("Couldn't open file");
        serde_yaml::to_writer(f, &self.list).unwrap();
    }
}
