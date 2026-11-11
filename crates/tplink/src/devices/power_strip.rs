use {
    crate::{
        client::{TpLinkClient, PORT},
        error::{Result, TpLinkError},
    },
    serde_json::{json, Value},
    std::net::{IpAddr, SocketAddr},
};

/// One outlet of a power strip, as the strip reports it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Outlet {
    /// What the strip calls it — the name set in the Kasa app.
    pub alias: String,
    /// The id commands to this outlet name in `context.child_ids`.
    pub id: String,
    pub on: bool,
}

/// A power strip and its outlets, in the order the strip lists them (which is
/// the order they are on the strip).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StripInfo {
    pub alias: String,
    pub model: String,
    pub outlets: Vec<Outlet>,
}

/// A live meter reading.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PowerReading {
    pub watts: f64,
    pub volts: f64,
    pub amps: f64,
    /// Energy used since the meter was last reset, in watt-hours.
    pub total_wh: f64,
}

fn error(message: impl Into<String>) -> TpLinkError {
    TpLinkError::DeviceCommunication {
        message: message.into(),
    }
}

/// A device's `err_code`, refused when it is not zero.
fn check(section: &Value) -> Result<()> {
    match section.get("err_code").and_then(Value::as_i64) {
        Some(0) | None => Ok(()),
        Some(code) => Err(error(format!(
            "the device refused (error {code}{})",
            section
                .get("err_msg")
                .and_then(Value::as_str)
                .map(|msg| format!(": {msg}"))
                .unwrap_or_default()
        ))),
    }
}

/// Read a strip's `get_sysinfo` reply.
pub fn parse_strip_info(answer: &Value) -> Result<StripInfo> {
    let info = answer
        .pointer("/system/get_sysinfo")
        .ok_or_else(|| error("the reply has no system information"))?;
    check(info)?;
    let children = info
        .get("children")
        .and_then(Value::as_array)
        .ok_or_else(|| error("this device has no outlets: it is not a power strip"))?;
    let device_id = info.get("deviceId").and_then(Value::as_str).unwrap_or("");
    let outlets = children
        .iter()
        .map(|child| {
            let id = child
                .get("id")
                .and_then(Value::as_str)
                .ok_or_else(|| error("an outlet has no id"))?;
            Ok(Outlet {
                alias: child
                    .get("alias")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_string(),
                // Some firmware gives only the two-digit suffix; commands
                // need the whole id.
                id: if id.len() <= 2 {
                    format!("{device_id}{id}")
                } else {
                    id.to_string()
                },
                on: child.get("state").and_then(Value::as_i64) == Some(1),
            })
        })
        .collect::<Result<Vec<_>>>()?;
    Ok(StripInfo {
        alias: info
            .get("alias")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string(),
        model: info
            .get("model")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string(),
        outlets,
    })
}

/// Read an `emeter.get_realtime` reply. Older firmware answers in watts and
/// volts, newer in milliwatts and millivolts; both are read.
pub fn parse_power_reading(answer: &Value) -> Result<PowerReading> {
    let meter = answer
        .pointer("/emeter/get_realtime")
        .ok_or_else(|| error("this device has no power meter"))?;
    check(meter)?;
    let value = |plain: &str, milli: &str| -> Option<f64> {
        meter
            .get(plain)
            .and_then(Value::as_f64)
            .or_else(|| meter.get(milli).and_then(Value::as_f64).map(|v| v / 1000.0))
    };
    Ok(PowerReading {
        watts: value("power", "power_mw").ok_or_else(|| error("the reading has no power"))?,
        volts: value("voltage", "voltage_mv").unwrap_or(0.0),
        amps: value("current", "current_ma").unwrap_or(0.0),
        total_wh: meter
            .get("total_wh")
            .and_then(Value::as_f64)
            .or_else(|| {
                meter
                    .get("total")
                    .and_then(Value::as_f64)
                    .map(|kwh| kwh * 1000.0)
            })
            .unwrap_or(0.0),
    })
}

