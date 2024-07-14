use crate::bus::Card;
use crate::mmu::Mmu;
use crate::video::Video;
use std::io::ErrorKind;
use std::io::{Read, Write};
use std::net::{IpAddr, Ipv4Addr, Shutdown, SocketAddr, TcpListener, TcpStream, ToSocketAddrs};
use std::time::Duration;

#[cfg(feature = "pcap")]
use std::fmt;

#[cfg(feature = "pcap")]
use std::fmt::{Debug, Formatter};

#[cfg(feature = "serde_support")]
use crate::marshal::{as_hex, from_hex_32k};

const U2_DEBUG: bool = false;
const LABEL: &str = "Uthernet2";

// Uthernet II registers
const U2_C0X_MASK: u8 = 0x03;
const U2_C0X_MODE_REGISTER: u8 = 0x04 & U2_C0X_MASK;
const U2_C0X_ADDRESS_HIGH: u8 = 0x05 & U2_C0X_MASK;
const U2_C0X_ADDRESS_LOW: u8 = 0x06 & U2_C0X_MASK;
const U2_C0X_DATA_PORT: u8 = 0x07 & U2_C0X_MASK;

// _W5100 common constants
const W5100_MR: usize = 0x00;
const W5100_GAR0: usize = 0x01;
const _W5100_GAR1: usize = 0x02;
const _W5100_GAR2: usize = 0x03;
const W5100_GAR3: usize = 0x04;
const W5100_SUBR0: usize = 0x05;
const _W5100_SUBR1: usize = 0x06;
const _W5100_SUBR2: usize = 0x07;
const W5100_SUBR3: usize = 0x08;
const W5100_SHAR0: usize = 0x09;
const _W5100_SHAR1: usize = 0x0a;
const _W5100_SHAR2: usize = 0x0b;
const _W5100_SHAR3: usize = 0x0c;
const _W5100_SHAR4: usize = 0x0d;
const W5100_SHAR5: usize = 0x0e;
const W5100_SIPR0: usize = 0x0f;
const W5100_SIPR1: usize = 0x10;
const W5100_SIPR2: usize = 0x11;
const W5100_SIPR3: usize = 0x12;
const _W5100_IR: usize = 0x15;
const _W5100_IMR: usize = 0x16;
const W5100_RTR0: usize = 0x17;
const W5100_RTR1: usize = 0x18;
const W5100_RCR: usize = 0x19;
const W5100_RMSR: usize = 0x1a;
const W5100_TMSR: usize = 0x1b;
const _W5100_PATR0: usize = 0x1c;
const _W5100_PATR1: usize = 0x1d;
const W5100_PTIMER: usize = 0x28;
const _W5100_PMAGIC: usize = 0x29;
const _W5100_UIPR0: usize = 0x2a;
const _W5100_UIPR1: usize = 0x2b;
const _W5100_UIPR2: usize = 0x2c;
const _W5100_UIPR3: usize = 0x2d;
const _W5100_UPORT0: usize = 0x2e;
const W5100_UPORT1: usize = 0x2f;

const W5100_MR_IND: u8 = 0x01;
const W5100_MR_AI: u8 = 0x02;
const W5100_MR_RST: u8 = 0x80;

// _W5100 socket constants
const W5100_SN_MR: usize = 0x00;
const W5100_SN_CR: usize = 0x01;
const _W5100_SN_IR: usize = 0x02;
const W5100_SN_SR: usize = 0x03;
const W5100_SN_PORT0: usize = 0x04;
const W5100_SN_PORT1: usize = 0x05;
const W5100_SN_DHAR0: usize = 0x06;
const W5100_SN_DHAR1: usize = 0x07;
const W5100_SN_DHAR2: usize = 0x08;
const W5100_SN_DHAR3: usize = 0x09;
const W5100_SN_DHAR4: usize = 0x0a;
const W5100_SN_DHAR5: usize = 0x0b;
const W5100_SN_DIPR0: usize = 0x0c;
const W5100_SN_DIPR1: usize = 0x0d;
const W5100_SN_DIPR2: usize = 0x0e;
const W5100_SN_DIPR3: usize = 0x0f;
const W5100_SN_DPORT0: usize = 0x10;
const W5100_SN_DPORT1: usize = 0x11;
const W5100_SN_MSSR0: usize = 0x12;
const W5100_SN_MSSR1: usize = 0x13;
const W5100_SN_PROTO: usize = 0x14;
const W5100_SN_TOS: usize = 0x15;
const W5100_SN_TTL: usize = 0x16;
const W5100_SN_TX_FSR0: usize = 0x20;
const W5100_SN_TX_FSR1: usize = 0x21;
const W5100_SN_TX_RD0: usize = 0x22;
const W5100_SN_TX_RD1: usize = 0x23;
const W5100_SN_TX_WR0: usize = 0x24;
const W5100_SN_TX_WR1: usize = 0x25;
const W5100_SN_RX_RSR0: usize = 0x26;
const W5100_SN_RX_RSR1: usize = 0x27;
const W5100_SN_RX_RD0: usize = 0x28;
const W5100_SN_RX_RD1: usize = 0x29;
const W5100_SN_DNS_NAME_LEN: usize = 0x2a;
const W5100_SN_DNS_NAME_BEGIN: usize = 0x2b;
const W5100_SN_DNS_NAME_END: usize = 0xff;
const W5100_SN_DNS_NAME_CPTY: usize = W5100_SN_DNS_NAME_END - W5100_SN_DNS_NAME_BEGIN;

// _W5100 socket mode register constants
const W5100_SN_MR_PROTO_MASK: u8 = 0x0f;
const _W5100_SN_MR_MF: u8 = 0x40;
const W5100_SN_MR_CLOSED: u8 = 0x00;
const W5100_SN_MR_TCP: u8 = 0x01;
const W5100_SN_MR_UDP: u8 = 0x02;
const W5100_SN_MR_IPRAW: u8 = 0x03;
const W5100_SN_MR_MACRAW: u8 = 0x04;
const _W5100_SN_MR_PPPOE: u8 = 0x05;
const W5100_SN_VIRTUAL_DNS: u8 = 0x08;
const W5100_SN_MR_TCP_DNS: u8 = W5100_SN_VIRTUAL_DNS | W5100_SN_MR_TCP;
const W5100_SN_MR_UDP_DNS: u8 = W5100_SN_VIRTUAL_DNS | W5100_SN_MR_UDP;
const W5100_SN_MR_IPRAW_DNS: u8 = W5100_SN_VIRTUAL_DNS | W5100_SN_MR_IPRAW;

