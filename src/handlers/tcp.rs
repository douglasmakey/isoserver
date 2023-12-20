use std::io;

use log::{error, info};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

pub struct TcpServer {
    address: String,
}

impl TcpServer {
    pub fn new(address: String) -> Self {
        Self { address }
    }

    pub async fn echo(self) -> Result<(), io::Error> {
        let listener = TcpListener::bind(&self.address).await?;
        info!("TCP echo server listening on: {}", self.address);

        loop {
            info!("waiting for new client connection");
            let (mut socket, _) = match listener.accept().await {
                Ok(s) => s,
                Err(e) => {
                    error!("failed to accept socket; error = {}", e);
                    continue;
                }
            };

            info!("new client connection");
            tokio::spawn(async move {
                let mut buf = vec![0; 1024];

                // In a loop, read data from the socket and write the data back.
                loop {
                    let n = match socket.read(&mut buf).await {
                        Ok(n) => {
                            if n == 0 {
                                info!("Client disconnected");
                                return;
                            }

                            info!("Read {} bytes from the socket", n);
                            n
                        }
                        Err(e) => {
                            error!("Failed to read data from socket: {}", e);
                            return;
                        }
                    };

                    if n == 0 {
                        return;
                    }

                    if let Err(e) = socket.write_all(&buf[0..n]).await {
                        error!("Failed to write data to socket: {}", e);
                        return;
                    }
                    info!("Wrote {} bytes to the socket", n);
                }
            });
        }
    }
}