/// Energy one outlet used on one day, by the strip's own counter.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DayEnergy {
    pub year: i32,
    pub month: u32,
    pub day: u32,
    pub wh: f64,
}

/// Read an `emeter.get_daystat` reply: one entry per day of the month the
/// strip has counted. Newer firmware answers in watt-hours (`energy_wh`),
/// older in kilowatt-hours (`energy`); both are read.
pub fn parse_daystat(answer: &Value) -> Result<Vec<DayEnergy>> {
    let stat = answer
        .pointer("/emeter/get_daystat")
        .ok_or_else(|| error("this device has no power meter"))?;
    check(stat)?;
    let days = stat
        .get("day_list")
        .and_then(Value::as_array)
        .ok_or_else(|| error("the reply has no list of days"))?;
    days.iter()
        .map(|entry| {
            let number = |key: &str| entry.get(key).and_then(Value::as_i64);
            Ok(DayEnergy {
                year: number("year").ok_or_else(|| error("a day has no year"))? as i32,
                month: number("month").ok_or_else(|| error("a day has no month"))? as u32,
                day: number("day").ok_or_else(|| error("a day has no day"))? as u32,
                wh: entry
                    .get("energy_wh")
                    .and_then(Value::as_f64)
                    .or_else(|| {
                        entry
                            .get("energy")
                            .and_then(Value::as_f64)
                            .map(|kwh| kwh * 1000.0)
                    })
                    .ok_or_else(|| error("a day has no energy"))?,
            })
        })
        .collect()
}

fn address(ip: &str) -> Result<SocketAddr> {
    let ip: IpAddr = ip
        .parse()
        .map_err(|_| TpLinkError::InvalidIpAddress(ip.to_string()))?;
    Ok(SocketAddr::new(ip, PORT))
}

/// Parsed power strip reads.
impl TpLinkClient {
    /// The strip's name and its outlets, in order.
    pub async fn strip_info(&self, ip: &str) -> Result<StripInfo> {
        self.strip_info_at(address(ip)?).await
    }

    pub async fn strip_info_at(&self, addr: SocketAddr) -> Result<StripInfo> {
        parse_strip_info(
            &self
                .send_to(addr, json!({"system": {"get_sysinfo": {}}}))
                .await?,
        )
    }

    /// One outlet's live power reading.
    pub async fn outlet_power(&self, ip: &str, outlet_id: &str) -> Result<PowerReading> {
        self.outlet_power_at(address(ip)?, outlet_id).await
    }

    pub async fn outlet_power_at(&self, addr: SocketAddr, outlet_id: &str) -> Result<PowerReading> {
        parse_power_reading(
            &self
                .send_to(
                    addr,
                    json!({
                        "context": {"child_ids": [outlet_id]},
                        "emeter": {"get_realtime": {}}
                    }),
                )
                .await?,
        )
    }

    /// One outlet's energy per day for a month, by the strip's own counters.
    pub async fn outlet_daily_energy_at(
        &self,
        addr: SocketAddr,
        outlet_id: &str,
        year: i32,
        month: u32,
    ) -> Result<Vec<DayEnergy>> {
        parse_daystat(
            &self
                .send_to(
                    addr,
                    json!({
                        "context": {"child_ids": [outlet_id]},
                        "emeter": {"get_daystat": {"year": year, "month": month}}
                    }),
                )
                .await?,
        )
    }

    /// Switch one outlet, and confirm the strip did it.
    pub async fn set_outlet_at(&self, addr: SocketAddr, outlet_id: &str, on: bool) -> Result<()> {
        let answer = self
            .send_to(
                addr,
                json!({
                    "context": {"child_ids": [outlet_id]},
                    "system": {"set_relay_state": {"state": u8::from(on)}}
                }),
            )
            .await?;
        check(
            answer
                .pointer("/system/set_relay_state")
                .ok_or_else(|| error("the strip did not say whether it switched"))?,
        )
    }
}