// _W5100 socket status constants
const W5100_SN_SR_CLOSED: u8 = 0x00;
const _W5100_SN_SR_SOCK_ARP: u8 = 0x01;
const W5100_SN_SR_SOCK_INIT: u8 = 0x13;
const W5100_SN_SR_SOCK_LISTEN: u8 = 0x14;
const _W5100_SN_SR_SOCK_SYNSENT: u8 = 0x15;
const _W5100_SN_SR_SOCK_SYNRECV: u8 = 0x16;
const W5100_SN_SR_SOCK_ESTABLISHED: u8 = 0x17;
const _W5100_SN_SR_SOCK_FIN_WAIT: u8 = 0x18;
const _W5100_SN_SR_SOCK_CLOSING: u8 = 0x1a;
const _W5100_SN_SR_SOCK_TIME_WAIT: u8 = 0x1b;
const _W5100_SN_SR_SOCK_CLOSE_WAIT: u8 = 0x1c;
const _W5100_SN_SR_SOCK_LAST_ACK: u8 = 0x1d;
const W5100_SN_SR_SOCK_UDP: u8 = 0x22;
const W5100_SN_SR_SOCK_IPRAW: u8 = 0x32;
const W5100_SN_SR_SOCK_MACRAW: u8 = 0x42;
const _W5100_SN_SR_SOCK_PPPOE: u8 = 0x5f;

const W5100_S0_BASE: usize = 0x0400;
const W5100_S3_MAX: usize = 0x07ff;
const W5100_TX_BASE: usize = 0x4000;
const W5100_RX_BASE: usize = 0x6000;
const W5100_MEM_SIZE: usize = 0x8000;

// _W5100 socket command constants
const W5100_SN_CR_OPEN: u8 = 0x01;
const W5100_SN_CR_LISTEN: u8 = 0x02;
const W5100_SN_CR_CONNECT: u8 = 0x04;
const W5100_SN_CR_DISCONNECT: u8 = 0x08;
const W5100_SN_CR_CLOSE: u8 = 0x10;
const W5100_SN_CR_SEND: u8 = 0x20;
const W5100_SN_CR_RECV: u8 = 0x40;

macro_rules! u2_debug {
    () => {
        if U2_DEBUG {
            eprintln!()
        }
    };
    ($($arg:tt)*) => {{
        if U2_DEBUG {
            eprint!("{LABEL} - ");
            eprintln!($($arg)*);
        }
    }};
}

#[cfg(feature = "serde_support")]
use serde::{Deserialize, Serialize};

#[cfg(feature = "pcap")]
struct PcapCapture(pcap::Capture<pcap::Active>);

#[cfg(feature = "pcap")]
impl Debug for PcapCapture {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "PcapCapture BlockMode: {}", self.0.is_nonblock())
    }
}

#[derive(Debug, Default)]
enum Proto {
    #[default]
    None,
    Tcp(TcpStream),
    TcpListener(TcpListener),

    #[cfg(feature = "pcap")]
    MacRaw(PcapCapture),
}

#[derive(Debug, Default)]
#[cfg_attr(feature = "serde_support", derive(Serialize, Deserialize))]
struct Socket {
    #[cfg_attr(feature = "serde_support", serde(default), serde(skip))]
    socket_handle: Proto,

    transmit_addr: usize,
    transmit_size: usize,
    receive_addr: usize,
    receive_size: usize,
    receive_pointer: usize,
    status: u8,
}

impl Socket {
    fn clear_socket_handle(&mut self) {
        if let Proto::Tcp(stream) = &mut self.socket_handle {
            let _ = stream.shutdown(Shutdown::Both);
        }
        self.socket_handle = Proto::None;
        self.status = W5100_SN_SR_CLOSED;
    }

    fn set_socket_handle(&mut self, proto: Proto) {
        self.clear_socket_handle();
        self.socket_handle = proto
    }

    fn is_open(&self) -> bool {
        !matches!(self.socket_handle, Proto::None)
            && ((self.status == W5100_SN_SR_SOCK_ESTABLISHED)
                || (self.status == W5100_SN_SR_SOCK_UDP))
    }
}

impl Drop for Socket {
    fn drop(&mut self) {
        self.clear_socket_handle();
    }
}

#[derive(Debug)]
#[cfg_attr(feature = "serde_support", derive(Serialize, Deserialize))]
pub struct Uthernet2 {
    mode: usize,
    addr: usize,

    #[cfg_attr(
        feature = "serde_support",
        serde(serialize_with = "as_hex", deserialize_with = "from_hex_32k")
    )]
    mem: Vec<u8>,
    socket: Vec<Socket>,
    interface: Option<String>,
}

impl Default for Uthernet2 {
    fn default() -> Self {
        let mut socket = Vec::new();
        for _ in 0..4 {
            socket.push(Socket::default());
        }
        let mut instance = Uthernet2 {
            mode: 0,
            addr: 0,
            mem: vec![0; 0x8000],
            socket,
            interface: None,
        };
        instance.reset(true);
        instance
    }
}

impl Uthernet2 {
    pub fn new() -> Self {
        Uthernet2::default()
    }

