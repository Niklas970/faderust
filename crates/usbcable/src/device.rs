//! The virtual device: a USB Audio Class 1.0 sound card with one stereo
//! speaker (endpoint 1 OUT) and one stereo microphone (endpoint 2 IN), 48 kHz,
//! 16 bit. What is played into the speaker comes out of the microphone, like
//! a cable.

use crate::wire::DeviceInfo;

pub const RATE: u32 = 48_000;
pub const FRAME_BYTES: usize = 4; // 2 channels * 16 bit
/// Bytes of one 1 ms USB frame.
pub const PACKET_BYTES: usize = RATE as usize / 1000 * FRAME_BYTES;
/// One more sample than average, so a rate that drifts upward still fits.
const MAX_PACKET: u16 = (PACKET_BYTES + FRAME_BYTES) as u16;

pub const EP_PLAY: u32 = 1;
pub const EP_REC: u32 = 2;

const VENDOR: u16 = 0x1209; // pid.codes test range
const PRODUCT: u16 = 0x0001;

fn string_table(index: u8) -> Option<&'static str> {
    match index {
        1 => Some("Faderust"),
        2 => Some("Faderust Cable"),
        3 => Some("0001"),
        _ => None,
    }
}

pub fn device_descriptor() -> Vec<u8> {
    // USB 1.1: the stream descriptors are written for full speed.
    let mut d = vec![18, 1, 0x10, 0x01, 0, 0, 0, 64];
    d.extend(VENDOR.to_le_bytes());
    d.extend(PRODUCT.to_le_bytes());
    d.extend([0x00, 0x01, 1, 2, 3, 1]);
    d
}

fn streaming_interface(number: u8, terminal: u8, endpoint: u8, attributes: u8) -> Vec<u8> {
    let mut d = Vec::new();
    // Alternate setting 0: no endpoints, no bandwidth used.
    d.extend([9, 4, number, 0, 0, 1, 2, 0, 0]);
    // Alternate setting 1: the stream.
    d.extend([9, 4, number, 1, 1, 1, 2, 0, 0]);
    // AS general: terminal link, delay 1, PCM.
    d.extend([7, 0x24, 1, terminal, 1, 1, 0]);
    // Format type I: 2 channels, 2 bytes, 16 bit, one fixed rate.
    d.extend([11, 0x24, 2, 1, 2, 2, 16, 1]);
    d.extend(&RATE.to_le_bytes()[..3]);
    // Isochronous endpoint, 1 ms interval.
    d.extend([9, 5, endpoint, attributes]);
    d.extend(MAX_PACKET.to_le_bytes());
    d.extend([1, 0, 0]);
    // Class-specific endpoint: no controls.
    d.extend([7, 0x25, 1, 0, 0, 0, 0]);
    d
}

pub fn configuration_descriptor() -> Vec<u8> {
    let mut body = Vec::new();
    // Interface 0: audio control.
    body.extend([9, 4, 0, 0, 0, 1, 1, 0, 0]);
    let mut control = vec![
        // Input terminal 1: USB stream from the host.
        12, 0x24, 2, 1, 0x01, 0x01, 0, 2, 3, 0, 0, 0, // Output terminal 2: speaker.
        9, 0x24, 3, 2, 0x01, 0x03, 0, 1, 0, // Input terminal 3: microphone.
        12, 0x24, 2, 3, 0x01, 0x02, 0, 2, 3, 0, 0, 0,
        // Output terminal 4: USB stream to the host.
        9, 0x24, 3, 4, 0x01, 0x01, 0, 3, 0,
    ];
    let total = (8 + 2 + control.len()) as u16;
    body.extend([10, 0x24, 1, 0x00, 0x01]);
    body.extend(total.to_le_bytes());
    body.extend([2, 1, 2]);
    body.append(&mut control);
    // Playback adapts to the host's rate; the microphone side is asynchronous.
    body.extend(streaming_interface(1, 1, 0x01, 0x09));
    body.extend(streaming_interface(2, 4, 0x82, 0x05));

    let mut d = vec![9, 2];
    d.extend(((9 + body.len()) as u16).to_le_bytes());
    d.extend([3, 1, 0, 0x80, 50]);
    d.extend(body);
    d
}

fn string_descriptor(index: u8) -> Option<Vec<u8>> {
    if index == 0 {
        return Some(vec![4, 3, 0x09, 0x04]);
    }
    let text = string_table(index)?;
    let units: Vec<u8> = text.encode_utf16().flat_map(u16::to_le_bytes).collect();
    let mut d = vec![(2 + units.len()) as u8, 3];
    d.extend(units);
    Some(d)
}

pub fn info(busid: &str) -> DeviceInfo {
    DeviceInfo {
        path: format!("/faderust/usbcable/{busid}"),
        busid: busid.to_string(),
        busnum: 1,
        devnum: 2,
        // Full speed: 1 ms frames, as the endpoint intervals assume.
        speed: 2,
        vendor: VENDOR,
        product: PRODUCT,
        bcd_device: 0x0100,
        class: 0,
        subclass: 0,
        protocol: 0,
        configuration_value: 1,
        configurations: 1,
        interfaces: vec![(1, 1, 0), (1, 2, 0), (1, 2, 0)],
    }
}

/// Outcome of a control transfer on endpoint 0.
pub enum Control {
    /// Data to return (empty for requests without a data stage).
    Done(Vec<u8>),
    /// The device does not support the request.
    Stall,
}

/// State a host can change: the selected configuration and the alternate
/// setting of each streaming interface.
#[derive(Default)]
pub struct State {
    pub configuration: u8,
    pub alt: [u8; 3],
}