/// Power strip operations
impl TpLinkClient {
    /// Turns off a specific socket on a smart power strip
    pub async fn turn_power_strip_socket_off(&self, ip: &str, socket_id: &str) -> Result<()> {
        self.send(
            ip,
            json!({
                "context": {
                    "child_ids": [socket_id]
                },
                "system":{"set_relay_state":{"state": 0}}
            }),
        )
        .await?;
        Ok(())
    }

    /// Turns on a specific socket on a smart power strip
    pub async fn turn_power_strip_socket_on(&self, ip: &str, socket_id: &str) -> Result<()> {
        self.send(
            ip,
            json!({
                "context": {
                    "child_ids": [socket_id]
                },
                "system":{"set_relay_state":{"state": 1}}
            }),
        )
        .await?;
        Ok(())
    }

    /// Gets this month's energy use per day for a specific socket on a smart
    /// power strip (the device's `emeter.get_daystat`).
    pub async fn get_power_strip_energy_usage(&self, ip: &str, socket_id: &str) -> Result<Value> {
        let response = self
            .send(
                ip,
                json!({
                    "context": {
                        "child_ids": [socket_id]
                    },
                    "emeter":{"get_daystat":{}}
                }),
            )
            .await?;
        Ok(response)
    }

    /// Gets the live meter reading for a specific socket on a smart power strip
    /// (the device's `emeter.get_realtime`). [`TpLinkClient::outlet_power`] is
    /// the same reading, parsed.
    pub async fn get_power_strip_emeter_data(&self, ip: &str, socket_id: &str) -> Result<Value> {
        let response = self
            .send(
                ip,
                json!({
                    "context": {
                        "child_ids": [socket_id]
                    },
                    "emeter":{"get_realtime":{}}
                }),
            )
            .await?;
        Ok(response)
    }
}

// Standalone functions for backwards compatibility
pub async fn turn_power_strip_socket_off(ip: &str, socket_id: &str) -> Result<()> {
    TpLinkClient::new()
        .turn_power_strip_socket_off(ip, socket_id)
        .await
}

pub async fn turn_power_strip_socket_on(ip: &str, socket_id: &str) -> Result<()> {
    TpLinkClient::new()
        .turn_power_strip_socket_on(ip, socket_id)
        .await
}

pub async fn get_power_strip_energy_usage(ip: &str, socket_id: &str) -> Result<Value> {
    TpLinkClient::new()
        .get_power_strip_energy_usage(ip, socket_id)
        .await
}

pub async fn get_power_strip_emeter_data(ip: &str, socket_id: &str) -> Result<Value> {
    TpLinkClient::new()
        .get_power_strip_emeter_data(ip, socket_id)
        .await
}

#[cfg(test)]
mod tests {
    use {super::*, crate::client::fake};

    /// An HS300's `get_sysinfo`, trimmed, with real-looking ids.
    fn hs300() -> Value {
        json!({"system": {"get_sysinfo": {
            "alias": "Server rack", "model": "HS300(US)", "err_code": 0,
            "deviceId": "8006ABCDEF",
            "children": [
                {"id": "8006ABCDEF00", "alias": "lemon", "state": 1},
                {"id": "01", "alias": "switch", "state": 0}
            ]
        }}})
    }

    #[tokio::test]
    async fn a_strip_lists_its_outlets_in_order_with_whole_ids() {
        let (addr, _) = fake::device(|_| hs300()).await;
        let info = TpLinkClient::new().strip_info_at(addr).await.unwrap();
        assert_eq!(info.alias, "Server rack");
        assert_eq!(info.model, "HS300(US)");
        assert_eq!(
            info.outlets,
            vec![
                Outlet {
                    alias: "lemon".into(),
                    id: "8006ABCDEF00".into(),
                    on: true
                },
                Outlet {
                    alias: "switch".into(),
                    id: "8006ABCDEF01".into(),
                    on: false
                },
            ]
        );
    }

