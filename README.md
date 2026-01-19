# Nethuns-rs

A unified API for fast and portable network programming in Rust.

[![License](https://img.shields.io/badge/license-BSD--3--Clause-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-2021-orange.svg)](https://www.rust-lang.org/)

## Introduction

Nethuns-rs is a Rust library that provides a unified API to access and manage low-level network operations over different underlying network I/O frameworks. This is a Rust port of the original [Nethuns C library](https://github.com/larthia/nethuns).

The design of Nethuns originates from the practical requirement of developing portable network applications with extremely high data rate targets. Instead of re-writing applications to match the underlying network I/O engines available over different operating systems, Nethuns offers a unified abstraction layer that allows programmers to implement their applications regardless of the underlying technology.

Network applications using Nethuns-rs only need to be **re-compiled** to run on top of a different engine — with no code adaptation required.

## Supported Backends

| Backend | Platform | Feature Flag | Description |
|---------|----------|--------------|-------------|
| **pcap** | All (Linux, macOS, BSD, Windows) | Default | libpcap-based capture, works everywhere |
| **AF_XDP** | Linux | `af_xdp` | High-performance packet I/O using Linux AF_XDP sockets |
| **netmap** | Linux, FreeBSD | `netmap` | Fast packet I/O using the netmap framework |
| **DPDK** | Linux | `dpdk` | Data Plane Development Kit for line-rate packet processing |

## Installation

Add Nethuns-rs to your `Cargo.toml`:

```toml
[dependencies]
nethuns_rs = { git = "https://github.com/leonardogiovannoni/nethuns-rs" }
```

### Enabling Platform-Specific Backends

On Linux, you can enable additional high-performance backends:

```toml
[dependencies]
nethuns_rs = { git = "https://github.com/leonardogiovannoni/nethuns-rs", features = ["af_xdp"] }
```

Or enable multiple backends:

```toml
[dependencies]
nethuns_rs = { git = "https://github.com/leonardogiovannoni/nethuns-rs", features = ["af_xdp", "netmap", "dpdk"] }
```

## Quick Start

### Basic Packet Capture (pcap - works on all platforms)

```rust
use nethuns_rs::api::Socket;
use nethuns_rs::pcap::{Sock, PcapFlags};

fn main() -> anyhow::Result<()> {
    // Create socket with default flags
    let flags = PcapFlags::default();
    let socket = Sock::create("eth0", None, flags)?;;
    
    // Receive packets
    loop {
        match socket.recv() {
            Ok((packet, metadata)) => {
                println!("Received {} bytes", packet.len());
                // Process packet...
            }
            Err(_) => continue,
        }
    }
}
```

### High-Performance Capture with AF_XDP (Linux)

```rust
#[cfg(all(target_os = "linux", feature = "af_xdp"))]
use nethuns_rs::af_xdp::{Sock as AfXdpSocket, AfXdpFlags};
use nethuns_rs::api::Socket;

fn main() -> anyhow::Result<()> {
    let flags = AfXdpFlags {
        bind_flags: 0,
        xdp_flags: 0,
        num_frames: 4096,
        frame_size: 4096,
        tx_size: 2048,
        rx_size: 2048,
    };
    
    let socket = AfXdpSocket::create("eth0", Some(0), flags)?;
    
    loop {
        match socket.recv() {
            Ok((packet, _metadata)) => {
                println!("Received {} bytes via AF_XDP", packet.len());
            }
            Err(_) => continue,
        }
    }
}
```

## API Overview

### Core Traits

#### `Socket`

The main trait for network I/O operations:

```rust
pub trait Socket: Send + Sized {
    type Context: Context;
    type Metadata: Metadata;
    type Flags: Flags;
    
    /// Receive a packet and its metadata
    fn recv(&self) -> Result<(Payload<'_, Self::Context>, Self::Metadata)>;
    
    /// Send a packet
    fn send(&self, packet: &[u8]) -> Result<()>;
    
    /// Flush pending transmissions
    fn flush(&self);
    
    /// Create a new socket bound to the specified interface
    fn create(portspec: &str, queue: Option<usize>, flags: Self::Flags) -> Result<Self>;
    
    /// Get the socket's context
    fn context(&self) -> &Self::Context;
}
```

#### `Context`

Manages buffer ownership and provides safe access to packet data:

```rust
pub trait Context: Sized + Clone + Send + 'static {
    /// Get the pool ID for token validation
    fn pool_id(&self) -> u32;
    
    /// Access the underlying buffer (unsafe)
    unsafe fn unsafe_buffer(&self, buf_idx: BufferDesc, size: usize) -> *mut [u8];
    
    /// Release a buffer back to the pool
    fn release(&self, buf_idx: BufferDesc);
}
```

### Backend-Specific Flags

Each backend has its own flags structure for configuration:

```rust
// pcap
let pcap_flags = PcapFlags {
    snaplen: 65535,
    promiscuous: true,
    timeout_ms: 1,
    immediate: true,
    filter: Some("tcp port 80".to_string()),
    buffer_size: 2048,
    buffer_count: 32,
};

// AF_XDP
let xdp_flags = AfXdpFlags {
    bind_flags: 0,
    xdp_flags: 0,
    num_frames: 4096,
    frame_size: 4096,
    tx_size: 2048,
    rx_size: 2048,
};

// netmap
let netmap_flags = NetmapFlags {
    extra_buf: 1024,
};
```

## Examples

The library includes several example applications in the `examples/` directory:

### Packet Meter

Measures packet capture rate on an interface:

```bash
# On macOS (pcap only)
sudo cargo run --example meter -- -i en0 pcap

# On Linux with AF_XDP
sudo cargo run --example meter --features af_xdp -- -i eth0 af-xdp

# On Linux with netmap
sudo cargo run --example meter --features netmap -- -i eth0 netmap
```

### Packet Forwarder

Forwards packets between two interfaces:

```bash
# On macOS
sudo cargo run --example forward -- en0 en1 pcap

# On Linux with netmap
sudo cargo run --example forward --features netmap -- eth0 eth1 netmap

# With a specific queue
sudo cargo run --example forward --features netmap -- eth0 eth1 --queue 0 netmap
```

### Packet Generator

High-speed traffic generator:

```bash
sudo cargo run --example pkt-gen --features netmap -- \
    -i eth0 --sockets 4 --multithreading \
    --dst-mac 11:22:33:44:55:66 \
    --src-ip 10.0.0.1 --dst-ip 10.0.0.2 \
    netmap
```

## Platform-Specific Setup

### Linux

#### AF_XDP

AF_XDP requires:

- Linux kernel 4.19 or later (5.x recommended)
- libbpf and libxdp development headers
- CAP_NET_ADMIN capability or root access

```bash
# Ubuntu/Debian
sudo apt install libbpf-dev libxdp-dev

# Build with AF_XDP support
cargo build --features af_xdp
```

#### netmap

netmap requires the netmap kernel module and library:

```bash
# Clone and build netmap
git clone https://github.com/luigirizzo/netmap
cd netmap
./configure && make && sudo make install
sudo insmod netmap.ko

# Build with netmap support
cargo build --features netmap
```

#### DPDK

DPDK requires the DPDK libraries and environment setup:

```bash
# Install DPDK (Ubuntu)
sudo apt install dpdk dpdk-dev

# Build with DPDK support
cargo build --features dpdk
```

### macOS

On macOS, only the pcap backend is supported (default):

```bash
# libpcap is included with macOS, just build
cargo build

# Run with sudo (required for packet capture)
sudo cargo run --example meter -- -i en0 pcap
```

### FreeBSD

On FreeBSD, pcap and netmap are supported:

```bash
# netmap is included in FreeBSD
cargo build --features netmap
```

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    Application Layer                         │
│              (Your Network Application Code)                 │
└──────────────────────────┬──────────────────────────────────┘
                           │
┌──────────────────────────▼──────────────────────────────────┐
│                  Nethuns-rs API                              │
│         Socket, Context, Payload, Token traits               │
└──────┬─────────┬─────────┬─────────┬────────────────────────┘
       │         │         │         │
┌──────▼───┐ ┌───▼────┐ ┌──▼───┐ ┌───▼──┐
│  pcap    │ │ AF_XDP │ │netmap│ │ DPDK │
│(portable)│ │(Linux) │ │(L/FB)│ │(Linux)│
└──────────┘ └────────┘ └──────┘ └──────┘
```

## Performance Tips

1. **Use AF_XDP on Linux** for maximum performance with commodity NICs
2. **Use DPDK** when you need the absolute highest throughput
3. **Pin to CPU cores** when using multiple sockets with `--multithreading`
4. **Increase ring buffer sizes** for high packet rates
5. **Use batch operations** when possible (not yet exposed in the public API)

## Comparison with Original C Library

| Feature | C Nethuns | Rust Nethuns-rs |
|---------|-----------|-----------------|
| Memory Safety | Manual | Guaranteed |
| Zero-copy | Yes | Yes |
| pcap | ✓ | ✓ |
| AF_XDP | ✓ | ✓ |
| netmap | ✓ | ✓ |
| DPDK | ✓ | ✓ |
| TPACKET_V3 | ✓ | Planned |
| Thread Safety | Manual | Type-enforced |

## Contributing

Contributions are welcome! Please feel free to submit issues and pull requests.

### Building from Source

```bash
git clone https://github.com/leonardogiovannoni/nethuns-rs
cd nethuns-rs
cargo build
cargo test
```

## License

This project is licensed under the BSD-3-Clause License - see the [LICENSE](LICENSE) file for details.

## Citation

If you use this library in your research, please cite the original Nethuns paper:

```bibtex
@article{10.1145/3544912.3544917,
    author = {Bonelli, Nicola and Del Vigna, Fabio and Fais, Alessandra and Lettieri, Giuseppe and Procissi, Gregorio},
    title = {{Programming Socket-Independent Network Functions with Nethuns}},
    year = {2022},
    journal = {SIGCOMM Computer Communication Review},
    volume = {52},
    number = {2},
    pages = {35–48},
    doi = {10.1145/3544912.3544917}
}
```

## Credits

### Original Authors

- Nicola Bonelli

### Contributors

- Fabio Del Vigna
- Alessandra Fais
- Giuseppe Lettieri
- Gregorio Procissi

### Rust Port

- Leonardo Giovannoni

## See Also

- [Original Nethuns (C)](https://github.com/larthia/nethuns) - The original C implementation
- [libpcap](https://www.tcpdump.org/) - Portable packet capture library
- [netmap](https://github.com/luigirizzo/netmap) - Fast packet I/O framework
- [DPDK](https://www.dpdk.org/) - Data Plane Development Kit
- [AF_XDP](https://www.kernel.org/doc/html/latest/networking/af_xdp.html) - Linux Express Data Path
