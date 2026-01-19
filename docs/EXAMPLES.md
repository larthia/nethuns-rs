# Nethuns-rs Examples Guide

This directory contains example applications demonstrating various use cases of the Nethuns-rs library.

## Available Examples

| Example | Description | Platforms |
|---------|-------------|-----------|
| `meter` | Packet capture throughput measurement | All |
| `meter2` | Alternative meter implementation with token API | All |
| `forward` | Simple packet forwarder between interfaces | All |
| `forward_mt` | Multi-threaded packet forwarder | All |
| `pkt-gen` | High-speed packet generator | All |

## Running Examples

### Prerequisites

All examples require root/sudo access for packet capture capabilities.

### On macOS

Only the pcap backend is available on macOS:

```bash
# Packet meter
sudo cargo run --example meter -- -i en0 pcap

# Packet forwarder
sudo cargo run --example forward -- en0 en1 pcap

# Packet generator
sudo cargo run --example pkt-gen -- -i en0 \
    --dst-mac ff:ff:ff:ff:ff:ff \
    --src-ip 10.0.0.1 --dst-ip 10.0.0.2 \
    pcap
```

### On Linux

You can use additional high-performance backends:

```bash
# With AF_XDP (requires kernel 4.19+)
sudo cargo run --example meter --features af_xdp -- -i eth0 af-xdp

# With netmap (requires netmap kernel module)
sudo cargo run --example meter --features netmap -- -i eth0 netmap

# With DPDK (requires DPDK environment)
sudo cargo run --example meter --features dpdk -- -i eth0 dpdk
```

---

## Example 1: Packet Meter (`meter`)

The meter example demonstrates basic packet capture and throughput measurement.

### Usage

```bash
cargo run --example meter -- [OPTIONS] <FRAMEWORK>
```

### Options

| Option | Description |
|--------|-------------|
| `-i, --interface <NAME>` | Network interface name (required) |
| `--queue <N>` | Specific queue to capture from |
| `-s, --sockets <N>` | Number of sockets to use (default: 1) |
| `-m, --multithreading` | Enable multi-threaded capture |
| `-S, --sockstats <ID>` | Show stats for specific socket |
| `-d, --debug` | Enable debug output |

### Framework Subcommands

**pcap** (all platforms):

```bash
sudo cargo run --example meter -- -i eth0 pcap \
    --snaplen 65535 \
    --promiscuous true \
    --timeout-ms 1
```

**af-xdp** (Linux only):

```bash
sudo cargo run --example meter --features af_xdp -- -i eth0 af-xdp \
    --bind-flags 0 \
    --xdp-flags 0
```

**netmap** (Linux/FreeBSD):

```bash
sudo cargo run --example meter --features netmap -- -i eth0 netmap \
    --extra-buf 1024
```

### Code Walkthrough

The key pattern in meter.rs:

```rust
// Create a socket with the selected backend
let socket = Sock::create(&args.interface, args.queue, flags)?;

// Receive packets in a loop
loop {
    match socket.recv() {
        Ok((packet, _metadata)) => {
            // Increment counter
            total.fetch_add(1, Ordering::Relaxed);
            
            // Optionally process packet
            if debug {
                println!("Packet: {} bytes", packet.len());
            }
        }
        Err(_) => continue,
    }
}
```

---

## Example 2: Packet Forwarder (`forward`)

Demonstrates packet forwarding between two network interfaces.

### Usage

```bash
cargo run --example forward -- <IN_IF> <OUT_IF> [OPTIONS] <FRAMEWORK>
```

### Example

```bash
# Forward packets from eth0 to eth1 using pcap
sudo cargo run --example forward -- eth0 eth1 pcap

# Forward using netmap on queue 0
sudo cargo run --example forward --features netmap -- eth0 eth1 --queue 0 netmap
```

### Code Pattern

```rust
// Create input and output sockets
let in_socket = Sock::create(&args.in_if, args.queue, flags.clone())?;
let out_socket = Sock::create(&args.out_if, args.queue, flags)?;

// Forward loop
while !term.load(Ordering::SeqCst) {
    // Receive from input
    let (packet, _meta) = match in_socket.recv() {
        Ok((p, m)) => (p, m),
        Err(_) => continue,
    };
    
    // Send to output
    loop {
        match out_socket.send(&packet) {
            Ok(_) => break,
            Err(_) => out_socket.flush(),
        }
    }
}
```

---

## Example 3: Packet Generator (`pkt-gen`)

High-speed traffic generator similar to netmap's pkt-gen.

### Usage