    #[tokio::test]
    async fn a_plug_is_not_a_strip() {
        let reply = json!({"system": {"get_sysinfo": {"alias": "lamp", "relay_state": 1}}});
        let (addr, _) = fake::device(move |_| reply.clone()).await;
        assert!(TpLinkClient::new().strip_info_at(addr).await.is_err());
    }

    #[tokio::test]
    async fn an_outlet_reading_asks_that_outlet_and_reads_milliwatts() {
        let (addr, seen) = fake::device(|_| {
            json!({"emeter": {"get_realtime": {
                "err_code": 0, "power_mw": 61250, "voltage_mv": 121400,
                "current_ma": 540, "total_wh": 1830
            }}})
        })
        .await;
        let reading = TpLinkClient::new()
            .outlet_power_at(addr, "8006ABCDEF00")
            .await
            .unwrap();
        assert_eq!(reading.watts, 61.25);
        assert_eq!(reading.volts, 121.4);
        assert_eq!(reading.amps, 0.54);
        assert_eq!(reading.total_wh, 1830.0);
        assert_eq!(
            seen.lock().unwrap()[0],
            json!({"context": {"child_ids": ["8006ABCDEF00"]}, "emeter": {"get_realtime": {}}})
        );
    }

    #[test]
    fn older_firmware_answers_in_watts() {
        let reading = parse_power_reading(&json!({"emeter": {"get_realtime": {
            "power": 12.5, "voltage": 120.1, "current": 0.1, "total": 2.5
        }}}))
        .unwrap();
        assert_eq!(reading.watts, 12.5);
        assert_eq!(reading.total_wh, 2500.0);
    }

    #[test]
    fn a_refusal_is_an_error_not_a_zero() {
        assert!(parse_power_reading(&json!({"emeter": {"get_realtime": {
            "err_code": -1, "err_msg": "module not support"
        }}}))
        .is_err());
    }

    #[tokio::test]
    async fn a_month_of_days_is_asked_of_one_outlet_and_read_in_watt_hours() {
        let (addr, seen) = fake::device(|_| {
            json!({"emeter": {"get_daystat": {"err_code": 0, "day_list": [
                {"year": 2026, "month": 11, "day": 1, "energy_wh": 3400},
                {"year": 2026, "month": 11, "day": 2, "energy_wh": 3512}
            ]}}})
        })
        .await;
        let days = TpLinkClient::new()
            .outlet_daily_energy_at(addr, "8006ABCDEF05", 2026, 11)
            .await
            .unwrap();
        assert_eq!(days.len(), 2);
        assert_eq!(
            days[1],
            DayEnergy {
                year: 2026,
                month: 11,
                day: 2,
                wh: 3512.0
            }
        );
        assert_eq!(
            seen.lock().unwrap()[0],
            json!({"context": {"child_ids": ["8006ABCDEF05"]},
                   "emeter": {"get_daystat": {"year": 2026, "month": 11}}})
        );
        let older = parse_daystat(&json!({"emeter": {"get_daystat": {"day_list": [
            {"year": 2026, "month": 11, "day": 1, "energy": 3.4}
        ]}}}))
        .unwrap();
        assert_eq!(older[0].wh, 3400.0);
        assert!(parse_daystat(&json!({"emeter": {"get_daystat": {"err_code": -1}}})).is_err());
    }

    #[tokio::test]
    async fn switching_an_outlet_names_it_and_checks_the_answer() {
        let (addr, seen) =
            fake::device(|_| json!({"system": {"set_relay_state": {"err_code": 0}}})).await;
        TpLinkClient::new()
            .set_outlet_at(addr, "8006ABCDEF01", true)
            .await
            .unwrap();
        assert_eq!(
            seen.lock().unwrap()[0],
            json!({"context": {"child_ids": ["8006ABCDEF01"]}, "system": {"set_relay_state": {"state": 1}}})
        );
        let (refusing, _) =
            fake::device(|_| json!({"system": {"set_relay_state": {"err_code": -3}}})).await;
        assert!(TpLinkClient::new()
            .set_outlet_at(refusing, "x", true)
            .await
            .is_err());
    }
}
