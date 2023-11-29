use {
    serde_json::{json, Value},
    std::{error::Error, io, net::IpAddr},
    tokio::{
        io::{AsyncReadExt, AsyncWriteExt},
        net::{TcpStream, UdpSocket},
        time::{timeout, Duration},
    },
};

const KEY: u8 = 0xAB;

pub async fn discover_devices() -> Result<(), Box<dyn Error>> {
    let socket = UdpSocket::bind("0.0.0.0:9999").await.unwrap();
    socket.set_broadcast(true).unwrap();
    let msg_bytes = serde_json::to_vec(&json!({"system":{"get_sysinfo":{}}}))
        .expect("Should be able to serialize hardcoded data w/o error");
    let discover_msg = encrypt(&msg_bytes, KEY);

    let broadcast_addr = "255.255.255.255:9999";
    socket.send_to(&discover_msg, broadcast_addr).await.unwrap();

    let mut buf = [0; 1024];
    let timeout_duration = Duration::from_secs(5);

    loop {
        match timeout(timeout_duration, socket.recv_from(&mut buf)).await {
            Ok(Ok((num_bytes, src_addr))) => {
                let incoming_data = decrypt(&buf, KEY);
                let msg = serde_json::from_slice::<Value>(&incoming_data[..num_bytes]).unwrap();
                println!("Received from {}: {}", src_addr, msg);
            }
            Ok(Err(e)) => {
                eprintln!("Error receiving message: {}", e);
                break;
            }
            Err(_) => {
                println!("Timeout reached, no more responses.");
                break;
            }
        }
    }

    Ok(())
}

pub async fn send() -> io::Result<()> {
    let ip: IpAddr = "10.0.0.197".parse().unwrap();
    let mut stream = TcpStream::connect((ip, 9999)).await?;

    let msg_bytes = serde_json::to_vec(&json!({"system":{"set_relay_state":{"state":1}}}))
        .expect("Should be able to serialize hardcoded data w/o error");
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
    for i in 0..buf.len() {
        let next_key = buf[i];
        buf[i] ^= key;
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
