//! Error types for nethuns-rs.

use std::io;

use thiserror::Error;

/// Error type for nethuns-rs operations.
#[derive(Error, Debug)]
pub enum Error {
    /// No packet available for receive
    #[error("Can't receive packet")]
    NoPacket,
    
    /// Memory allocation failed
    #[error("Can't allocate memory")]
    NoMemory,
    
    /// Netmap-specific error (only on supported platforms)
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
    
    /// Unknown or unspecified error
    #[error("unknown error")]
    Unknown,
}
