//! # Nethuns-rs
//!
//! A unified API for fast and portable network programming in Rust.
//!
//! Nethuns-rs provides a unified abstraction layer that allows programmers to
//! implement network applications regardless of the underlying network I/O
//! framework. Applications using nethuns-rs only need to be recompiled to run
//! on top of a different engine.
//!
//! ## Supported Backends
//!
//! - **pcap**: Available on all platforms (Linux, macOS, BSD, Windows)
//! - **AF_XDP**: Linux-only, requires the `af_xdp` feature
//! - **netmap**: Linux/FreeBSD, requires the `netmap` feature
//! - **DPDK**: Linux-only, requires the `dpdk` feature
//!
//! ## Usage
//!
//! ```rust,no_run
//! use nethuns_rs::api::Socket;
//! use nethuns_rs::pcap::{Sock, PcapFlags};
//!
//! let flags = PcapFlags::default();
//! let socket = Sock::create("eth0", None, flags).unwrap();
//! ```

// Always available modules
pub mod api;
pub mod errors;
pub mod pcap;
pub mod unsafe_refcell;

// Linux-only modules (controlled by features)
#[cfg(all(target_os = "linux", feature = "af_xdp"))]
pub mod af_xdp;

#[cfg(all(target_os = "linux", feature = "dpdk"))]
pub mod dpdk;

#[cfg(all(any(target_os = "linux", target_os = "freebsd"), feature = "netmap"))]
pub mod netmap;
