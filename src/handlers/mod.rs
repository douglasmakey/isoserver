pub mod tcp;
pub mod udp;

pub async fn execute(handler: String, addr: String) -> Result<(), Box<dyn std::error::Error>> {
    match handler.as_str() {
        "tcp-echo" => {
            let s = tcp::TcpServer::new(addr);
            s.echo()
                .await
                .map_err(|e| format!("tcp-echo error: {}", e))?;
        }
        "udp-echo" => {
            let s = udp::UdpServer::new(addr);
            s.echo()
                .await
                .map_err(|e| format!("udp-echo error: {}", e))?;
        }
        _ => {
            return Err(format!("unknown handler: {}", handler).into());
        }
    };

    Ok(())
}
