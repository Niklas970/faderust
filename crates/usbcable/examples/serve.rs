//! Runs the virtual cable. Then, in an administrator prompt on Windows with
//! usbip-win2 installed:
//!
//!   usbip attach -r 127.0.0.1 -b 1-1
//!
//! and play something to "Faderust Cable" in the Windows sound settings: it
//! comes out of the cable's recording side.

use std::thread;
use std::time::Duration;

use faderust_usbcable::server::{self, Server, DEFAULT_PORT};

fn main() -> std::io::Result<()> {
    // Full protocol log; `--quiet` turns it off.
    server::VERBOSE.store(
        !std::env::args().any(|a| a == "--quiet"),
        std::sync::atomic::Ordering::Relaxed,
    );
    // `--cables N` offers more than one (bus ids 1-1, 1-2, …).
    let args: Vec<String> = std::env::args().collect();
    let count = args
        .iter()
        .position(|a| a == "--cables")
        .and_then(|i| args.get(i + 1))
        .and_then(|n| n.parse().ok())
        .unwrap_or(1);
    let server = Server::bind_cables(DEFAULT_PORT, count)?;
    println!(
        "USB/IP server on 127.0.0.1:{}, bus ids 1-1 …",
        server.port()
    );
    let ring = server.ring(0);
    thread::spawn(move || {
        use std::sync::atomic::Ordering::Relaxed;
        loop {
            thread::sleep(Duration::from_secs(2));
            println!(
                "written {} bytes, dropped {}, underruns {}",
                ring.written.load(Relaxed),
                ring.dropped.load(Relaxed),
                ring.underruns.load(Relaxed)
            );
        }
    });
    server.run()
}