    pub fn reset(&mut self, cold_reset: bool) {
        if cold_reset {
            self.addr = 0;
        }

        self.mode = 0;
        self.mem = vec![0; 0x8000];

        for socket in &mut self.socket {
            socket.clear_socket_handle();
        }

        // Initialize the 4 sockets in _W5100
        for i in 0..4 {
            self.reset_rxtx_buffers(i);
            let addr = 0x400 + (i << 8);
            self.mem[addr + W5100_SN_DHAR0] = 0xff;
            self.mem[addr + W5100_SN_DHAR1] = 0xff;
            self.mem[addr + W5100_SN_DHAR2] = 0xff;
            self.mem[addr + W5100_SN_DHAR3] = 0xff;
            self.mem[addr + W5100_SN_DHAR4] = 0xff;
            self.mem[addr + W5100_SN_DHAR5] = 0xff;
            self.mem[addr + W5100_SN_TTL] = 0x80;
        }

        self.mem[W5100_RTR0] = 0x07;
        self.mem[W5100_RTR1] = 0xd0;
        self.mem[W5100_RCR] = 0x08;

        self.set_receive_size(W5100_RMSR, 0x55);
        self.set_transmit_size(W5100_TMSR, 0x55);

        // Always use Virtual DNS. Only supports UDP/TCP-based transports protocol
        // Actual real W5100, value should be 0x28
        self.mem[W5100_PTIMER] = 0x0;
    }

    pub fn set_interface(&mut self, name: String) {
        self.interface = Some(name);
    }

    pub fn get_interface(&self) -> Option<String> {
        self.interface.clone()
    }

    fn auto_increment(&mut self) {
        // If auto increment mode is enabled, increment the address
        // Auto-increment is only available if indirect bus i/f mode is enabled
        if self.mode & (W5100_MR_IND as usize) > 0 && self.mode & (W5100_MR_AI as usize) > 0 {
            self.addr += 1;

            if self.addr == 0x6000 || self.addr == 0x8000 {
                self.addr -= 0x2000;
            }
        }
    }

    /* Documented from
     *
     * http://dserver.macgui.com/Uthernet%20II%20manual%2017%20Nov%2018.pdf
     * https://www.wiznet.io/wp-content/uploads/wiznethome/Chip/_W5100/Document/W5100_DS_V128E.pdf

    | Function                                  | Address          | Len |
    |-------------------------------------------|------------------|-----|
    | Mode Register(MR)                         | $0               | 1   |
    | Gateway Address                           | $1               | 4   |
    | Subnet Mask                               | $5               | 4   |
    | MAC Address                               | $9               | 6   |
    | Source IP Address                         | $F               | 4   |
    | Interrupt(IR)                             | $15              | 1   |
    | Interrupt Mask(IMR)                       | $16              | 1   |
    | Retry Time (RTR)                          | $17              | 2   |
    | Retry Count (RCR)                         | $19              | 1   |
    | RX Memory Size (RMSR)                     | $1A              | 1   |
    | TX Memory Size (TMSR)                     | $1B              | 1   |
    | PPP LCP Request Timer                     | $28              | 1   |
    | PPP LCP Magic number                      | $29              | 1   |
    | Unreachable IP                            | $2A              | 4   |
    | Unreachable Port                          | $2E              | 2   |
    | Socket Mode (SN_MR)                       | $400 + N * $100  | 1   |
    | Socket Cmd (SN_CR)                        | $401 + N * $100  | 1   |
    | Socket Interrupt (SN_IR)                  | $402 + N * $100  | 1   |
    | Socket Status (SN_SR)                     | $403 + N * $100  | 1   |
    | Socket Source Port (SN_PORT)              | $404 + N * $100  | 2   |
    | Socket Destination MAC (SN_DHAR)          | $406 + N * $100  | 6   |
    | Socket Destination IP Addr (SN_DIPR)      | $40C + N * $100  | 4   |
    | Socket Destination Port (SN_DPORT)        | $410 + N * $100  | 2   |
    | Socket Maximum Segment Size (SN_MSSR)     | $412 + N * $100  | 2   |
    | Socket Protocol in IP Raw Mode (SN_PROTO) | $414 + N * $100  | 1   |
    | Socket IP TOS (SN_TOS)                    | $415 + N * $100  | 1   |
    | Socket IP TTL (SN_TTL)                    | $416 + N * $100  | 1   |
    | Socket TX Free Size (SN_TX_FSR)           | $420 + N * $100  | 2   |
    | Socket TX Read Pointer (SN_TX_RD)         | $422 + N * $100  | 2   |
    | Socket TX Write Pointer (SN_TX_WR)        | $424 + N * $100  | 2   |
    | Socket RX Received Size (SN_RX_RSR)       | $426 + N * $100  | 2   |
    | Socket RX Read Pointer (SN_RX_RD)         | $428 + N * $100  | 2   |

    */
    fn read_value_at(&mut self, addr: usize) -> u8 {
        let eaddr = addr & 0x7fff;
        if eaddr == W5100_MR {
            self.mode as u8
        } else if (W5100_S0_BASE..=W5100_S3_MAX).contains(&eaddr) {
            self.read_socket_register(eaddr)
        } else if (W5100_GAR0..=W5100_UPORT1).contains(&eaddr)
            || (W5100_TX_BASE..W5100_MEM_SIZE).contains(&eaddr)
        {
            self.mem[eaddr]
        } else {
            0
        }
    }

    fn read_value(&mut self) -> u8 {
        let value = self.read_value_at(self.addr);
        self.auto_increment();
        value
    }

    fn read_socket_register(&mut self, addr: usize) -> u8 {
        let mut value = self.mem[addr];
        let unit = (addr >> 8) - 4;
        let loc = addr & 0xff;

        match loc {
            W5100_SN_TX_FSR0 => value = self.get_transmit_free_size_register(unit, 8),
            W5100_SN_TX_FSR1 => value = self.get_transmit_free_size_register(unit, 0),
            W5100_SN_RX_RSR0 => {
                self.receive_one_packet(unit);
                value = self.mem[addr];
            }
            W5100_SN_RX_RSR1 => {
                self.receive_one_packet(unit);
                value = self.mem[addr];
            }
            _ => {}
        }
        value
    }

    fn receive_one_packet(&mut self, i: usize) {
        let socket = &mut self.socket[i];
        if socket.is_open() {
            match socket.status {
                W5100_SN_SR_SOCK_ESTABLISHED => self.receive_one_packet_from_socket(i),
                W5100_SN_SR_CLOSED => {
                    u2_debug!("Received Socket #{i} reading from a closed socket")
                }
                _ => {
                    u2_debug!("Received Socket #{i} Unknown mode: 0x{:02X}", socket.status)
                }
            }
        } else if socket.status == W5100_SN_SR_SOCK_MACRAW {
            #[cfg(feature = "pcap")]
            self.receive_one_raw_packet_from_socket(i)
        }
    }

