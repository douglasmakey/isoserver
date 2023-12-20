use futures::TryStreamExt;
use log::info;
use rand::distributions::{Alphanumeric, DistString};
use rtnetlink::{new_connection, AddressHandle, Handle};
use std::{net::Ipv4Addr, str::FromStr};

use std::fmt;

#[derive(Debug)]
pub enum NetworkError {
    ConnectionError(rtnetlink::Error),
    OperationError(String),
    AddressParseError(std::net::AddrParseError),
    Other(std::io::Error),
}

impl fmt::Display for NetworkError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NetworkError::ConnectionError(e) => write!(f, "Connection error: {}", e),
            NetworkError::OperationError(msg) => write!(f, "Operation error: {}", msg),
            NetworkError::AddressParseError(e) => write!(f, "Address parse error: {}", e),
            NetworkError::Other(e) => write!(f, "IO error: {}", e),
        }
    }
}

impl std::error::Error for NetworkError {}

// Implementing From<T> for NetworkError allows you to use '?' in functions that return Result<T, NetworkError>
impl From<rtnetlink::Error> for NetworkError {
    fn from(err: rtnetlink::Error) -> Self {
        NetworkError::ConnectionError(err)
    }
}

impl From<std::net::AddrParseError> for NetworkError {
    fn from(err: std::net::AddrParseError) -> Self {
        NetworkError::AddressParseError(err)
    }
}

impl From<std::io::Error> for NetworkError {
    fn from(err: std::io::Error) -> Self {
        NetworkError::Other(err)
    }
}

fn random_suffix() -> String {
    Alphanumeric.sample_string(&mut rand::thread_rng(), 4)
}

pub async fn prepare_net(
    bridge_name: String,
    bridge_ip: &str,
    subnet: u8,
) -> Result<(u32, u32, u32), NetworkError> {
    let (connection, handle, _) = new_connection()?;
    tokio::spawn(connection);

    info!("Interact with bridge {bridge_name} at cidr {bridge_ip}/{subnet}");

    // create bridge if not exist
    let bridge_idx = match get_bridge_idx(&handle, bridge_name.clone()).await {
        Ok(idx) => {
            info!("bridge {} already exist", bridge_name);
            idx
        }
        Err(_) => create_bridge(bridge_name, bridge_ip, subnet).await?,
    };

    let (veth_idx, veth2_idx) = create_veth_pair(bridge_idx).await?;
    Ok((bridge_idx, veth_idx, veth2_idx))
}

async fn get_bridge_idx(handle: &Handle, bridge_name: String) -> Result<u32, NetworkError> {
    // retrieve bridge index
    let bridge_idx = handle
        .link()
        .get()
        .match_name(bridge_name)
        .execute()
        .try_next()
        .await?
        .ok_or_else(|| NetworkError::OperationError("failed to get bridge index".to_string()))?
        .header
        .index;

    Ok(bridge_idx)
}

async fn create_bridge(name: String, bridge_ip: &str, subnet: u8) -> Result<u32, NetworkError> {
    let (connection, handle, _) = new_connection()?;
    tokio::spawn(connection);

    // Create a bridge
    handle
        .link()
        .add()
        .bridge(name.clone())
        .execute()
        .await
        .map_err(|e| {
            NetworkError::OperationError(format!(
                "create bridge with name {} failed: {}",
                name.clone(),
                e
            ))
        })?;

    // Bring up the bridge
    let bridge_idx = handle
        .link()
        .get()
        .match_name(name)
        .execute()
        .try_next()
        .await?
        .ok_or_else(|| NetworkError::OperationError("failed to get bridge index".to_string()))?
        .header
        .index;

    // add ip address to bridge
    let bridge_addr = std::net::IpAddr::V4(Ipv4Addr::from_str(bridge_ip)?);
    AddressHandle::new(handle.clone())
        .add(bridge_idx, bridge_addr, subnet)
        .execute()
        .await
        .map_err(|e| {
            NetworkError::OperationError(format!("add IP address to bridge failed: {}", e))
        })?;

    // set bridge up
    handle
        .link()
        .set(bridge_idx)
        .up()
        .execute()
        .await
        .map_err(|e| {
            NetworkError::OperationError(format!(
                "set bridge with idx {} to up failed: {}",
                bridge_idx, e
            ))
        })?;

    Ok(bridge_idx)
}

