//! Packet forwarder example - forwards packets between interfaces
//!
//! This example demonstrates using nethuns-rs to forward packets
//! from one network interface to another.
//!
//! # Usage
//!
//! On macOS (pcap only):
//! ```bash
//! sudo cargo run --example forward -- en0 en1 pcap
//! ```
//!
//! On Linux with netmap:
//! ```bash
//! sudo cargo run --example forward --features netmap -- eth0 eth1 netmap
//! ```

use anyhow::Result;
use clap::{Parser, Subcommand};
use nethuns_rs::{api::Socket, pcap};
use std::sync::{
    atomic::{AtomicBool, AtomicU64, Ordering},
    Arc,
};
use std::thread;
use std::time::Duration;

// Conditional imports for platform-specific backends
#[cfg(all(target_os = "linux", feature = "af_xdp"))]
use nethuns_rs::af_xdp;
#[cfg(all(any(target_os = "linux", target_os = "freebsd"), feature = "netmap"))]
use nethuns_rs::netmap;

/// Command-line arguments.
#[derive(Parser, Debug)]
#[clap(author, version, about)]
struct Args {
    /// Input interface name.
    in_if: String,

    /// Output interface name.
    out_if: String,

    /// Queue number (optional)
    #[clap(short, long)]
    queue: Option<usize>,

    /// Enable verbose mode (log errors)
    #[clap(short, long)]
    verbose: bool,

    /// Choose the network framework.
    #[clap(subcommand)]
    framework: Framework,
}

#[derive(Subcommand, Debug, Clone)]
enum Framework {
    /// Use netmap framework (Linux/FreeBSD only)
    #[cfg(all(any(target_os = "linux", target_os = "freebsd"), feature = "netmap"))]
    Netmap(NetmapArgs),
    /// Use AF_XDP framework (Linux only)
    #[cfg(all(target_os = "linux", feature = "af_xdp"))]
    AfXdp(AfXdpArgs),
    /// Use pcap (available on all platforms)
    Pcap(PcapArgs),
}

/// Netmap-specific arguments.
#[cfg(all(any(target_os = "linux", target_os = "freebsd"), feature = "netmap"))]
#[derive(Parser, Debug, Clone)]
struct NetmapArgs {
    /// Extra buffer size for netmap.
    #[clap(long, default_value_t = 1024)]
    extra_buf: u32,
    #[clap(long, default_value_t = 256)]
    consumer_buffer_size: usize,
    #[clap(long, default_value_t = 256)]
    producer_buffer_size: usize,
}

/// AF_XDP-specific arguments.
#[cfg(all(target_os = "linux", feature = "af_xdp"))]
#[derive(Parser, Debug, Clone)]
struct AfXdpArgs {
    /// Bind flags for AF_XDP.
    #[clap(long, default_value_t = 0)]
    bind_flags: u16,
    /// XDP flags for AF_XDP.
    #[clap(long, default_value_t = 0)]
    xdp_flags: u32,
}

/// Pcap-specific arguments.
#[derive(Parser, Debug, Clone)]
struct PcapArgs {
    /// Snaplen passed to libpcap.
    #[clap(long, default_value_t = 65535)]
    snaplen: i32,
    /// Promiscuous mode.
    #[clap(long, default_value_t = true)]
    promiscuous: bool,
    /// Read timeout in milliseconds.
    #[clap(long, default_value_t = 1)]
    timeout_ms: i32,
}