```bash
cargo run --example pkt-gen -- [OPTIONS] <FRAMEWORK>
```

### Options

| Option | Description |
|--------|-------------|
| `-i, --interface` | Network interface |
| `--sockets <N>` | Number of sending sockets |
| `-m, --multithreading` | Use one thread per socket |
| `-n, --count <N>` | Total packets to send (0 = unlimited) |
| `-r, --rate <PPS>` | Target packets per second |
| `-l, --len <BYTES>` | Packet length (default: 60) |
| `--src-mac` | Source MAC address |
| `--dst-mac` | Destination MAC address (required) |
| `--src-ip` | Source IP address |
| `--dst-ip` | Destination IP address |
| `--src-port` | UDP source port |
| `--dst-port` | UDP destination port |

### Example

```bash
# Generate traffic at 10 Mpps on 4 cores
sudo cargo run --example pkt-gen --features netmap -- \
    -i eth0 \
    --sockets 4 \
    --multithreading \
    --dst-mac 11:22:33:44:55:66 \
    --src-ip 10.0.0.1 \
    --dst-ip 10.0.0.2 \
    -r 10000000 \
    netmap
```

---

## Example 4: Multi-threaded Forwarder (`forward_mt`)

Demonstrates multi-threaded packet forwarding with per-queue processing.

### Features

- One thread per queue pair
- Automatic load balancing via RSS
- Zero-copy forwarding (when supported by backend)

### Usage

```bash
sudo cargo run --example forward_mt --features netmap -- \
    -i eth0 -o eth1 \
    --threads 4 \
    netmap
```

---

## Performance Benchmarking

### Methodology

1. **Baseline**: Start with pcap to verify functionality
2. **Optimize**: Switch to high-performance backend (AF_XDP/netmap/DPDK)
3. **Scale**: Add multiple sockets and threads
4. **Tune**: Adjust buffer sizes and batch parameters

### Expected Performance

| Backend | 64-byte packets | 1500-byte packets |
|---------|----------------|-------------------|
| pcap | ~1 Mpps | ~100 Kpps |
| AF_XDP | ~10 Mpps | ~4 Mpps |
| netmap | ~14 Mpps | ~5 Mpps |
| DPDK | ~14+ Mpps | ~5+ Mpps |

*Performance varies by hardware, kernel version, and configuration.*

---

## Troubleshooting

### Common Issues

**"Permission denied"**

```bash
# Run with sudo
sudo cargo run --example meter -- -i eth0 pcap
```

**"Interface not found"**

```bash
# List available interfaces
ip link show  # Linux
ifconfig     # macOS
```

**"AF_XDP: BPF program failed"**

```bash
# Check kernel version (need 4.19+)
uname -r

# Check BPF support
ls /sys/fs/bpf
```

**"netmap: No such device"**

```bash
# Load netmap kernel module
sudo insmod /usr/local/lib/netmap.ko

# Check if loaded
lsmod | grep netmap
```

---

## Writing Your Own Application

### Template

```rust
use nethuns_rs::api::Socket;
use nethuns_rs::pcap::{Sock, PcapFlags};

fn main() -> anyhow::Result<()> {
    // 1. Configure the backend
    let flags = PcapFlags::default();
    
    // 2. Create the socket
    let socket = Sock::create("eth0", None, flags)?;;
    
    // 3. Process packets
    loop {
        let (packet, metadata) = socket.recv()?;
        
        // Your processing logic here
        process_packet(&packet);
    }
}

fn process_packet(packet: &[u8]) {
    // Parse headers, filter, transform, etc.
}
```

### Cross-Platform Pattern

```rust
#[cfg(all(target_os = "linux", feature = "af_xdp"))]
use nethuns_rs::af_xdp::{Sock as AfXdpSocket, AfXdpFlags};

use nethuns_rs::pcap::{Sock as PcapSocket, PcapFlags};
use nethuns_rs::api::Socket;

fn main() -> anyhow::Result<()> {
    #[cfg(all(target_os = "linux", feature = "af_xdp"))]
    {
        let flags = AfXdpFlags { /* ... */ };
        run_with_socket(AfXdpSocket::create("eth0", None, flags)?)?;
    }
    
    #[cfg(not(all(target_os = "linux", feature = "af_xdp")))]
    {
        let flags = PcapFlags::default();
        run_with_socket(PcapSocket::create("eth0", None, flags)?)?;
    }
    
    Ok(())
}

fn run_with_socket<S: Socket>(socket: S) -> anyhow::Result<()> {
    loop {
        let (packet, _) = socket.recv()?;
        // Process...
    }
}
```