    #[cfg(feature = "pcap")]
    fn receive_one_raw_packet_from_socket(&mut self, i: usize) {
        let base_addr = self.get_base_socket_addr(i);
        let socket = &mut self.socket[i];
        if let Proto::MacRaw(pcap_capture) = &mut socket.socket_handle {
            let result = pcap_capture.0.next_packet();
            if let Ok(packet) = result {
                let buffer = Vec::from(packet.data);
                let mac = &self.mem[W5100_SHAR0..=W5100_SHAR5];
                let broadcast = [0xff, 0xff, 0xff, 0xff, 0xff, 0xff];
                let mr = self.mem[base_addr + W5100_SN_MR];
                let accept_all = mr & _W5100_SN_MR_MF == 0;

                if accept_all || *mac == buffer[0..6] || broadcast == buffer[0..6] {
                    self.write_raw_data_for_protocol(i, &buffer);
                }
            } else if let Err(error) = result {
                let mut close_flag = true;
                if error == pcap::Error::TimeoutExpired {
                    close_flag = false;
                }
                if close_flag {
                    u2_debug!(
                        "Read bytes received from peer ERROR {:?} - Closing socket",
                        error
                    );
                    self.clear_socket(i);
                }
            }
        }
    }

    fn receive_one_packet_from_socket(&mut self, i: usize) {
        let base_addr = self.get_base_socket_addr(i);
        let socket = &mut self.socket[i];
        if socket.is_open() {
            if let Proto::Tcp(stream) = &mut socket.socket_handle {
                let rsr = u16::from_be_bytes([
                    self.mem[base_addr + W5100_SN_RX_RSR0],
                    self.mem[base_addr + W5100_SN_RX_RSR1],
                ]) as usize;
                let free_available = socket.receive_size - rsr;
                if free_available > 32 {
                    let mut buffer = vec![0; free_available - 1];
                    let result = stream.read(&mut buffer);
                    if let Ok(size) = result {
                        //u2_debug!("Read bytes received from peer = 0x{size:02X}");
                        if size == 0 {
                            self.clear_socket(i);
                        } else {
                            self.write_data_for_protocol(i, &buffer[0..size]);
                        }
                    } else if let Err(error) = result {
                        //u2_debug!("Read bytes received error = {error:?}");
                        if !(matches!(error.kind(), ErrorKind::WouldBlock)) {
                            u2_debug!("Read bytes received from peer ERROR - Closing socket");
                            self.clear_socket(i);
                        }
                    }
                }
            }
        }
    }

    fn write_data_for_protocol(&mut self, i: usize, data: &[u8]) {
        let base_addr = self.get_base_socket_addr(i);
        let socket = &mut self.socket[i];
        let mut rsr = u16::from_be_bytes([
            self.mem[base_addr + W5100_SN_RX_RSR0],
            self.mem[base_addr + W5100_SN_RX_RSR1],
        ]) as usize;
        for item in data {
            self.mem[socket.receive_addr + socket.receive_pointer] = *item;
            socket.receive_pointer = (socket.receive_pointer + 1) % socket.receive_size;
            rsr += 1;
        }
        let size = u16::to_be_bytes(rsr as u16);
        self.mem[base_addr + W5100_SN_RX_RSR0] = size[0];
        self.mem[base_addr + W5100_SN_RX_RSR1] = size[1];
    }

    #[cfg(feature = "pcap")]
    fn write_raw_data_for_protocol(&mut self, i: usize, data: &[u8]) {
        let base_addr = self.get_base_socket_addr(i);
        let socket = &mut self.socket[i];
        let mut rsr = u16::from_be_bytes([
            self.mem[base_addr + W5100_SN_RX_RSR0],
            self.mem[base_addr + W5100_SN_RX_RSR1],
        ]) as usize;

        let len = (2 + data.len()) & 0xffff;
        let buffer_len = [((len & 0xff00) >> 8) as u8, (len & 0xff) as u8];

        for item in buffer_len {
            self.mem[socket.receive_addr + socket.receive_pointer] = item;
            socket.receive_pointer = (socket.receive_pointer + 1) % socket.receive_size;
            rsr += 1;
        }

        for item in data {
            self.mem[socket.receive_addr + socket.receive_pointer] = *item;
            socket.receive_pointer = (socket.receive_pointer + 1) % socket.receive_size;
            rsr += 1;
        }

        let size = u16::to_be_bytes(rsr as u16);
        self.mem[base_addr + W5100_SN_RX_RSR0] = size[0];
        self.mem[base_addr + W5100_SN_RX_RSR1] = size[1];
    }

    fn get_transmit_free_size_register(&self, i: usize, shift: usize) -> u8 {
        let socket = &self.socket[i];
        let size = socket.transmit_size;
        let present = self.get_transmit_data_size(i);
        (((size - present) >> shift) & 0xff) as u8
    }

    fn get_transmit_data_size(&self, i: usize) -> usize {
        let base_addr = self.get_base_socket_addr(i);
        let socket = &self.socket[i];
        let size = socket.transmit_size;

        let sn_tx_rd = u16::from_be_bytes([
            self.mem[base_addr + W5100_SN_TX_RD0],
            self.mem[base_addr + W5100_SN_TX_RD1],
        ]);
        let sn_tx_wr = u16::from_be_bytes([
            self.mem[base_addr + W5100_SN_TX_WR0],
            self.mem[base_addr + W5100_SN_TX_WR1],
        ]);

        let data_present = if sn_tx_rd > sn_tx_wr {
            size as u16 - sn_tx_rd + sn_tx_wr
        } else {
            sn_tx_wr - sn_tx_rd
        };

        data_present as usize
    }

    fn write_value_at(&mut self, addr: usize, value: u8) {
        let eaddr = addr & 0x7fff;
        if eaddr == W5100_MR {
            self.set_mode_register(value);
        } else if (W5100_GAR0..=W5100_UPORT1).contains(&eaddr) {
            self.write_common_register(eaddr, value);
        } else if (W5100_S0_BASE..=W5100_S3_MAX).contains(&eaddr) {
            self.write_socket_register(eaddr, value);
        } else if (W5100_TX_BASE..W5100_MEM_SIZE).contains(&eaddr) {
            self.mem[eaddr] = value;
            //u2_debug!("Write to memory addr = 0x{eaddr:04X} value = 0x{value:02X}");
        }
    }

