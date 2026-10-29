//! USB/IP wire format (protocol 1.1.1). All integers are big-endian.

use std::io::{self, Read, Write};

pub const VERSION: u16 = 0x0111;
pub const OP_REQ_DEVLIST: u16 = 0x8005;
pub const OP_REP_DEVLIST: u16 = 0x0005;
pub const OP_REQ_IMPORT: u16 = 0x8003;
pub const OP_REP_IMPORT: u16 = 0x0003;

pub const CMD_SUBMIT: u32 = 1;
pub const CMD_UNLINK: u32 = 2;
pub const RET_SUBMIT: u32 = 3;
pub const RET_UNLINK: u32 = 4;

/// `number_of_packets` of a URB that is not isochronous.
pub const NO_PACKETS: u32 = 0xffff_ffff;

pub const BUSID_LEN: usize = 32;
const PATH_LEN: usize = 256;

fn u16_at(b: &[u8], at: usize) -> u16 {
    u16::from_be_bytes([b[at], b[at + 1]])
}

fn u32_at(b: &[u8], at: usize) -> u32 {
    u32::from_be_bytes([b[at], b[at + 1], b[at + 2], b[at + 3]])
}

/// Fixed fields of the exported device, as sent in the import reply.
#[derive(Clone, Debug)]
pub struct DeviceInfo {
    pub path: String,
    pub busid: String,
    pub busnum: u32,
    pub devnum: u32,
    /// 1 = low, 2 = full, 3 = high speed.
    pub speed: u32,
    pub vendor: u16,
    pub product: u16,
    pub bcd_device: u16,
    pub class: u8,
    pub subclass: u8,
    pub protocol: u8,
    pub configuration_value: u8,
    pub configurations: u8,
    /// (class, subclass, protocol) of each interface, for the device list.
    pub interfaces: Vec<(u8, u8, u8)>,
}

fn padded(text: &str, len: usize) -> Vec<u8> {
    let mut out = vec![0u8; len];
    let n = text.len().min(len - 1);
    out[..n].copy_from_slice(&text.as_bytes()[..n]);
    out
}

impl DeviceInfo {
    fn encode_fixed(&self) -> Vec<u8> {
        let mut out = padded(&self.path, PATH_LEN);
        out.extend(padded(&self.busid, BUSID_LEN));
        out.extend(self.busnum.to_be_bytes());
        out.extend(self.devnum.to_be_bytes());
        out.extend(self.speed.to_be_bytes());
        out.extend(self.vendor.to_be_bytes());
        out.extend(self.product.to_be_bytes());
        out.extend(self.bcd_device.to_be_bytes());
        out.extend([
            self.class,
            self.subclass,
            self.protocol,
            self.configuration_value,
            self.configurations,
            self.interfaces.len() as u8,
        ]);
        out
    }
}

fn header(code: u16, status: u32) -> Vec<u8> {
    let mut out = VERSION.to_be_bytes().to_vec();
    out.extend(code.to_be_bytes());
    out.extend(status.to_be_bytes());
    out
}

pub fn encode_devlist_reply(device: &DeviceInfo) -> Vec<u8> {
    let mut out = header(OP_REP_DEVLIST, 0);
    out.extend(1u32.to_be_bytes());
    out.extend(device.encode_fixed());
    for &(class, subclass, protocol) in &device.interfaces {
        out.extend([class, subclass, protocol, 0]);
    }
    out
}

pub fn encode_import_reply(device: &DeviceInfo) -> Vec<u8> {
    let mut out = header(OP_REP_IMPORT, 0);
    out.extend(device.encode_fixed());
    out
}

pub fn encode_import_error() -> Vec<u8> {
    header(OP_REP_IMPORT, 1)
}

/// The management request that opens a connection.
#[derive(Debug, PartialEq, Eq)]
pub enum Request {
    DevList,
    Import { busid: String },
}

