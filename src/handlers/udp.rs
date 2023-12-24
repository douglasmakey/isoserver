use log::info;
use std::io;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::UdpSocket;
use tokio::spawn;
use tokio::sync::mpsc;

pub struct UdpServer {
    address: String,
}

impl UdpServer {
    pub fn new(address: String) -> Self {
        Self { address }
    }

    pub async fn run(self) -> Result<(), io::Error> {
        let socket = UdpSocket::bind(&self.address).await?;
        info!("UDP echo server listening on: {}", socket.local_addr()?);
        let r = Arc::new(socket);
        let s = r.clone();
        let (tx, mut rx) = mpsc::channel::<(Vec<u8>, SocketAddr)>(1_000);

        spawn(async move {
            while let Some((bytes, addr)) = rx.recv().await {
                let len = s.send_to(&bytes, &addr).await.unwrap();
                info!("{:?} bytes sent", len);
            }
        });

        let mut buf = [0; 1024];
        loop {
            let (len, addr) = r.recv_from(&mut buf).await?;
            info!("{:?} bytes received from {:?}", len, addr);
            tx.send((buf[..len].to_vec(), addr)).await.unwrap();
        }
    }
}