    fn write_value(&mut self, value: u8) {
        self.write_value_at(self.addr, value);
        self.auto_increment();
    }

    fn set_mode_register(&mut self, value: u8) {
        if value & W5100_MR_RST == 0 {
            self.mode = value as usize;
        } else {
            self.reset(false);
        }
    }

    fn write_common_register(&mut self, addr: usize, value: u8) {
        // UDP/TCP mode forwarding completely ignores the Gateway Address,
        // Subnet mask Address, Source Hardware Address and Source IP Address registers
        match addr {
            W5100_TMSR => self.set_transmit_size(addr, value),
            W5100_RMSR => self.set_receive_size(addr, value),
            W5100_GAR0..=W5100_GAR3 => self.mem[addr] = value,
            W5100_SHAR0..=W5100_SHAR5 => self.mem[addr] = value,
            W5100_SIPR0..=W5100_SIPR3 => self.mem[addr] = value,
            W5100_SUBR0..=W5100_SUBR3 => self.mem[addr] = value,
            _ => {}
        };
        //u2_debug!("Write to memory addr = 0x{addr:04X} value = 0x{value:02X}");
    }

    fn set_transmit_size(&mut self, addr: usize, value: u8) {
        //u2_debug!("Set Transmit Size value = 0x{value:02X}");

        self.mem[addr] = value;

        let mut base_address = W5100_TX_BASE;
        let end = W5100_RX_BASE;
        let mut tx_size = value;

        for socket in self.socket.iter_mut() {
            socket.transmit_addr = base_address;
            let bits = tx_size & 0x3;
            tx_size >>= 2;
            let size = 1 << (10 + bits);
            base_address += size;

            if base_address >= end {
                base_address = end;
            }
            socket.transmit_size = base_address - socket.transmit_addr;

            //u2_debug!("Set Transmit Socket #{i} size addr=0x{:04X} size=0x{:04X}",
            //    socket.transmit_addr, socket.transmit_size);
        }
    }

    fn set_receive_size(&mut self, addr: usize, value: u8) {
        //u2_debug!("Set Receive Size value = 0x{value:02X}");

        self.mem[addr] = value;

        let mut base_address = W5100_RX_BASE;
        let end = W5100_MEM_SIZE;
        let mut rx_size = value;

        for socket in self.socket.iter_mut() {
            socket.receive_addr = base_address;
            let bits = rx_size & 0x3;
            rx_size >>= 2;
            let size = 1 << (10 + bits);
            base_address += size;

            if base_address >= end {
                base_address = end;
            }
            socket.receive_size = base_address - socket.receive_addr;

            //u2_debug!("Set Receive Socket #{i} size addr=0x{:04X} size=0x{:04X}",
            //    socket.receive_addr, socket.receive_size);
        }
    }

    fn write_socket_register(&mut self, addr: usize, value: u8) {
        //self.mem[addr] = value;
        let unit = (addr >> 8) - 4;
        let loc = addr & 0xff;

        if loc >= W5100_SN_DNS_NAME_LEN {
            self.mem[addr] = value;
            return;
        }

        match loc {
            W5100_SN_MR => self.set_socket_mode_register(unit, addr, value),
            W5100_SN_CR => self.set_command_register(unit, addr, value),
            W5100_SN_PORT0 | W5100_SN_PORT1 | W5100_SN_DPORT0 | W5100_SN_DPORT1 => {
                self.mem[addr] = value
            }
            W5100_SN_PROTO => self.mem[addr] = value,
            W5100_SN_TOS => self.mem[addr] = value,
            W5100_SN_TTL => self.mem[addr] = value,
            W5100_SN_DIPR0 | W5100_SN_DIPR1 | W5100_SN_DIPR2 | W5100_SN_DIPR3 => {
                self.mem[addr] = value
            }
            W5100_SN_TX_WR0 | W5100_SN_TX_WR1 | W5100_SN_RX_RD0 | W5100_SN_RX_RD1 => {
                self.mem[addr] = value
            }
            W5100_SN_MSSR0 | W5100_SN_MSSR1 => self.mem[addr] = value,
            _ => {
                //u2_debug!("Write to socket unit = {unit} addr = 0x{addr:04X} loc = 0x{loc:02X} value = 0x{value:02X}")
            }
        }
    }

    fn set_socket_mode_register(&mut self, i: usize, addr: usize, value: u8) {
        self.mem[addr] = value;
        let protocol = value & W5100_SN_MR_PROTO_MASK;

        if U2_DEBUG {
            match protocol {
                W5100_SN_MR_CLOSED => u2_debug!("Socket #{i} mode: CLOSED"),
                W5100_SN_MR_TCP | W5100_SN_MR_TCP_DNS => u2_debug!("Socket #{i} mode: TCP"),
                W5100_SN_MR_UDP | W5100_SN_MR_UDP_DNS => u2_debug!("Socket #{i} mode: UDP"),
                W5100_SN_MR_IPRAW | W5100_SN_MR_IPRAW_DNS => u2_debug!("Socket #{i} mode: IPRAW"),
                W5100_SN_MR_MACRAW => u2_debug!("Socket #{i} mode: MACRAW"),
                _ => u2_debug!("Socket #{i} mode: Unknown = {protocol:02X}"),
            }
        }
    }

    fn set_command_register(&mut self, i: usize, addr: usize, value: u8) {
        self.mem[addr] = 0;
        match value {
            W5100_SN_CR_OPEN => self.open_socket(i),
            W5100_SN_CR_LISTEN => {
                //u2_debug!("LISTEN command received on #{i}: Not supported yet");
                self.listen_socket(i);
            }

            W5100_SN_CR_CONNECT => self.connect_socket(i),
            W5100_SN_CR_CLOSE | W5100_SN_CR_DISCONNECT => self.close_socket(i),
            W5100_SN_CR_SEND => self.send_data(i),
            W5100_SN_CR_RECV => self.update_rsr(i),
            _ => u2_debug!("Unknown Command received on #{i} Command: 0x{value:02X}"),
        }
    }

