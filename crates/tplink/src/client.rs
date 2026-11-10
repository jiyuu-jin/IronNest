use {
    crate::{
        error::{Result, TpLinkError},
        protocol::{decrypt, encrypt_with_header},
    },
    serde_json::Value,
    std::net::{IpAddr, SocketAddr},
    tokio::{
        io::{AsyncReadExt, AsyncWriteExt},
        net::TcpStream,
    },
};

/// The port every Kasa device answers its local protocol on.
pub const PORT: u16 = 9999;

/// The largest reply accepted. A power strip's `get_sysinfo` with six outlets
/// is about 2 KiB; anything near this is not a Kasa device.
const MAX_REPLY: usize = 64 * 1024;

/// Core TP-Link client for device communication
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
        self.send_to(SocketAddr::new(ip_addr, PORT), json).await
    }

    /// Sends a JSON command to a device at an explicit address and port.
    ///
    /// The reply is read whole: the protocol prefixes it with its length, and
    /// a reply longer than one read (a power strip's status is) arrives in
    /// several pieces.
    pub async fn send_to(&self, addr: SocketAddr, json: Value) -> Result<Value> {
        let mut stream = TcpStream::connect(addr).await?;
        stream
            .write_all(&encrypt_with_header(&serde_json::to_vec(&json)?))
            .await?;

        let mut header = [0u8; 4];
        stream.read_exact(&mut header).await.map_err(|error| {
            if error.kind() == std::io::ErrorKind::UnexpectedEof {
                TpLinkError::DeviceCommunication {
                    message: "No response from device".to_string(),
                }
            } else {
                error.into()
            }
        })?;
        let len = u32::from_be_bytes(header) as usize;
        if len > MAX_REPLY {
            return Err(TpLinkError::DeviceCommunication {
                message: format!("reply of {len} bytes is too large to be a TP-Link device"),
            });
        }
        let mut body = vec![0u8; len];
        stream.read_exact(&mut body).await?;

        let response = serde_json::from_slice::<Value>(&decrypt(&body))?;
        log::trace!("Device response: {:#}", response);
        Ok(response)
    }
}

impl Default for TpLinkClient {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
pub(crate) mod fake {
    //! A device on a local port that answers every request, and records what
    //! it was sent.

    use {
        crate::protocol::{decrypt, encrypt_with_header},
        serde_json::Value,
        std::{
            net::SocketAddr,
            sync::{Arc, Mutex},
        },
        tokio::{
            io::{AsyncReadExt, AsyncWriteExt},
            net::TcpListener,
        },
    };

    pub async fn device(
        answer: impl Fn(&Value) -> Value + Send + Sync + 'static,
    ) -> (SocketAddr, Arc<Mutex<Vec<Value>>>) {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let seen = Arc::new(Mutex::new(Vec::new()));
        let record = Arc::clone(&seen);
        tokio::spawn(async move {
            loop {
                let Ok((mut stream, _)) = listener.accept().await else {
                    return;
                };
                let mut header = [0u8; 4];
                stream.read_exact(&mut header).await.unwrap();
                let mut body = vec![0u8; u32::from_be_bytes(header) as usize];
                stream.read_exact(&mut body).await.unwrap();
                let request: Value = serde_json::from_slice(&decrypt(&body)).unwrap();
                let reply = encrypt_with_header(&serde_json::to_vec(&answer(&request)).unwrap());
                record.lock().unwrap().push(request);
                // In small pieces, the way a slow device's reply arrives.
                for chunk in reply.chunks(300) {
                    stream.write_all(chunk).await.unwrap();
                    stream.flush().await.unwrap();
                }
            }
        });
        (addr, seen)
    }
}

#[cfg(test)]
mod tests {
    use {super::*, serde_json::json};

    #[tokio::test]
    async fn a_reply_longer_than_one_read_arrives_whole() {
        let long = "x".repeat(5000);
        let reply = json!({"system": {"get_sysinfo": {"alias": long}}});
        let (addr, _) = fake::device(move |_| reply.clone()).await;
        let answer = TpLinkClient::new()
            .send_to(addr, json!({"system": {"get_sysinfo": {}}}))
            .await
            .unwrap();
        assert_eq!(
            answer["system"]["get_sysinfo"]["alias"]
                .as_str()
                .unwrap()
                .len(),
            5000
        );
    }
}