async fn create_veth_pair(bridge_idx: u32) -> Result<(u32, u32), NetworkError> {
    let (connection, handle, _) = new_connection()?;
    tokio::spawn(connection);

    // create veth interfaces
    let veth: String = format!("veth{}", random_suffix());
    let veth_2: String = format!("{}_peer", veth.clone());

    handle
        .link()
        .add()
        .veth(veth.clone(), veth_2.clone())
        .execute()
        .await
        .map_err(|e| {
            NetworkError::OperationError(format!(
                "create veth pair {} and {} failed: {}",
                veth, veth_2, e
            ))
        })?;

    let veth_idx = handle
        .link()
        .get()
        .match_name(veth.clone())
        .execute()
        .try_next()
        .await?
        .ok_or_else(|| NetworkError::OperationError("failed to get veth index".to_string()))?
        .header
        .index;

    let veth_2_idx = handle
        .link()
        .get()
        .match_name(veth_2.clone())
        .execute()
        .try_next()
        .await?
        .ok_or_else(|| NetworkError::OperationError("failed to get veth index".to_string()))?
        .header
        .index;

    // set master veth up
    handle
        .link()
        .set(veth_idx)
        .up()
        .execute()
        .await
        .map_err(|e| {
            NetworkError::OperationError(format!(
                "set veth with idx {} to up failed: {}",
                veth_idx, e
            ))
        })?;

    // set master veth to bridge
    handle
        .link()
        .set(veth_idx)
        .controller(bridge_idx)
        .execute()
        .await
        .map_err(|e| {
            NetworkError::OperationError(format!(
                "set veth with idx {} to bridge with idx {} failed: {}",
                veth_idx, bridge_idx, e
            ))
        })?;

    Ok((veth_idx, veth_2_idx))
}

pub async fn join_veth_to_ns(veth_idx: u32, pid: u32) -> Result<(), NetworkError> {
    let (connection, handle, _) = new_connection()?;
    tokio::spawn(connection);

    // set veth to the process network namespace
    handle
        .link()
        .set(veth_idx)
        .setns_by_pid(pid)
        .execute()
        .await
        .map_err(|e| {
            NetworkError::OperationError(format!(
                "set veth with idx {} to process with pid {} failed: {}",
                veth_idx, pid, e
            ))
        })?;

    Ok(())
}

pub async fn setup_veth_peer(
    veth_idx: u32,
    ns_ip: &String,
    subnet: u8,
) -> Result<(), NetworkError> {
    let (connection, handle, _) = new_connection()?;
    tokio::spawn(connection);

    info!("setup veth peer with ip: {}/{}", ns_ip, subnet);

    // set veth peer address
    let veth_2_addr = std::net::IpAddr::V4(Ipv4Addr::from_str(ns_ip)?);
    AddressHandle::new(handle.clone())
        .add(veth_idx, veth_2_addr, subnet)
        .execute()
        .await
        .map_err(|e| {
            NetworkError::OperationError(format!("add IP address to veth peer failed: {}", e))
        })?;

    handle
        .link()
        .set(veth_idx)
        .up()
        .execute()
        .await
        .map_err(|e| {
            NetworkError::OperationError(format!(
                "set veth with idx {} to up failed: {}",
                veth_idx, e
            ))
        })?;

    // set lo interface to up
    let lo_idx = handle
        .link()
        .get()
        .match_name("lo".to_string())
        .execute()
        .try_next()
        .await?
        .ok_or_else(|| NetworkError::OperationError("failed to get lo index".to_string()))?
        .header
        .index;

    handle
        .link()
        .set(lo_idx)
        .up()
        .execute()
        .await
        .map_err(|e| {
            NetworkError::OperationError(format!(
                "set lo interface with idx {} to up failed: {}",
                lo_idx, e
            ))
        })?;

    Ok(())
}
