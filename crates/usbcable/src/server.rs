//! USB/IP server for the virtual cable. Windows' usbip client attaches to it
//! (`usbip attach -r 127.0.0.1 -b <busid>`) and sees a USB sound card.
//!
//! Isochronous URBs are answered at the pace of real USB frames (1 ms each),
//! so the host's audio clock runs at the nominal rate.

use std::collections::HashSet;
use std::io::{self, Write};
use std::net::{Shutdown, TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::mpsc::{self, Receiver};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use crate::device::{self, Control, State, EP_PLAY, PACKET_BYTES};
use crate::ring::Ring;
use crate::wire::{self, Command, IsoPacket, Request, RetSubmit};

/// Linux errno values the USB/IP protocol uses for URB status.
const EPIPE: i32 = -32;
const ECONNRESET: i32 = -104;

/// Writes what the host does to stderr: every control request, and a summary
/// of the isochronous traffic once a second.
pub static VERBOSE: AtomicBool = AtomicBool::new(false);

fn log(message: impl FnOnce() -> String) {
    if VERBOSE.load(Ordering::Relaxed) {
        eprintln!("usbcable: {}", message());
    }
}

static SHOWN: AtomicU32 = AtomicU32::new(0);

pub const DEFAULT_PORT: u16 = 3240;
pub const BUSID: &str = "1-1";

pub struct Server {
    pub ring: Arc<Ring>,
    listener: TcpListener,
}

impl Server {
    pub fn bind(port: u16) -> io::Result<Self> {
        Ok(Self {
            ring: Arc::new(Ring::new()),
            // Local only: nobody else on the network may attach.
            listener: TcpListener::bind(("127.0.0.1", port))?,
        })
    }

    pub fn port(&self) -> u16 {
        self.listener.local_addr().map_or(0, |a| a.port())
    }

    /// Serves connections one after the other until the listener fails.
    pub fn run(&self) -> io::Result<()> {
        for stream in self.listener.incoming() {
            let stream = stream?;
            if let Err(e) = serve(stream, &self.ring) {
                // A closed connection is the normal way to detach.
                if e.kind() != io::ErrorKind::UnexpectedEof {
                    eprintln!("usbcable: connection ended: {e}");
                }
            }
        }
        Ok(())
    }
}

struct Shared {
    writer: Mutex<TcpStream>,
    state: Mutex<State>,
    pending: Mutex<HashSet<u32>>,
    ring: Arc<Ring>,
}

impl Shared {
    fn send(&self, bytes: &[u8]) -> io::Result<()> {
        let mut w = self.writer.lock().unwrap_or_else(|e| e.into_inner());
        w.write_all(bytes)?;
        w.flush()
    }
}

struct IsoJob {
    start_frame: u32,
    seq: u32,
    input: bool,
    endpoint: u32,
    iso: Vec<IsoPacket>,
    data: Vec<u8>,
}

fn serve(mut stream: TcpStream, ring: &Arc<Ring>) -> io::Result<()> {
    stream.set_nodelay(true)?;
    let dev = device::info(BUSID);
    match wire::read_request(&mut stream)? {
        Request::DevList => {
            wire::write_all(&mut stream, &wire::encode_devlist_reply(&dev))?;
            let _ = stream.shutdown(Shutdown::Both);
            Ok(())
        }
        Request::Import { busid } if busid == BUSID => {
            wire::write_all(&mut stream, &wire::encode_import_reply(&dev))?;
            run_urbs(stream, ring)
        }
        Request::Import { .. } => wire::write_all(&mut stream, &wire::encode_import_error()),
    }
}

fn run_urbs(stream: TcpStream, ring: &Arc<Ring>) -> io::Result<()> {
    let shared = Arc::new(Shared {
        writer: Mutex::new(stream.try_clone()?),
        state: Mutex::new(State::default()),
        pending: Mutex::new(HashSet::new()),
        ring: ring.clone(),
    });
    let (play_tx, play_rx) = mpsc::channel();
    let (rec_tx, rec_rx) = mpsc::channel();
    let workers = [
        spawn_pacer(shared.clone(), play_rx),
        spawn_pacer(shared.clone(), rec_rx),
    ];

    let mut reader = stream;
    let result = loop {
        match wire::read_command(&mut reader) {
            Ok(Command::Submit {
                seq,
                input,
                endpoint,
                setup,
                iso,
                data,
                buffer_len,
                start_frame,
            }) => {
                let sent = if endpoint == 0 {
                    control(&shared, seq, input, setup, buffer_len)
                } else {
                    // The first few only: there are hundreds per second.
                    if SHOWN.fetch_add(1, Ordering::Relaxed) < 10 {
                        log(|| {
                            format!(
                                "URB {seq} endpoint {endpoint} {}: {} packets, {buffer_len} bytes",
                                if input { "in" } else { "out" },
                                iso.len()
                            )
                        });
                    }
                    shared
                        .pending
                        .lock()
                        .unwrap_or_else(|e| e.into_inner())
                        .insert(seq);
                    let job = IsoJob {
                        start_frame,
                        seq,
                        input,
                        endpoint,
                        iso,
                        data,
                    };
                    let queue = if endpoint == EP_PLAY {
                        &play_tx
                    } else {
                        &rec_tx
                    };
                    queue
                        .send(job)
                        .map_err(|_| io::Error::new(io::ErrorKind::BrokenPipe, "pacer stopped"))
                };
                if let Err(e) = sent {
                    break Err(e);
                }
            }
            Ok(Command::Unlink { seq, target }) => {
                let was_pending = shared
                    .pending
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .remove(&target);
                let status = if was_pending { ECONNRESET } else { 0 };
                if let Err(e) = shared.send(&wire::encode_ret_unlink(seq, status)) {
                    break Err(e);
                }
            }
            Err(e) => break Err(e),
        }
    };
    // Dropping the senders ends the pacer threads.
    drop((play_tx, rec_tx));
    for worker in workers {
        let _ = worker.join();
    }
    result
}

fn control(
    shared: &Shared,
    seq: u32,
    input: bool,
    setup: [u8; 8],
    buffer_len: u32,
) -> io::Result<()> {
    let outcome = shared
        .state
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .control(setup);
    log(|| {
        format!(
            "control {:02x} {:02x} value {:#06x} index {} length {} -> {}",
            setup[0],
            setup[1],
            u16::from_le_bytes([setup[2], setup[3]]),
            u16::from_le_bytes([setup[4], setup[5]]),
            u16::from_le_bytes([setup[6], setup[7]]),
            if matches!(outcome, Control::Done(_)) {
                "ok"
            } else {
                "stall"
            },
        )
    });
    let reply = match outcome {
        Control::Done(mut data) => {
            data.truncate(buffer_len as usize);
            let actual = if input { data.len() as u32 } else { buffer_len };
            wire::encode_ret_submit(&RetSubmit {
                seq,
                input,
                endpoint: 0,
                status: 0,
                actual,
                start_frame: 0,
                data: &data,
                iso: None,
            })
        }
        Control::Stall => wire::encode_ret_submit(&RetSubmit {
            seq,
            input,
            endpoint: 0,
            status: EPIPE,
            actual: 0,
            start_frame: 0,
            data: &[],
            iso: None,
        }),
    };
    shared.send(&reply)
}

/// Completes the URBs of one endpoint, each when its frames are over.
fn spawn_pacer(shared: Arc<Shared>, jobs: Receiver<IsoJob>) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        // The moment up to which this endpoint's frames are already spoken for.
        let mut clock: Option<Instant> = None;
        let (mut urbs, mut packets, mut late_ms) = (0u32, 0u32, 0u128);
        let mut report = Instant::now();
        for job in jobs {
            urbs += 1;
            packets += job.iso.len() as u32;
            if urbs == 1 {
                log(|| {
                    format!(
                        "first URB on endpoint {}: {} packets of {:?} bytes, start frame {}",
                        job.endpoint,
                        job.iso.len(),
                        job.iso.first().map(|p| p.length),
                        job.start_frame
                    )
                });
            }
            let frames = job.iso.len().max(1) as u64;
            let now = Instant::now();
            // Keep to one absolute schedule so that late wake-ups are made up
            // for; only a long stall (the host paused) starts a new one.
            let start = clock
                .filter(|&c| now.duration_since(c.min(now)) < Duration::from_millis(50))
                .unwrap_or(now);
            let due = start + Duration::from_millis(frames);
            clock = Some(due);
            if let Some(wait) = due.checked_duration_since(Instant::now()) {
                thread::sleep(wait);
            } else {
                late_ms += Instant::now().duration_since(due).as_millis();
            }
            if report.elapsed() >= Duration::from_secs(1) {
                log(|| {
                    format!("endpoint {}: {urbs} URBs, {packets} packets, {late_ms} ms late in the last second", job.endpoint)
                });
                (urbs, packets, late_ms) = (0, 0, 0);
                report = Instant::now();
            }
            if complete(&shared, job).is_err() {
                return;
            }
        }
    })
}

