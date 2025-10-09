use {
    crate::{
        error::{Result, TpLinkError},
        protocol::encrypt_with_header,
    },
    serde_json::Value,
    std::net::IpAddr,
    tokio::{
        io::{AsyncReadExt, AsyncWriteExt},
        net::TcpStream,
    },
};

/// Core TP-Link client for device communication
#[derive(Debug)]
pub struct TpLinkClient;

impl TpLinkClient {
    /// Creates a new TP-Link client
    pub fn new() -> Self {
        Self
    }

    /// Sends a JSON command to a TP-Link device and returns the response
    pub async fn send(&self, ip: &str, json: Value) -> Result<Value> {
        let ip_addr: IpAddr = ip
            .parse()
            .map_err(|_| TpLinkError::InvalidIpAddress(ip.to_string()))?;

        let mut stream = TcpStream::connect((ip_addr, 9999)).await?;

        let msg_bytes = serde_json::to_vec(&json)?;
        let discover_msg = encrypt_with_header(&msg_bytes);

        stream.write_all(&discover_msg).await?;

        let mut buf = [0u8; 1024];
        let bytes_read = stream.read(&mut buf).await?;

        if bytes_read == 0 {
            return Err(TpLinkError::DeviceCommunication {
                message: "No response from device".to_string(),
            });
        }

        let decrypted_msg = crate::protocol::decrypt_with_header(&buf);
        let response = serde_json::from_slice::<Value>(&decrypted_msg)?;

        log::trace!("Device response: {:#}", response);
        Ok(response)
    }

    // Device-specific methods

    /// Turn on a smart plug
    pub async fn turn_plug_on(&self, ip: &str) -> crate::error::Result<()> {
        self.send(ip, serde_json::json!({"system":{"set_relay_state":{"state": 1}}}))
            .await?;
        Ok(())
    }

    /// Turn off a smart plug
    pub async fn turn_plug_off(&self, ip: &str) -> crate::error::Result<()> {
        self.send(ip, serde_json::json!({"system":{"set_relay_state":{"state": 0}}}))
            .await?;
        Ok(())
    }

    /// Set smart plug alias
    pub async fn set_plug_alias(&self, ip: &str, alias: &str) -> crate::error::Result<()> {
        self.send(ip, serde_json::json!({"system":{"set_dev_alias":{"alias": alias}}}))
            .await?;
        Ok(())
    }

    /// Reboot smart plug
    pub async fn reboot_plug(&self, ip: &str) -> crate::error::Result<()> {
        self.send(ip, serde_json::json!({"system":{"reboot":{"delay": 1}}}))
            .await?;
        Ok(())
    }

    /// Turn light on/off
    pub async fn turn_light_on_off(&self, ip: &str, on: bool) -> crate::error::Result<()> {
        let state = if on { 1 } else { 0 };
        self.send(
            ip,
            serde_json::json!({"smartlife.iot.smartbulb.lightingservice":{"transition_light_state":{"on_off":state,"transition_period":0}}}),
        )
        .await?;
        Ok(())
    }

    /// Set light brightness
    pub async fn set_light_brightness(&self, ip: &str, brightness: u8) -> crate::error::Result<()> {
        if brightness > 100 {
            return Err(crate::error::TpLinkError::DeviceCommunication {
                message: "Brightness must be between 0 and 100".to_string(),
            });
        }
        
        self.send(
            ip, 
            serde_json::json!({"smartlife.iot.smartbulb.lightingservice":{"transition_light_state":{"brightness":brightness,"transition_period":0}}}),
        )
        .await?;
        Ok(())
    }

    /// Set light color
    pub async fn set_light_color(&self, ip: &str, color: &str) -> crate::error::Result<()> {
        match csscolorparser::parse(color) {
            Ok(color) => {
                let [h, s, v, _a] = color.to_hsva();
                let hue = h as u8;
                let saturation = (s * 100.) as u8;
                let value = (v * 100.) as u8;
                let color_temp = 0u8;
                
                self.send(
                    ip,
                    serde_json::json!({
                        "smartlife.iot.smartbulb.lightingservice":{
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
                .await?;
                Ok(())
            }
            Err(e) => Err(crate::error::TpLinkError::InvalidColor(format!(
                "Failed to parse color '{color}': {e}"
            ))),
        }
    }

    /// Set dimmer brightness
    pub async fn set_dimmer_brightness(&self, ip: &str, brightness: u8) -> crate::error::Result<()> {
        if brightness > 100 {
            return Err(crate::error::TpLinkError::DeviceCommunication {
                message: "Brightness must be between 0 and 100".to_string(),
            });
        }

        self.send(
            ip,
            serde_json::json!({"smartlife.iot.dimmer":{"set_dimmer_transition":{"brightness": brightness, "duration": 1}}}),
        )
        .await?;
        Ok(())
    }

    /// Set dimmer inactivity timeout
    pub async fn set_dimmer_inactivity_timeout(&self, ip: &str, timeout_minutes: u8) -> crate::error::Result<()> {
        self.send(
            ip,
            serde_json::json!({"smartlife.iot.dimmer":{"set_cold_time": {"cold_time": timeout_minutes}}}),
        )
        .await?;
        Ok(())
    }

    /// Turn power strip socket on
    pub async fn turn_power_strip_socket_on(&self, ip: &str, socket_id: &str) -> crate::error::Result<()> {
        self.send(
            ip,
            serde_json::json!({
                "context": {
                    "child_ids": [socket_id]
                },
                "system":{"set_relay_state":{"state": 1}}
            }),
        )
        .await?;
        Ok(())
    }

    /// Turn power strip socket off
    pub async fn turn_power_strip_socket_off(&self, ip: &str, socket_id: &str) -> crate::error::Result<()> {
        self.send(
            ip,
            serde_json::json!({
                "context": {
                    "child_ids": [socket_id]
                },
                "system":{"set_relay_state":{"state": 0}}
            }),
        )
        .await?;
        Ok(())
    }

    /// Get power strip energy usage
    pub async fn get_power_strip_energy_usage(&self, ip: &str, socket_id: &str) -> crate::error::Result<serde_json::Value> {
        let response = self.send(
            ip,
            serde_json::json!({
                "context": {
                    "child_ids": [socket_id]
                },
                "system":{"get_energy_usage":{}}
            }),
        )
        .await?;
        Ok(response)
    }

    /// Get power strip emeter data
    pub async fn get_power_strip_emeter_data(&self, ip: &str, socket_id: &str) -> crate::error::Result<serde_json::Value> {
        let response = self.send(
            ip,
            serde_json::json!({
                "context": {
                    "child_ids": [socket_id]
                },
                "system":{"get_emeter_data":{}}
            }),
        )
        .await?;
        Ok(response)
    }
}

impl Default for TpLinkClient {
    fn default() -> Self {
        Self::new()
    }
}