pub fn read_request(r: &mut impl Read) -> io::Result<Request> {
    let mut head = [0u8; 8];
    r.read_exact(&mut head)?;
    match u16_at(&head, 2) {
        OP_REQ_DEVLIST => Ok(Request::DevList),
        OP_REQ_IMPORT => {
            let mut busid = [0u8; BUSID_LEN];
            r.read_exact(&mut busid)?;
            let end = busid.iter().position(|&b| b == 0).unwrap_or(BUSID_LEN);
            Ok(Request::Import {
                busid: String::from_utf8_lossy(&busid[..end]).into_owned(),
            })
        }
        other => Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("unknown USB/IP request {other:#06x}"),
        )),
    }
}

/// One isochronous packet descriptor.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct IsoPacket {
    pub offset: u32,
    pub length: u32,
    pub actual: u32,
    pub status: u32,
}

#[derive(Debug)]
pub enum Command {
    Submit {
        seq: u32,
        /// true = device to host.
        input: bool,
        endpoint: u32,
        buffer_len: u32,
        setup: [u8; 8],
        iso: Vec<IsoPacket>,
        /// Data of an OUT transfer.
        data: Vec<u8>,
    },
    Unlink {
        seq: u32,
        target: u32,
    },
}

/// Reads the next URB command from an imported connection.
pub fn read_command(r: &mut impl Read) -> io::Result<Command> {
    let mut h = [0u8; 48];
    r.read_exact(&mut h)?;
    let seq = u32_at(&h, 4);
    match u32_at(&h, 0) {
        CMD_SUBMIT => {
            let input = u32_at(&h, 12) == 1;
            let endpoint = u32_at(&h, 16);
            let buffer_len = u32_at(&h, 24);
            let packets = u32_at(&h, 32);
            let mut setup = [0u8; 8];
            setup.copy_from_slice(&h[40..48]);
            // Guard against a bogus length from a broken client.
            if buffer_len > 1 << 20 || (packets != NO_PACKETS && packets > 4096) {
                return Err(io::Error::new(io::ErrorKind::InvalidData, "URB too large"));
            }
            let mut data = vec![0u8; if input { 0 } else { buffer_len as usize }];
            r.read_exact(&mut data)?;
            let mut iso = Vec::new();
            if packets != NO_PACKETS {
                for _ in 0..packets {
                    let mut p = [0u8; 16];
                    r.read_exact(&mut p)?;
                    iso.push(IsoPacket {
                        offset: u32_at(&p, 0),
                        length: u32_at(&p, 4),
                        actual: u32_at(&p, 8),
                        status: u32_at(&p, 12),
                    });
                }
            }
            Ok(Command::Submit {
                seq,
                input,
                endpoint,
                buffer_len,
                setup,
                iso,
                data,
            })
        }
        CMD_UNLINK => Ok(Command::Unlink {
            seq,
            target: u32_at(&h, 20),
        }),
        other => Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("unknown URB command {other}"),
        )),
    }
}

fn ret_header(command: u32, seq: u32, input: bool, endpoint: u32) -> Vec<u8> {
    let mut out = command.to_be_bytes().to_vec();
    out.extend(seq.to_be_bytes());
    out.extend(0u32.to_be_bytes()); // devid, unused in replies
    out.extend(u32::from(input).to_be_bytes());
    out.extend(endpoint.to_be_bytes());
    out
}

/// Reply to a submitted URB. `data` is only sent for transfers into the host.
pub struct RetSubmit<'a> {
    pub seq: u32,
    pub input: bool,
    pub endpoint: u32,
    pub status: i32,
    pub actual: u32,
    pub data: &'a [u8],
    /// Packet descriptors of an isochronous transfer; `None` for the others.
    pub iso: Option<&'a [IsoPacket]>,
}