    fn reset_rxtx_buffers(&mut self, i: usize) {
        let base_addr = self.get_base_socket_addr(i);
        let socket = &mut self.socket[i];
        socket.receive_pointer = 0;

        self.mem[base_addr + W5100_SN_TX_RD0] = 0x00;
        self.mem[base_addr + W5100_SN_TX_RD1] = 0x00;
        self.mem[base_addr + W5100_SN_TX_WR0] = 0x00;
        self.mem[base_addr + W5100_SN_TX_WR1] = 0x00;
        self.mem[base_addr + W5100_SN_RX_RD0] = 0x00;
        self.mem[base_addr + W5100_SN_RX_RD1] = 0x00;
        self.mem[base_addr + W5100_SN_RX_RSR0] = 0x00;
        self.mem[base_addr + W5100_SN_RX_RSR1] = 0x00;
    }

    fn get_base_socket_addr(&self, i: usize) -> usize {
        W5100_S0_BASE + (i << 8)
    }

    fn clear_socket(&mut self, i: usize) {
        let base_addr = self.get_base_socket_addr(i);
        let socket = &mut self.socket[i];
        socket.clear_socket_handle();
        self.mem[base_addr + W5100_SN_SR] = socket.status;
    }

    fn set_socket_status(&mut self, i: usize, status: u8) {
        let base_addr = self.get_base_socket_addr(i);
        let socket = &mut self.socket[i];
        socket.status = status;
        self.mem[base_addr + W5100_SN_SR] = status;
    }

    fn get_socket_status(&self, i: usize) -> u8 {
        let socket = &self.socket[i];
        socket.status
    }

    fn open_socket(&mut self, i: usize) {
        let base_addr = self.get_base_socket_addr(i);
        let mode_register = self.mem[base_addr];
        let protocol = mode_register & W5100_SN_MR_PROTO_MASK;

        u2_debug!("Open Socket on #{i} with protocol = 0x{protocol:02x}");

        // Open the socket
        match protocol {
            W5100_SN_MR_IPRAW | W5100_SN_MR_IPRAW_DNS => {
                self.set_socket_status(i, W5100_SN_SR_SOCK_IPRAW)
            }
            W5100_SN_MR_MACRAW => {
                #[cfg(feature = "pcap")]
                self.assign_interface_to_raw_protocol(i);

                self.set_socket_status(i, W5100_SN_SR_SOCK_MACRAW);
            }

            W5100_SN_MR_TCP | W5100_SN_MR_TCP_DNS => {
                self.set_socket_status(i, W5100_SN_SR_SOCK_INIT);
            }
            W5100_SN_MR_UDP | W5100_SN_MR_UDP_DNS => {
                self.set_socket_status(i, W5100_SN_SR_SOCK_UDP)
            }
            _ => {
                u2_debug!("Open Socket with unknown mode 0x{protocol:02X}")
            }
        }

        // Resolve the DNS for TCP/UDP DNS
        match protocol {
            W5100_SN_MR_TCP_DNS | W5100_SN_MR_UDP_DNS => self.resolve_dns(i),
            _ => {}
        }

        self.reset_rxtx_buffers(i);
    }

    #[cfg(feature = "pcap")]
    fn assign_interface_to_raw_protocol(&mut self, i: usize) {
        let mut device = None;
        if let Some(name) = &self.interface {
            let result = pcap::Device::list();
            device = if let Ok(devices) = result {
                devices
                    .into_iter()
                    .filter(|device| {
                        if device.name.to_lowercase().contains(&name.to_lowercase()) {
                            true
                        } else {
                            device
                                .desc
                                .as_ref()
                                .map_or(false, |s| s.to_lowercase().contains(&name.to_lowercase()))
                        }
                    })
                    .nth(0)
            } else {
                if let Err(error) = result {
                    u2_debug!("Unable to list pcap devices: {:?}", error);
                }
                None
            };
        }

        if device.is_none() {
            // No interface provided or found, try to get default capture device from pcap
            let result = pcap::Device::lookup();
            if let Ok(Some(lookup_device)) = result {
                device = Some(lookup_device)
            } else {
                if let Err(error) = result {
                    u2_debug!("Unable to lookup device: {:?}", error);
                }
            }
        }

        if let Some(device) = device {
            u2_debug!("Using device name: {} desc: {:?}", device.name, device.desc);

            let cap_active = self.get_pcap_device(&device);

            // Try to convert capture<active> to non-blocking
            if let Some(tcap_active) = cap_active {
                let nonblock_result = tcap_active.setnonblock();
                if let Ok(nonblock_cap) = nonblock_result {
                    self.socket[i].set_socket_handle(Proto::MacRaw(PcapCapture(nonblock_cap)));
                } else {
                    if let Err(error) = nonblock_result {
                        u2_debug!(
                            "Unable to set nonblock to device. Fall back to blocking mode: {:?}",
                            error
                        );
                    }

                    // Fallback to blocking with timeout of 20 ms if failed to set non-blocking
                    let cap_active = self.get_pcap_device(&device);
                    if let Some(tcap_active) = cap_active {
                        self.socket[i].set_socket_handle(Proto::MacRaw(PcapCapture(tcap_active)));
                    } else {
                        u2_debug!("Unable to get blocking device");
                    }
                }
            }
        }
    }

    #[cfg(feature = "pcap")]
    fn get_pcap_device(&self, device: &pcap::Device) -> Option<pcap::Capture<pcap::Active>> {
        let result = pcap::Capture::from_device(device.clone());
        if let Ok(cap) = result {
            let cap_result = cap.snaplen(1700).promisc(true).timeout(20).open();
            if let Ok(cap) = cap_result {
                Some(cap)
            } else {
                if let Err(error) = cap_result {
                    u2_debug!("Unable to get device: {:?}", error);
                }
                None
            }
        } else {
            if let Err(error) = result {
                u2_debug!("Unable to get device: {:?}", error);
            }
            None
        }
    }

