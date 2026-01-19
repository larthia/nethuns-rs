# Nethuns-rs API Reference

This document provides a comprehensive reference for the Nethuns-rs API.

## Table of Contents

1. [Core Traits](#core-traits)
2. [Socket Types](#socket-types)
3. [Context and Memory Management](#context-and-memory-management)
4. [Configuration Flags](#configuration-flags)
5. [Error Handling](#error-handling)
6. [Metadata Types](#metadata-types)

---

## Core Traits

### `Socket`

The primary trait for all network I/O operations.

```rust
pub trait Socket: Send + Sized {
    type Context: Context;
    type Metadata: Metadata;
    type Flags: Flags;
    
    /// Receive a packet from the socket.
    /// 
    /// Returns a tuple of (Payload, Metadata) on success.
    /// The Payload provides access to the packet data and is automatically
    /// released when dropped.
    fn recv(&self) -> Result<(Payload<'_, Self::Context>, Self::Metadata)>;
    
    /// Receive a packet as a token (advanced API).
    /// 
    /// Returns a Token that must be explicitly consumed via `context.packet(token)`.
    /// This is useful when you need to transfer ownership across contexts.
    fn recv_token(&self) -> Result<(Token, Self::Metadata)>;
    
    /// Send a packet.
    /// 
    /// The packet data is copied into the transmission ring.
    fn send(&self, packet: &[u8]) -> Result<()>;
    
    /// Flush pending transmissions.
    /// 
    /// Forces any queued packets to be transmitted immediately.
    fn flush(&self);
    
    /// Create a new socket bound to the specified interface.
    /// 
    /// # Arguments
    /// * `portspec` - Interface name (e.g., "eth0", "en0", "netmap:eth0")
    /// * `queue` - Optional queue number for multi-queue NICs
    /// * `flags` - Backend-specific configuration flags
    fn create(portspec: &str, queue: Option<usize>, flags: Self::Flags) -> Result<Self>;
    
    /// Get a reference to the socket's context.
    fn context(&self) -> &Self::Context;
}
```

### `Context`

Manages buffer pools and provides safe access to packet memory.

```rust
pub trait Context: Sized + Clone + Send + 'static {
    /// Get the unique pool ID for this context.
    /// 
    /// Used to validate that tokens belong to this context.
    fn pool_id(&self) -> u32;
    
    /// Get a raw pointer to a buffer.
    /// 
    /// # Safety
    /// Caller must ensure exclusive access to the buffer region.
    unsafe fn unsafe_buffer(&self, buf_idx: BufferDesc, size: usize) -> *mut [u8];
    
    /// Release a buffer back to the pool.
    /// 
    /// Called automatically when a Payload is dropped.
    fn release(&self, buf_idx: BufferDesc);
    
    /// Convert a token into a Payload.
    /// 
    /// # Panics
    /// Panics if the token's pool_id doesn't match this context.
    fn packet<'ctx>(&'ctx self, token: Token) -> Payload<'ctx, Self>;
}
```

### `Flags`

Marker trait for backend-specific configuration.

```rust
pub trait Flags: Clone + Debug {}
```

### `Metadata`

Trait for packet metadata.

```rust
pub trait Metadata: Send {
    fn into_enum(self) -> MetadataType;
}
```

---

## Socket Types

### `pcap::Sock` (All Platforms)

libpcap-based packet capture, works on all platforms.

```rust
use nethuns_rs::pcap::{Sock, PcapFlags};

let flags = PcapFlags {
    snaplen: 65535,          // Maximum capture length
    promiscuous: true,       // Capture all packets
    timeout_ms: 1,           // Read timeout
    immediate: true,         // Deliver packets immediately
    filter: None,            // Optional BPF filter
    buffer_size: 2048,       // Buffer size per packet
    buffer_count: 32,        // Number of buffers
};

let socket = Sock::create("eth0", None, flags)?;
```

### `af_xdp::Sock` (Linux only)

High-performance AF_XDP socket.

```rust
#[cfg(all(target_os = "linux", feature = "af_xdp"))]
use nethuns_rs::af_xdp::{Sock, AfXdpFlags};

let flags = AfXdpFlags {
    bind_flags: 0,           // XDP_ZEROCOPY, XDP_COPY, etc.
    xdp_flags: 0,            // XDP_FLAGS_* values
    num_frames: 4096,        // UMEM frame count
    frame_size: 4096,        // UMEM frame size
    tx_size: 2048,           // TX ring size
    rx_size: 2048,           // RX ring size
};

let socket = Sock::create("eth0", Some(0), flags)?;
```

### `netmap::Sock` (Linux/FreeBSD)

netmap-based packet I/O.

```rust
#[cfg(all(any(target_os = "linux", target_os = "freebsd"), feature = "netmap"))]
use nethuns_rs::netmap::{Sock, NetmapFlags};

let flags = NetmapFlags {
    extra_buf: 1024,         // Extra buffer count
};

let socket = Sock::create("netmap:eth0", Some(0), flags)?;
```

### `dpdk::Sock` (Linux only)

DPDK-based packet I/O.

```rust
#[cfg(all(target_os = "linux", feature = "dpdk"))]
use nethuns_rs::dpdk::{Sock, DpdkFlags};

let flags = DpdkFlags {
    num_mbufs: 8192,
    mbuf_cache_size: 250,
    mbuf_default_buf_size: 2176,
};

let socket = Sock::create("0000:01:00.0", Some(0), flags)?;
```

---

## Context and Memory Management

### `Payload<'ctx, Ctx>`

Smart pointer to packet data with automatic cleanup.

```rust
/// Represents a received packet with automatic lifetime management.
#[repr(C)]
pub struct Payload<'ctx, Ctx: Context> {
    token: ManuallyDrop<Token>,
    ctx: &'ctx Ctx,
}

impl<'ctx, Ctx: Context> Deref for Payload<'ctx, Ctx> {
    type Target = [u8];
    
    fn deref(&self) -> &Self::Target {
        // Returns the packet data as a byte slice
    }
}

impl<'ctx, Ctx: Context> DerefMut for Payload<'ctx, Ctx> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        // Returns mutable access to packet data
    }
}
```

**Usage:**

```rust
let (packet, _meta) = socket.recv()?;

// Access packet data via Deref
println!("First byte: {}", packet[0]);

// Packet is automatically released when `packet` goes out of scope
```

### `Token`

Represents ownership of a buffer without borrowing the context.

```rust
pub struct Token {
    idx: BufferDesc,
    len: u32,
    buffer_pool: u32,
}

impl Token {
    /// Get the buffer descriptor
    pub fn buffer_desc(&self) -> BufferDesc;
    
    /// Get the packet size in bytes
    pub fn size(&self) -> u32;
    
    /// Get the pool ID this token belongs to
    pub fn pool_id(&self) -> u32;
    
    /// Validate this token against a context
    pub fn check_token<Ctx: Context>(&self, ctx: &Ctx) -> bool;
    
    /// Consume the token to get a Payload
    pub fn consume<'ctx, Ctx: Context>(self, ctx: &'ctx Ctx) -> Payload<'ctx, Ctx>;
}
```

**Usage:**

```rust
// Receive as token (doesn't borrow socket)
let (token, _meta) = socket.recv_token()?;

// Later, consume the token
let ctx = socket.context();
let packet = token.consume(ctx);
```

### `BufferDesc` and `BufferRef`

Internal types for buffer management.

```rust
/// Buffer descriptor (index into buffer pool)
#[derive(Clone, Copy, Debug)]
pub struct BufferDesc(pub(crate) usize);

/// Buffer reference (similar to BufferDesc)
#[derive(Clone, Copy, Debug)]
pub struct BufferRef(pub(crate) usize);
```

---

## Configuration Flags

### `PcapFlags`

```rust
#[derive(Clone, Debug)]
pub struct PcapFlags {
    /// Maximum number of bytes to capture per packet
    pub snaplen: i32,
    
    /// Enable promiscuous mode
    pub promiscuous: bool,
    
    /// Read timeout in milliseconds
    pub timeout_ms: i32,
    
    /// Enable immediate delivery mode
    pub immediate: bool,
    
    /// BPF filter expression (tcpdump syntax)
    pub filter: Option<String>,
    
    /// Size of each buffer in the pool
    pub buffer_size: usize,
    
    /// Number of buffers to preallocate
    pub buffer_count: usize,
}

impl Default for PcapFlags {
    fn default() -> Self {
        Self {
            snaplen: 65535,
            promiscuous: true,
            timeout_ms: 1,
            immediate: true,
            filter: None,
            buffer_size: 2048,
            buffer_count: 32,
        }
    }
}
```

### `AfXdpFlags`

```rust
#[derive(Clone, Debug)]
pub struct AfXdpFlags {
    /// Bind flags (XDP_ZEROCOPY, XDP_COPY, etc.)
    pub bind_flags: u16,
    
    /// XDP flags (XDP_FLAGS_DRV_MODE, XDP_FLAGS_SKB_MODE, etc.)
    pub xdp_flags: u32,
    
    /// Number of UMEM frames
    pub num_frames: u32,
    
    /// Size of each UMEM frame
    pub frame_size: u32,
    
    /// TX ring size (must be power of 2)
    pub tx_size: u32,
    
    /// RX ring size (must be power of 2)
    pub rx_size: u32,
}
```

### `NetmapFlags`

```rust
#[derive(Clone, Debug)]
pub struct NetmapFlags {
    /// Number of extra buffers to allocate
    pub extra_buf: u32,
}
```

### `DpdkFlags`

```rust
#[derive(Clone, Debug)]
pub struct DpdkFlags {
    /// Number of mbufs in the mempool
    pub num_mbufs: u32,
    
    /// Per-core mbuf cache size
    pub mbuf_cache_size: u32,
    
    /// Default buffer size for mbufs
    pub mbuf_default_buf_size: u16,
}
```

---

## Error Handling

### `Error` Enum

```rust
#[derive(Error, Debug)]
pub enum Error {
    /// No packet available for receive
    #[error("Can't receive packet")]
    NoPacket,
    
    /// Memory allocation failed
    #[error("Can't allocate memory")]
    NoMemory,
    
    /// Netmap-specific error (Linux/FreeBSD only)
    #[cfg(all(any(target_os = "linux", target_os = "freebsd"), feature = "netmap"))]
    #[error("{0}")]
    Netmap(#[from] netmap_rs::errors::Error),
    
    /// Packet size exceeds buffer capacity
    #[error("Too big packet: {0}")]
    TooBigPacket(usize),
    
    /// Generic I/O error
    #[error("{0}")]
    Generic(#[from] io::Error),
    
    /// Pcap-specific error
    #[error("{0}")]
    Pcap(#[from] pcap::Error),
    
    /// Unknown error
    #[error("unknown error")]
    Unknown,
}
```

### `Result` Type

```rust
pub type Result<T> = std::result::Result<T, crate::errors::Error>;
```

---

## Metadata Types

### `MetadataType` Enum

```rust
pub enum MetadataType {
    /// Netmap metadata
    #[cfg(all(any(target_os = "linux", target_os = "freebsd"), feature = "netmap"))]
    Netmap(netmap::Meta),
    
    /// AF_XDP metadata
    #[cfg(all(target_os = "linux", feature = "af_xdp"))]
    AfXdp(af_xdp::Meta),
    
    /// DPDK metadata
    #[cfg(all(target_os = "linux", feature = "dpdk"))]
    Dpdk(dpdk::Meta),
    
    /// Pcap metadata
    Pcap(pcap::Meta),
}
```

### `pcap::Meta`

```rust
pub struct Meta {
    /// Packet arrival timestamp
    pub timestamp: libc::timeval,
    
    /// Original packet length (before truncation)
    pub len: u32,
    
    /// Captured length
    pub caplen: u32,
}
```

### `netmap::Meta`

```rust
pub struct Meta {
    /// Packet arrival timestamp
    pub timestamp: TimeVal,
}
```

---

## Advanced Features

### Channel API

For multi-threaded applications with producer/consumer patterns:

```rust
use nethuns_rs::api::{Socket, NethunsPusher, NethunsPopper};

const BATCH_SIZE: usize = 32;

// Create socket with a channel
let (socket, pusher, popper) = Sock::create_with_channel::<BATCH_SIZE>(
    "eth0",
    None,
    flags,
    flume::bounded(1024),
)?;

// Producer thread: receive packets and push to channel
let producer = std::thread::spawn(move || {
    let batch = collect_batch(&socket);
    pusher.push(batch).unwrap();
});

// Consumer thread: pop from channel and process
let consumer = std::thread::spawn(move || {
    while let Some(batch) = popper.pop() {
        for packet in batch {
            process(&packet);
        }
    }
});
```

### Zero-Copy Operations

For maximum performance, work with tokens directly:

```rust
// Receive as token (no borrow of socket)
let (token, meta) = socket.recv_token()?;

// Store token for later processing
stored_tokens.push(token);

// Later, consume all tokens
for token in stored_tokens {
    let packet = token.consume(ctx);
    process(&packet);
}
```

---

## Module Structure

```
nethuns_rs
├── api           # Core traits and types
│   ├── Socket
│   ├── Context
│   ├── Payload
│   ├── Token
│   └── ...
├── errors        # Error types
├── pcap          # pcap backend (all platforms)
├── af_xdp        # AF_XDP backend (Linux, feature = "af_xdp")
├── netmap        # netmap backend (Linux/FreeBSD, feature = "netmap")
└── dpdk          # DPDK backend (Linux, feature = "dpdk")
```