pub fn encode_ret_submit(r: &RetSubmit) -> Vec<u8> {
    let mut out = ret_header(RET_SUBMIT, r.seq, r.input, r.endpoint);
    out.extend(r.status.to_be_bytes());
    out.extend(r.actual.to_be_bytes());
    out.extend(0u32.to_be_bytes()); // start_frame
    out.extend(r.iso.map_or(NO_PACKETS, |p| p.len() as u32).to_be_bytes());
    out.extend(0u32.to_be_bytes()); // error_count
    out.extend([0u8; 8]);
    if r.input {
        out.extend(r.data);
    }
    for p in r.iso.unwrap_or_default() {
        out.extend(p.offset.to_be_bytes());
        out.extend(p.length.to_be_bytes());
        out.extend(p.actual.to_be_bytes());
        out.extend(p.status.to_be_bytes());
    }
    out
}

pub fn encode_ret_unlink(seq: u32, status: i32) -> Vec<u8> {
    let mut out = ret_header(RET_UNLINK, seq, false, 0);
    out.extend(status.to_be_bytes());
    out.extend([0u8; 24]);
    out
}

pub fn write_all(w: &mut impl Write, bytes: &[u8]) -> io::Result<()> {
    w.write_all(bytes)?;
    w.flush()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn import_request_is_parsed() {
        let mut bytes = header(OP_REQ_IMPORT, 0);
        bytes.extend(padded("1-1", BUSID_LEN));
        assert_eq!(
            read_request(&mut bytes.as_slice()).unwrap(),
            Request::Import {
                busid: "1-1".into()
            }
        );
    }

    #[test]
    fn unknown_request_is_an_error() {
        let bytes = header(0x1234, 0);
        assert!(read_request(&mut bytes.as_slice()).is_err());
    }

    #[test]
    fn submit_with_iso_packets_is_parsed() {
        let mut bytes = Vec::new();
        bytes.extend(CMD_SUBMIT.to_be_bytes());
        bytes.extend(7u32.to_be_bytes());
        bytes.extend(0u32.to_be_bytes());
        bytes.extend(0u32.to_be_bytes()); // out
        bytes.extend(1u32.to_be_bytes());
        bytes.extend(0u32.to_be_bytes()); // flags
        bytes.extend(8u32.to_be_bytes()); // length
        bytes.extend(0u32.to_be_bytes());
        bytes.extend(2u32.to_be_bytes()); // packets
        bytes.extend(1u32.to_be_bytes());
        bytes.extend([0u8; 8]);
        bytes.extend([1, 2, 3, 4, 5, 6, 7, 8]);
        for (offset, length) in [(0u32, 4u32), (4, 4)] {
            bytes.extend(offset.to_be_bytes());
            bytes.extend(length.to_be_bytes());
            bytes.extend(0u32.to_be_bytes());
            bytes.extend(0u32.to_be_bytes());
        }
        let Command::Submit {
            seq,
            input,
            endpoint,
            data,
            iso,
            ..
        } = read_command(&mut bytes.as_slice()).unwrap()
        else {
            panic!("not a submit");
        };
        assert_eq!((seq, input, endpoint), (7, false, 1));
        assert_eq!(data, [1, 2, 3, 4, 5, 6, 7, 8]);
        assert_eq!(iso.len(), 2);
        assert_eq!(iso[1].offset, 4);
    }

    #[test]
    fn oversized_urb_is_refused() {
        let mut bytes = Vec::new();
        bytes.extend(CMD_SUBMIT.to_be_bytes());
        bytes.extend(1u32.to_be_bytes());
        bytes.extend(0u32.to_be_bytes());
        bytes.extend(0u32.to_be_bytes());
        bytes.extend(1u32.to_be_bytes());
        bytes.extend(0u32.to_be_bytes());
        bytes.extend(u32::MAX.to_be_bytes());
        bytes.extend([0u8; 20]);
        assert!(read_command(&mut bytes.as_slice()).is_err());
    }

    #[test]
    fn replies_have_the_documented_size() {
        assert_eq!(encode_ret_unlink(1, 0).len(), 48);
        let empty = RetSubmit {
            seq: 1,
            input: false,
            endpoint: 0,
            status: 0,
            actual: 0,
            data: &[],
            iso: None,
        };
        assert_eq!(encode_ret_submit(&empty).len(), 48);
    }
}