    fn clear_socket_dest(&mut self, i: usize) {
        let base_addr = self.get_base_socket_addr(i);
        let dest = &mut self.mem[base_addr + W5100_SN_DIPR0..=base_addr + W5100_SN_DIPR3];

        // Clear the destination
        for item in dest[0..4].iter_mut() {
            *item = 0
        }
    }

    fn resolve_dns(&mut self, i: usize) {
        let base_addr = self.get_base_socket_addr(i);
        self.clear_socket_dest(i);
        let length = self.mem[base_addr + W5100_SN_DNS_NAME_LEN] as usize;

        if length <= W5100_SN_DNS_NAME_CPTY {
            let name = String::from_utf8_lossy(
                &self.mem[base_addr + W5100_SN_DNS_NAME_BEGIN
                    ..base_addr + W5100_SN_DNS_NAME_BEGIN + length],
            );

            u2_debug!("Resolving DNS name={name} ...");

            let port = u16::from_be_bytes([
                self.mem[base_addr + W5100_SN_DPORT0],
                self.mem[base_addr + W5100_SN_DPORT1],
            ]);

            let resolve_name = format!("{name}:{port}");
            let addrs_iter = resolve_name.to_socket_addrs();
            if let Ok(mut addrs) = addrs_iter {
                if let Some(addr) = addrs.next() {
                    u2_debug!("DNS name={name} resolved to {}", addr.ip());

                    if let IpAddr::V4(ip) = addr.ip() {
                        let octets = ip.octets().to_vec();

                        let dest =
                            &mut self.mem[base_addr + W5100_SN_DIPR0..=base_addr + W5100_SN_DIPR3];
                        dest[0] = octets[0];
                        dest[1] = octets[1];
                        dest[2] = octets[2];
                        dest[3] = octets[3];
                    }
                }
            }
        }
    }

    fn connect_socket(&mut self, i: usize) {
        let base_addr = self.get_base_socket_addr(i);

        // Check that the socket created is a TCP socket. If not close the socket
        if self.get_socket_status(i) != W5100_SN_SR_SOCK_INIT {
            self.clear_socket(i);
            return;
        }

        let dest = &self.mem[base_addr + W5100_SN_DIPR0..=base_addr + W5100_SN_DIPR3];
        let port_bytes = [
            self.mem[base_addr + W5100_SN_DPORT0],
            self.mem[base_addr + W5100_SN_DPORT1],
        ];
        let port = u16::from_be_bytes(port_bytes);
        let sock_addr = SocketAddr::new(
            IpAddr::V4(Ipv4Addr::new(dest[0], dest[1], dest[2], dest[3])),
            port,
        );
        let rtr_bytes = [self.mem[W5100_RTR0], self.mem[W5100_RTR1]];
        let rtr = u16::from_be_bytes(rtr_bytes) as u64;

        u2_debug!(
            "Connect Socket on #{i} to {sock_addr:?} with timeout = {} µs ...",
            rtr * 100
        );

        if let Ok(stream) = TcpStream::connect_timeout(&sock_addr, Duration::from_micros(rtr * 100))
        {
            u2_debug!("Connect Socket on #{i} to {sock_addr:?} - OK");
            stream
                .set_nonblocking(true)
                .expect("Cannot set non-blocking stream");
            self.socket[i].set_socket_handle(Proto::Tcp(stream));
            self.set_socket_status(i, W5100_SN_SR_SOCK_ESTABLISHED);
        } else {
            u2_debug!("Connect Socket on #{i} to {sock_addr:?} FAILED");
            self.clear_socket(i);
        }
    }

    fn listen_socket(&mut self, i: usize) {
        let base_addr = self.get_base_socket_addr(i);

        // Check if the TCP socket is in listening mode
        if self.get_socket_status(i) == W5100_SN_SR_SOCK_LISTEN {
            if let Some(stream) = self.accept_socket(i) {
                let socket = &mut self.socket[i];
                socket.set_socket_handle(Proto::Tcp(stream));
            }
            return;
        }

        // Check that the socket created is a TCP socket. If not close the socket
        if self.get_socket_status(i) != W5100_SN_SR_SOCK_INIT {
            self.clear_socket(i);
            return;
        }

        let src = [
            self.mem[W5100_SIPR0],
            self.mem[W5100_SIPR1],
            self.mem[W5100_SIPR2],
            self.mem[W5100_SIPR3],
        ];
        let port_bytes = [
            self.mem[base_addr + W5100_SN_PORT0],
            self.mem[base_addr + W5100_SN_PORT1],
        ];
        let port = u16::from_be_bytes(port_bytes);

        let listen_string = format!("{}.{}.{}.{}:{port}", src[0], src[1], src[2], src[3]);
        u2_debug!("Listen Socket on #{i} to {listen_string} ...");

        if let Ok(listener) = TcpListener::bind(&listen_string) {
            u2_debug!("Listen Socket on #{i} to {listen_string} - OK");
            listener
                .set_nonblocking(true)
                .expect("Cannot set non-blocking listener");
            self.socket[i].set_socket_handle(Proto::TcpListener(listener));
            self.set_socket_status(i, W5100_SN_SR_SOCK_LISTEN);
        } else {
            u2_debug!("Listen Socket on #{i} to {listen_string} FAILED");
            self.clear_socket(i);
        }
    }

    fn accept_socket(&mut self, i: usize) -> Option<TcpStream> {
        let socket = &mut self.socket[i];

        if let Proto::TcpListener(listener) = &mut socket.socket_handle {
            let mut listener_iter = listener.incoming();
            if let Some(stream) = listener_iter.next() {
                match stream {
                    Ok(s) => {
                        self.set_socket_status(i, W5100_SN_SR_SOCK_ESTABLISHED);
                        return Some(s);
                    }
                    Err(ref e) if e.kind() == ErrorKind::WouldBlock => return None,
                    Err(e) => {
                        u2_debug!("Couldn't get client on #{i}: {e:?}");
                        return None;
                    }
                }
            }
        }
        None
    }

    fn close_socket(&mut self, i: usize) {
        u2_debug!("Close Socket on #{i}");
        self.clear_socket(i);
    }