pub fn main() -> Result<()> {
    // Parse command-line arguments.
    let args = Args::parse();

    // Set up a termination flag triggered on Ctrl-C.
    let term = Arc::new(AtomicBool::new(false));
    {
        let term = term.clone();
        ctrlc::set_handler(move || {
            term.store(true, Ordering::SeqCst);
        })
        .expect("Error setting Ctrl-C handler");
    }

    // Choose the proper framework and run the forwarder.
    match args.framework.clone() {
        #[cfg(all(any(target_os = "linux", target_os = "freebsd"), feature = "netmap"))]
        Framework::Netmap(netmap_args) => {
            let flags = netmap::NetmapFlags {
                extra_buf: netmap_args.extra_buf,
            };
            run_forwarder::<netmap::Sock>(flags, &args, term, args.verbose)
        }
        #[cfg(all(target_os = "linux", feature = "af_xdp"))]
        Framework::AfXdp(af_xdp_args) => {
            let flags = af_xdp::AfXdpFlags {
                bind_flags: af_xdp_args.bind_flags,
                xdp_flags: af_xdp_args.xdp_flags,
                num_frames: 4096,
                frame_size: 2048,
                tx_size: 2048,
                rx_size: 2048,
            };
            run_forwarder::<af_xdp::Sock>(flags, &args, term, args.verbose)
        }
        Framework::Pcap(pcap_args) => {
            let flags = pcap::PcapFlags {
                snaplen: pcap_args.snaplen,
                promiscuous: pcap_args.promiscuous,
                timeout_ms: pcap_args.timeout_ms,
                immediate: true,
                filter: None,
                buffer_size: 2048,
                buffer_count: 32,
            };
            run_forwarder::<pcap::Sock>(flags, &args, term, args.verbose)
        }
    }
}

/// The main packet-forwarding routine.
///
/// This function creates an input and an output socket, spawns a meter thread,
/// then enters a loop where it receives a packet on the input interface, and forwards it
/// to the output interface using a retry loop.
fn run_forwarder<Sock>(
    flags: Sock::Flags,
    args: &Args,
    term: Arc<AtomicBool>,
    verbose: bool,
) -> Result<()>
where
    Sock: Socket + 'static,
{
    println!("Starting packet forwarder:");
    println!("  Input interface: {}", args.in_if);
    println!("  Output interface: {}", args.out_if);

    // Create the input and output sockets using the selected framework.
    let in_socket = Sock::create(&args.in_if, args.queue, flags.clone())?;
    let out_socket = Sock::create(&args.out_if, args.queue, flags)?;

    // Atomic counters for received and forwarded packets.
    let total_rcv = Arc::new(AtomicU64::new(0));
    let total_fwd = Arc::new(AtomicU64::new(0));

    // Spawn a meter thread that prints packet rates every second.
    {
        let total_rcv = total_rcv.clone();
        let total_fwd = total_fwd.clone();
        let term_meter = term.clone();
        thread::spawn(move || {
            let mut prev_rcv = 0;
            let mut prev_fwd = 0;
            while !term_meter.load(Ordering::SeqCst) {
                thread::sleep(Duration::from_secs(1));
                let curr_rcv = total_rcv.load(Ordering::SeqCst);
                let curr_fwd = total_fwd.load(Ordering::SeqCst);
                println!(
                    "pkt/sec: {} fwd/sec: {}",
                    curr_rcv.saturating_sub(prev_rcv),
                    curr_fwd.saturating_sub(prev_fwd)
                );
                prev_rcv = curr_rcv;
                prev_fwd = curr_fwd;
            }
        });
    }

    // Forwarding loop.
    while !term.load(Ordering::SeqCst) {
        // Receive a packet from the input socket.
        let (packet, _meta) = match in_socket.recv() {
            Ok((p, m)) => (p, m),
            Err(e) => {
                if verbose {
                    eprintln!("recv error: {}", e);
                }
                continue;
            }
        };
        total_rcv.fetch_add(1, Ordering::SeqCst);

        // Forward the packet with a retry loop.
        loop {
            match out_socket.send(&packet) {
                Ok(_) => break,
                Err(e) => {
                    if verbose {
                        eprintln!("send error: {}, flushing...", e);
                    }
                    out_socket.flush();
                }
            }
        }
        total_fwd.fetch_add(1, Ordering::SeqCst);

        // Release the packet from the input socket.
        // (Assumes that meta contains a packet identifier for release.)
    }

    Ok(())
}
