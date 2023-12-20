# Isolated Network Namespace Servers in Rust

This repository hosts a simple Rust-based proof of concept (PoC) for implementing TCP and UDP echo servers, running in its own isolated network namespace. It's a exploration into advanced networking concepts in Rust, leveraging the power of network namespaces for process isolation and the Tokio runtime for asynchronous operations.

## Getting Started

### Prerequisites

Before you begin, ensure you have the following installed:

* Rust programming language: Install Rust
* Cargo, Rust's package manager (usually comes with Rust)

### Installation

1. Clone the Repository

First, clone the repository to your local machine using Git:

```bash
git clone [repository-url]
cd [repository-name]
```

2. Build the Project
Compile the project using Cargo:

```bash
cargo build --release
```

### Running the Server

Run the server using the command below. The arguments have default values, so you can omit them if the defaults work for your setup.

```bash
sudo RUST_LOG=info cargo run -- --server-addr [server address] --handler [handler] --bridge-name [bridge name] --bridge-ip [bridge IP] --subnet [subnet mask] --ns-ip [namespace IP]
```

Values
* --server-addr: No default value, must be specified (e.g., "0.0.0.0:8080").
* --handler: Default is "tcp-echo". Options are "tcp-echo" or "udp-echo".
* --bridge-name: Default is "isobr0".
* --bridge-ip: Default is "172.18.0.1".
* --subnet: Default is "16".
* --ns-ip: No default value, must be specified (e.g., "172.18.0.2").

**Examples**

* TCP Echo Server

```bash
sudo RUST_LOG=info ./isoserver --server-addr "0.0.0.0:8080" --ns-ip 172.18.0.2
```

This runs a TCP echo server with the default network configuration.

* UDP Echo Server

```bash
sudo RUST_LOG=info ./isoserver --server-addr "0.0.0.0:8081" --handler udp-echo --ns-ip 172.18.0.3
```

This starts a UDP echo server, also using default network settings, but with a different namespace IP.

## Use Case

This project serves as an educational tool for those interested in network programming, process isolation, and the practical application of network namespaces in Rust. It's ideal for understanding the intricacies of network communication in isolated environments.

## Contributing

Contributions, suggestions, and discussions are welcome! Whether you're enhancing the functionality, refining the concept, or fixing bugs, your input is valuable.
