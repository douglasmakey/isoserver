mod handlers;
mod net;
mod string_helpers;

use crate::handlers::execute;
use crate::net::{join_veth_to_ns, prepare_net, setup_veth_peer};
use clap::Parser;
use log::{error, info, warn};
use nix::sched::*;
use nix::sys::signal::Signal;
use nix::sys::wait::{waitpid, WaitStatus};
use std::{thread, time};

const STACK_SIZE: usize = 1024 * 1024;

#[derive(Debug, Parser, Clone)]
struct Args {
    #[arg(short, long)]
    /// The server address
    server_addr: String,
    #[arg(long, default_value = "tcp-echo")]
    /// The handler to use (tcp-echo, udp-echo)
    handler: String,
    #[arg(long, default_value = "isobr0")]
    /// This is the name of the bridge to create.
    bridge_name: String,
    #[arg(long, default_value = "172.18.0.1")]
    /// This is the IP address for the bridge.
    bridge_ip: String,
    #[arg(long, default_value = "16")]
    /// This is the subnet for the bridge.
    subnet: u8,
    /// This is the IP address for the process in the new namespace.
    /// For instance, if the default bridge IP ('172.18.0.1') is used, the value could be '172.18.0.2'.
    /// Please ensure this IP address is not already in use by another process/ns and is within the same subnet as the bridge IP.
    #[arg(long, required = true)]
    ns_ip: String,
}

fn main() {
    env_logger::init();
    let args = Args::parse();
    let rt = tokio::runtime::Runtime::new().expect("Failed to create Tokio runtime");
    let (_, _, veth2_idx) = rt
        .block_on(prepare_net(
            args.bridge_name.clone(),
            &args.bridge_ip,
            args.subnet,
        ))
        .expect("Failed to prepare network");

    // prepare child process
    let cb = Box::new(|| c_process(&args, veth2_idx));
    // let mut stack = [0; 512];
    let mut tmp_stack: [u8; STACK_SIZE] = [0; STACK_SIZE];
    let child_pid = unsafe {
        clone(
            cb,
            &mut tmp_stack,
            CloneFlags::CLONE_NEWNET | CloneFlags::CLONE_NEWUTS,
            Some(Signal::SIGCHLD as i32),
        )
    }
    .expect("Clone failed");

    info!("Parent pid: {}", nix::unistd::getpid());

    rt.block_on(async {
        join_veth_to_ns(veth2_idx, child_pid.as_raw() as u32)
            .await
            .expect("Failed to join veth to namespace");
    });

    thread::sleep(time::Duration::from_millis(500));

    match waitpid(child_pid, None) {
        Ok(WaitStatus::Exited(pid, status)) => {
            warn!(
                "Child process (PID: {}) exited with status: {}",
                pid, status
            );
        }
        Ok(WaitStatus::Signaled(pid, signal, _)) => {
            warn!(
                "Child process (PID: {}) was killed by signal: {:?}",
                pid, signal
            );
        }
        Err(e) => eprintln!("waitpid failed: {}", e),
        _ => error!("Error: Unexpected waitpid result"),
    }
}

fn c_process(args: &Args, veth_peer_idx: u32) -> isize {
    info!("Child process (PID: {}) started", nix::unistd::getpid());
    // Set the hostname of the new process
    let ns_hostname = format!("isoserver-{}", string_helpers::random_suffix(5));
    nix::unistd::sethostname(ns_hostname).expect("Failed to set hostname");

    // Spawn a new blocking task on the current runtime
    let rt = tokio::runtime::Runtime::new().expect("Failed to create Tokio runtime");
    let process = rt.block_on(async {
        setup_veth_peer(veth_peer_idx, &args.ns_ip, args.subnet).await?;
        execute(args.handler.clone(), args.server_addr.clone()).await
    });

    if let Err(e) = process {
        error!("Error: {}", e);
        return -1;
    }

    info!("Child process finished??");
    0
}