fn complete(shared: &Shared, job: IsoJob) -> io::Result<()> {
    // Unlinked while waiting: the host already got its answer.
    if !shared
        .pending
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .remove(&job.seq)
    {
        return Ok(());
    }
    let (playing, recording) = {
        let s = shared.state.lock().unwrap_or_else(|e| e.into_inner());
        (s.playing(), s.recording())
    };
    let mut iso = job.iso;
    let mut out = Vec::new();
    if job.input {
        // Microphone: one packet of the wire per frame.
        for p in &mut iso {
            let len = (p.length as usize).min(PACKET_BYTES);
            let mut chunk = vec![0u8; len];
            if recording {
                shared.ring.read(&mut chunk);
            }
            out.extend(&chunk);
            // The offset stays where the host put the packet: the data is
            // sent packed and the host spreads it out again with it.
            p.actual = len as u32;
            p.status = 0;
        }
    } else {
        // Speaker: hand the samples to the wire.
        for p in &mut iso {
            let start = (p.offset as usize).min(job.data.len());
            let end = (start + p.length as usize).min(job.data.len());
            if playing {
                shared.ring.write(&job.data[start..end]);
            }
            p.actual = p.length;
            p.status = 0;
        }
    }
    let actual = if job.input {
        out.len() as u32
    } else {
        job.data.len() as u32
    };
    let reply = wire::encode_ret_submit(&RetSubmit {
        seq: job.seq,
        input: job.input,
        endpoint: job.endpoint,
        status: 0,
        actual,
        start_frame: job.start_frame,
        data: &out,
        iso: Some(&iso),
    });
    shared.send(&reply)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::EP_REC;
    use std::io::Read;

    fn cmd_submit(
        seq: u32,
        input: bool,
        ep: u32,
        setup: [u8; 8],
        len: u32,
        packets: u32,
    ) -> Vec<u8> {
        let mut b = Vec::new();
        b.extend(wire::CMD_SUBMIT.to_be_bytes());
        b.extend(seq.to_be_bytes());
        b.extend(0u32.to_be_bytes());
        b.extend(u32::from(input).to_be_bytes());
        b.extend(ep.to_be_bytes());
        b.extend(0u32.to_be_bytes());
        b.extend(len.to_be_bytes());
        b.extend(0u32.to_be_bytes());
        b.extend(packets.to_be_bytes());
        b.extend(1u32.to_be_bytes());
        b.extend(setup);
        b
    }

    fn iso_descs(count: u32, length: u32) -> Vec<u8> {
        let mut b = Vec::new();
        for i in 0..count {
            b.extend((i * length).to_be_bytes());
            b.extend(length.to_be_bytes());
            b.extend(0u32.to_be_bytes());
            b.extend(0u32.to_be_bytes());
        }
        b
    }

    fn connect(server: &Server) -> TcpStream {
        let mut s = TcpStream::connect(("127.0.0.1", server.port())).unwrap();
        let mut req = vec![0x01, 0x11, 0x80, 0x03, 0, 0, 0, 0];
        let mut busid = [0u8; 32];
        busid[..3].copy_from_slice(b"1-1");
        req.extend(busid);
        s.write_all(&req).unwrap();
        let mut reply = [0u8; 8 + 312];
        s.read_exact(&mut reply).unwrap();
        assert_eq!(&reply[2..4], &[0x00, 0x03]);
        assert_eq!(&reply[4..8], &[0, 0, 0, 0]);
        s
    }

    fn start() -> Arc<Server> {
        let server = Arc::new(Server::bind(0).unwrap());
        let runner = server.clone();
        thread::spawn(move || {
            let _ = runner.run();
        });
        server
    }

    fn read_reply_header(s: &mut TcpStream) -> [u8; 48] {
        let mut h = [0u8; 48];
        s.read_exact(&mut h).unwrap();
        h
    }

    fn status(h: &[u8; 48]) -> i32 {
        i32::from_be_bytes(h[20..24].try_into().unwrap())
    }

    #[test]
    fn host_enumerates_and_audio_goes_through_the_cable() {
        let server = start();
        let mut s = connect(&server);

        // GET_DESCRIPTOR(device).
        let setup = [0x80, 6, 0, 1, 0, 0, 18, 0];
        s.write_all(&cmd_submit(1, true, 0, setup, 18, wire::NO_PACKETS))
            .unwrap();
        let h = read_reply_header(&mut s);
        assert_eq!(
            u32::from_be_bytes(h[0..4].try_into().unwrap()),
            wire::RET_SUBMIT
        );
        let mut desc = [0u8; 18];
        s.read_exact(&mut desc).unwrap();
        assert_eq!(desc[0], 18);
        assert_eq!(desc[1], 1);

        // SET_CONFIGURATION(1), then start both streams.
        for (seq, setup) in [
            (2, [0x00, 9, 1, 0, 0, 0, 0, 0]),
            (3, [0x01, 11, 1, 0, 1, 0, 0, 0]),
            (4, [0x01, 11, 1, 0, 2, 0, 0, 0]),
        ] {
            s.write_all(&cmd_submit(seq, false, 0, setup, 0, wire::NO_PACKETS))
                .unwrap();
            let h = read_reply_header(&mut s);
            assert_eq!(status(&h), 0, "request {seq}");
        }

        // Play 2 ms of a recognisable signal, then record 4 ms.
        let signal: Vec<u8> = (0..2 * PACKET_BYTES).map(|i| (i % 251) as u8 + 1).collect();
        s.write_all(&cmd_submit(
            5,
            false,
            EP_PLAY,
            [0; 8],
            signal.len() as u32,
            2,
        ))
        .unwrap();
        s.write_all(&signal).unwrap();
        s.write_all(&iso_descs(2, PACKET_BYTES as u32)).unwrap();
        let _ = read_reply_header(&mut s);
        let mut descs = [0u8; 32];
        s.read_exact(&mut descs).unwrap();

        s.write_all(&cmd_submit(
            6,
            true,
            EP_REC,
            [0; 8],
            4 * PACKET_BYTES as u32,
            4,
        ))
        .unwrap();
        s.write_all(&iso_descs(4, PACKET_BYTES as u32)).unwrap();
        let h = read_reply_header(&mut s);
        let actual = u32::from_be_bytes(h[24..28].try_into().unwrap()) as usize;
        assert_eq!(actual, 4 * PACKET_BYTES);
        let mut audio = vec![0u8; actual];
        s.read_exact(&mut audio).unwrap();
        assert_eq!(&audio[..signal.len()], &signal[..]);
        // Beyond what was played there is silence.
        assert!(audio[signal.len()..].iter().all(|&b| b == 0));
    }

    #[test]
    fn recording_keeps_packet_offsets_and_start_frame() {
        const ASKED: usize = PACKET_BYTES + crate::device::FRAME_BYTES;
        let server = start();
        let mut s = connect(&server);
        // The host asks for one sample more per packet than the cable gives:
        // the descriptors must keep the host's offsets, the data is packed.
        let mut submit = cmd_submit(1, true, EP_REC, [0; 8], 2 * ASKED as u32, 2);
        submit[28..32].copy_from_slice(&1234u32.to_be_bytes());
        s.write_all(&submit).unwrap();
        s.write_all(&iso_descs(2, ASKED as u32)).unwrap();
        let h = read_reply_header(&mut s);
        assert_eq!(u32::from_be_bytes(h[28..32].try_into().unwrap()), 1234);
        let actual = u32::from_be_bytes(h[24..28].try_into().unwrap()) as usize;
        assert_eq!(actual, 2 * PACKET_BYTES);
        let mut rest = vec![0u8; actual + 32];
        s.read_exact(&mut rest).unwrap();
        let descs = &rest[actual..];
        let offset = |i: usize| u32::from_be_bytes(descs[i * 16..i * 16 + 4].try_into().unwrap());
        let got = |i: usize| u32::from_be_bytes(descs[i * 16 + 8..i * 16 + 12].try_into().unwrap());
        assert_eq!((offset(0), offset(1)), (0, ASKED as u32));
        assert_eq!((got(0), got(1)), (PACKET_BYTES as u32, PACKET_BYTES as u32));
    }

    #[test]
    fn iso_urbs_are_paced_like_usb_frames() {
        let server = start();
        let mut s = connect(&server);
        s.write_all(&cmd_submit(
            1,
            false,
            0,
            [0, 9, 1, 0, 0, 0, 0, 0],
            0,
            wire::NO_PACKETS,
        ))
        .unwrap();
        let _ = read_reply_header(&mut s);

        let started = Instant::now();
        let data = vec![0u8; 20 * PACKET_BYTES];
        s.write_all(&cmd_submit(
            2,
            false,
            EP_PLAY,
            [0; 8],
            data.len() as u32,
            20,
        ))
        .unwrap();
        s.write_all(&data).unwrap();
        s.write_all(&iso_descs(20, PACKET_BYTES as u32)).unwrap();
        let _ = read_reply_header(&mut s);
        // 20 frames take 20 ms.
        assert!(
            started.elapsed() >= Duration::from_millis(19),
            "answered too early"
        );
    }

    #[test]
    fn unlinked_urb_is_not_answered_twice() {
        let server = start();
        let mut s = connect(&server);
        let data = vec![0u8; 50 * PACKET_BYTES];
        s.write_all(&cmd_submit(
            2,
            false,
            EP_PLAY,
            [0; 8],
            data.len() as u32,
            50,
        ))
        .unwrap();
        s.write_all(&data).unwrap();
        s.write_all(&iso_descs(50, PACKET_BYTES as u32)).unwrap();
        let mut unlink = Vec::new();
        unlink.extend(wire::CMD_UNLINK.to_be_bytes());
        unlink.extend(3u32.to_be_bytes());
        unlink.extend([0u8; 12]);
        unlink.extend(2u32.to_be_bytes());
        unlink.extend([0u8; 24]);
        s.write_all(&unlink).unwrap();
        let h = read_reply_header(&mut s);
        assert_eq!(
            u32::from_be_bytes(h[0..4].try_into().unwrap()),
            wire::RET_UNLINK
        );
        assert_eq!(status(&h), ECONNRESET);
        s.set_read_timeout(Some(Duration::from_millis(150)))
            .unwrap();
        let mut extra = [0u8; 1];
        assert!(
            s.read(&mut extra).is_err(),
            "the cancelled URB was answered"
        );
    }

    #[test]
    fn device_list_names_the_cable() {
        let server = start();
        let mut s = TcpStream::connect(("127.0.0.1", server.port())).unwrap();
        s.write_all(&[0x01, 0x11, 0x80, 0x05, 0, 0, 0, 0]).unwrap();
        let mut all = Vec::new();
        s.read_to_end(&mut all).unwrap();
        assert_eq!(&all[2..4], &[0x00, 0x05]);
        assert!(all.windows(3).any(|w| w == b"1-1"));
    }
}