impl State {
    pub fn playing(&self) -> bool {
        self.alt[1] == 1
    }

    pub fn recording(&self) -> bool {
        self.alt[2] == 1
    }

    /// Handles one control request. `setup` is the 8-byte setup packet.
    pub fn control(&mut self, setup: [u8; 8]) -> Control {
        let request_type = setup[0];
        let request = setup[1];
        let value = u16::from_le_bytes([setup[2], setup[3]]);
        let index = u16::from_le_bytes([setup[4], setup[5]]);
        let length = u16::from_le_bytes([setup[6], setup[7]]) as usize;
        let device_to_host = request_type & 0x80 != 0;
        let kind = (request_type >> 5) & 3;
        let truncated = |mut d: Vec<u8>| {
            d.truncate(length);
            Control::Done(d)
        };
        match (kind, request) {
            // Standard requests.
            (0, 0x06) if device_to_host => {
                let (kind, number) = ((value >> 8) as u8, (value & 0xff) as u8);
                match (kind, number) {
                    (1, 0) => truncated(device_descriptor()),
                    (2, 0) => truncated(configuration_descriptor()),
                    (3, n) => string_descriptor(n).map_or(Control::Stall, truncated),
                    _ => Control::Stall,
                }
            }
            (0, 0x00) => truncated(vec![0, 0]), // GET_STATUS
            // CLEAR_FEATURE / SET_FEATURE, mostly ENDPOINT_HALT: Windows
            // clears the halt of each audio endpoint before it streams.
            (0, 0x01 | 0x03) => Control::Done(Vec::new()),
            (0, 0x08) => truncated(vec![self.configuration]),
            (0, 0x09) => {
                self.configuration = value as u8;
                if value == 0 {
                    self.alt = [0; 3];
                }
                Control::Done(Vec::new())
            }
            (0, 0x0a) => truncated(vec![self.alt.get(index as usize).copied().unwrap_or(0)]),
            (0, 0x0b) => match self.alt.get_mut(index as usize) {
                Some(alt) if value <= 1 => {
                    *alt = value as u8;
                    Control::Done(Vec::new())
                }
                _ => Control::Stall,
            },
            // Class requests: accept the sampling frequency control of the
            // endpoints (SET_CUR / GET_CUR), nothing else.
            (1, 0x01) => Control::Done(Vec::new()),
            (1, 0x81) => truncated(RATE.to_le_bytes()[..3].to_vec()),
            _ => Control::Stall,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup(request_type: u8, request: u8, value: u16, index: u16, length: u16) -> [u8; 8] {
        let mut s = [0u8; 8];
        s[0] = request_type;
        s[1] = request;
        s[2..4].copy_from_slice(&value.to_le_bytes());
        s[4..6].copy_from_slice(&index.to_le_bytes());
        s[6..8].copy_from_slice(&length.to_le_bytes());
        s
    }

    #[test]
    fn configuration_length_field_matches() {
        let c = configuration_descriptor();
        assert_eq!(u16::from_le_bytes([c[2], c[3]]) as usize, c.len());
    }

    #[test]
    fn descriptors_are_well_formed() {
        // Every descriptor starts with its own length: walking them must end
        // exactly at the end of the configuration.
        let c = configuration_descriptor();
        let mut at = 0;
        while at < c.len() {
            assert!(c[at] >= 2, "descriptor at {at} has length {}", c[at]);
            at += c[at] as usize;
        }
        assert_eq!(at, c.len());
    }

    #[test]
    fn audio_control_header_covers_its_terminals() {
        let c = configuration_descriptor();
        // 9 (config) + 9 (AC interface) then the class-specific header.
        let header = &c[18..];
        assert_eq!(&header[..3], &[10, 0x24, 1]);
        let total = u16::from_le_bytes([header[5], header[6]]) as usize;
        assert_eq!(total, 10 + 12 + 9 + 12 + 9);
    }

    #[test]
    fn host_can_read_descriptors_and_select_streams() {
        let mut s = State::default();
        let Control::Done(d) = s.control(setup(0x80, 6, 0x0100, 0, 18)) else {
            panic!("stalled");
        };
        assert_eq!(d, device_descriptor());
        // The host asks for 9 bytes first, to learn the total length.
        let Control::Done(d) = s.control(setup(0x80, 6, 0x0200, 0, 9)) else {
            panic!("stalled");
        };
        assert_eq!(d.len(), 9);
        s.control(setup(0, 9, 1, 0, 0));
        assert_eq!(s.configuration, 1);
        s.control(setup(1, 11, 1, 1, 0));
        s.control(setup(1, 11, 1, 2, 0));
        assert!(s.playing() && s.recording());
        s.control(setup(1, 11, 0, 1, 0));
        assert!(!s.playing() && s.recording());
    }

    #[test]
    fn host_can_clear_an_endpoint_halt() {
        let mut s = State::default();
        // CLEAR_FEATURE(ENDPOINT_HALT) on the playback endpoint.
        assert!(matches!(
            s.control(setup(0x02, 1, 0, 1, 0)),
            Control::Done(d) if d.is_empty()
        ));
    }

    #[test]
    fn unknown_requests_stall() {
        let mut s = State::default();
        assert!(matches!(
            s.control(setup(0x80, 6, 0x0600, 0, 10)),
            Control::Stall
        ));
        assert!(matches!(s.control(setup(1, 11, 1, 9, 0)), Control::Stall));
    }

    #[test]
    fn string_descriptors_are_utf16() {
        let Some(d) = string_descriptor(2) else {
            panic!("missing");
        };
        assert_eq!(d[0] as usize, d.len());
        assert_eq!(d.len(), 2 + "Faderust Cable".len() * 2);
    }
}