    fn send_data(&mut self, i: usize) {
        //u2_debug!("Send Data on #{i}");

        let base_addr = self.get_base_socket_addr(i);
        let socket = &mut self.socket[i];

        let size = socket.transmit_size;
        let mask = size - 1;
        let sn_tx_rr = (u16::from_be_bytes([
            self.mem[base_addr + W5100_SN_TX_RD0],
            self.mem[base_addr + W5100_SN_TX_RD1],
        ]) as usize)
            & mask;
        let sn_tx_wr = (u16::from_be_bytes([
            self.mem[base_addr + W5100_SN_TX_WR0],
            self.mem[base_addr + W5100_SN_TX_WR1],
        ]) as usize)
            & mask;

        let base = socket.transmit_addr;
        let rd_address = base + sn_tx_rr;
        let wr_address = base + sn_tx_wr;

        // Copy socket data to vector
        let mut data = Vec::new();
        if rd_address <= wr_address {
            data.extend_from_slice(&self.mem[rd_address..wr_address]);
        } else {
            let end = base + size;
            data.extend_from_slice(&self.mem[rd_address..end]);
            data.extend_from_slice(&self.mem[base..wr_address]);
        }

        // Move read pointer to writer
        self.mem[base_addr + W5100_SN_TX_RD0] = self.mem[base_addr + W5100_SN_TX_WR0];
        self.mem[base_addr + W5100_SN_TX_RD1] = self.mem[base_addr + W5100_SN_TX_WR1];

        if socket.is_open() {
            match socket.status {
                W5100_SN_SR_SOCK_ESTABLISHED => self.send_data_to_socket(i, &data),
                _ => u2_debug!("Send data Socket#{i} Unknown mode: 0x{:02X}", socket.status),
            }
        } else if socket.status == W5100_SN_SR_SOCK_MACRAW {
            #[cfg(feature = "pcap")]
            self.send_raw_data_to_socket(i, &data);
        }
    }

    fn send_data_to_socket(&mut self, i: usize, data: &[u8]) {
        let socket = &mut self.socket[i];

        if socket.is_open() {
            if let Proto::Tcp(stream) = &mut socket.socket_handle {
                let result = stream.write_all(data);
                if let Err(error) = result {
                    if !(matches!(error.kind(), ErrorKind::WouldBlock)) {
                        self.clear_socket(i);
                    }
                }
            }
        }
    }

    #[cfg(feature = "pcap")]
    fn send_raw_data_to_socket(&mut self, i: usize, data: &[u8]) {
        let socket = &mut self.socket[i];

        if let Proto::MacRaw(pcap_capture) = &mut socket.socket_handle {
            let result = pcap_capture.0.sendpacket(data);
            if let Err(error) = result {
                u2_debug!("Send Raw data Socket#{i} Error: {:?}", error);
                self.clear_socket(i);
            }
        }
    }

    fn update_rsr(&mut self, i: usize) {
        //u2_debug!("Receive Data on #{i}");

        let base_addr = self.get_base_socket_addr(i);
        let socket = &mut self.socket[i];
        let size = socket.receive_size;
        let mask = size - 1;

        let sn_rx_rd = (u16::from_be_bytes([
            self.mem[base_addr + W5100_SN_RX_RD0],
            self.mem[base_addr + W5100_SN_RX_RD1],
        ]) as usize)
            & mask;
        let sn_rx_wr = socket.receive_pointer & mask;
        let data_present = if sn_rx_rd > sn_rx_wr {
            sn_rx_wr + size - sn_rx_rd
        } else {
            sn_rx_wr - sn_rx_rd
        };

        let rsr_to_update = u16::to_be_bytes(data_present as u16);
        self.mem[base_addr + W5100_SN_RX_RSR0] = rsr_to_update[0];
        self.mem[base_addr + W5100_SN_RX_RSR1] = rsr_to_update[1];
    }
}

impl Card for Uthernet2 {
    fn rom_access(
        &mut self,
        _mem: &mut Mmu,
        _video: &mut Video,
        _addr: u16,
        _value: u8,
        _write_flag: bool,
    ) -> u8 {
        panic!("No ROM in Uthernet2. This function should not be called")
    }

    fn io_access(
        &mut self,
        _mem: &mut Mmu,
        _video: &mut Video,
        addr: u16,
        value: u8,
        write_flag: bool,
    ) -> u8 {
        let slot = (((addr & 0x00ff) - 0x0080) >> 4) as usize;
        let io_addr = ((addr & 0x00ff) - ((slot as u16) << 4)) as u8;
        let addr = io_addr & U2_C0X_MASK;

        let mut return_value = 0;

        match addr {
            // Mode register
            U2_C0X_MODE_REGISTER => {
                if write_flag {
                    u2_debug!("Write Mode = {value:02X}");
                    self.set_mode_register(value);
                } else {
                    u2_debug!("Read Mode = {:02X}", self.mode);
                    return_value = self.mode
                }
            }

            // Address High
            U2_C0X_ADDRESS_HIGH => {
                if write_flag {
                    self.addr = ((value as usize) << 8) | (self.addr & 0x00ff);
                    //u2_debug!("Write Address High = 0x{value:02X} 0x{:04X}", self.addr);
                } else {
                    return_value = (self.addr & 0xff00) >> 8;
                    //u2_debug!("Read Address High = 0x{value:02X} 0x{:04X}", self.addr);
                }
            }

            // Address Low
            U2_C0X_ADDRESS_LOW => {
                if write_flag {
                    self.addr = (value as usize) | (self.addr & 0xff00);
                    //u2_debug!("Write Address Low = 0x{value:02X} 0x{:04X}",self.addr);
                } else {
                    return_value = self.addr & 0x00ff;
                    //u2_debug!("Read Address Low = 0x{value:02X} 0x{:04X}",self.addr);
                }
            }

            // Data
            U2_C0X_DATA_PORT => {
                if write_flag {
                    self.write_value(value);
                } else {
                    //let curr_addr = self.addr;
                    return_value = self.read_value() as usize;
                    //u2_debug!("Read Data = 0x{curr_addr:04X} 0x{return_value:02X}");
                }
            }

            _ => {}
        }

        return_value as u8
    }
}
