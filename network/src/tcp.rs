use crate::{channel::ChannelProtocol, types::Frame};
use bincode;
use mio::net::TcpStream;
use std::{
    io::{Read, Write},
    ops::Range,
};
use tracing::*;

pub(crate) struct TcpChannel {
    endpoint: TcpStream,
    //these buffers only ever contain 1 FRAME !
    read_buffer: Vec<u8>,
    write_buffer: Vec<u8>,
    filled_data: usize,
    serialized_data: usize,
    need_to_send_till: usize,
}

impl TcpChannel {
    pub fn new(endpoint: TcpStream) -> Self {
        //let mut b = vec![0; 1048576]; // 1 MB
        let mut b = vec![0; 2048]; // 1 MB
        Self {
            endpoint,
            read_buffer: b.clone(),
            write_buffer: b,
            filled_data: 0,
            serialized_data: 0,
            need_to_send_till: 0,
        }
    }
}

fn move_in_vec(vec: &mut Vec<u8>, src: Range<usize>, dest: Range<usize>) {
    debug_assert_eq!(src.end - src.start, dest.end - dest.start);
    let mut i2 = dest.start;
    for i in src {
        vec[i2] = vec[i];
        i2 += 1;
    }
}

impl ChannelProtocol for TcpChannel {
    type Handle = TcpStream;

    /// Execute when ready to read
    fn read(&mut self) -> Vec<Frame> {
        let mut result = Vec::new();
        loop {
            match self
                .endpoint
                .read(&mut self.read_buffer[self.filled_data..])
            {
                Ok(n) => {
                    trace!(?self.filled_data, "incomming message with len: {}", n);
                    self.filled_data += n;
                    let cursor_start = self.serialized_data;
                    let mut cur = std::io::Cursor::new(
                        &self.read_buffer[self.serialized_data..self.filled_data],
                    );
                    while cur.position() < n as u64 {
                        let round_start = cur.position() as usize;
                        let r: Result<Frame, _> = bincode::deserialize_from(&mut cur);
                        match r {
                            Ok(frame) => {
                                self.serialized_data = cursor_start + cur.position() as usize;
                                result.push(frame);
                            },
                            Err(e) => {
                                /* Probably we have to wait for moare data!
                                 * Our strategy is as follows: If there is space in our buffer,
                                 * we just set a flag to the failed start, and the point it's
                                 * filled to, On the next run, we
                                 * continue filling and retry to convert from the last point.
                                 * This way no memory needs to be copied, but we need a larger
                                 * buffer. Once either the
                                 * following will happen
                                 * a) We sucessfully deserialized everything we send -> So we can
                                 * safe reset to 0! b) Our buffer
                                 * is full =>    1) We started at
                                 * != 0 => we copy the memory to start, and set both variables to
                                 * 0    2) We need to increase
                                 * the buffer (this will never happenTM) */
                                let first_bytes_of_msg = &self.read_buffer[(round_start as usize)
                                    ..std::cmp::min(n as usize, (round_start + 16) as usize)];
                                debug!(?self, ?self.serialized_data, ?self.filled_data, ?e, ?n, ?round_start, ?first_bytes_of_msg, "message cant be parsed, probably because we need to wait for more data");
                                warn!("aa {:?}", self.read_buffer);
                                break;
                            },
                        }
                    }
                    if self.serialized_data == self.filled_data {
                        // reset the buffer as everything received was handled!
                        self.filled_data = 0;
                        self.serialized_data = 0;
                    } else {
                        // TODO: Checks for memory movement!
                        if self.filled_data == self.read_buffer.len() {
                            let move_src = self.serialized_data..self.filled_data;
                            trace!(?move_src, "readbuffer was full, moving memory to front");
                            warn!(?self.filled_data, ?self.serialized_data, "bb {:?}", self.read_buffer);
                            let move_dest = 0..self.filled_data - self.serialized_data;
                            move_in_vec(&mut self.read_buffer, move_src, move_dest.clone());
                            self.filled_data = move_dest.end;
                            self.serialized_data = 0;
                            warn!(?self.filled_data, ?self.serialized_data, "cc {:?}", self.read_buffer);
                        }
                    }
                },
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    debug!("would block");
                    break;
                },
                Err(e) => {
                    panic!("{}", e);
                },
            };
        }
        result
    }

    /// Execute when ready to write
    fn write(&mut self, frame: Frame) -> Result<(), ()> {
        if self.need_to_send_till != 0 {
            //send buffer first
            match self
                .endpoint
                .write(&self.write_buffer[..self.need_to_send_till])
            {
                Ok(n) if n == self.need_to_send_till => {
                    trace!("cleared buffer {}", n);
                    self.need_to_send_till = 0;
                },
                Ok(n) => {
                    debug!("could only send part of buffer, this is going bad if happens often! ");
                    let move_src = n..self.need_to_send_till;
                    let move_dest = 0..self.need_to_send_till - n;
                    move_in_vec(&mut self.read_buffer, move_src, move_dest.clone());
                    self.need_to_send_till = move_dest.end;
                },
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    debug!("would block");
                },
                Err(e) => {
                    panic!("{}", e);
                },
            };
        };
        if let Ok(mut data) = bincode::serialize(&frame) {
            let total = data.len();
            match self.endpoint.write(&data) {
                Ok(n) if n == total => {
                    trace!("send {} bytes", n);
                },
                Ok(n) => {
                    error!("could only send part");
                    self.write_buffer[self.need_to_send_till..self.need_to_send_till + total - n]
                        .clone_from_slice(&data[n..]);
                    self.need_to_send_till += total - n;
                    return Err(());
                },
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    debug!("would block");
                    self.write_buffer[self.need_to_send_till..self.need_to_send_till + total]
                        .clone_from_slice(&data[..]);
                    self.need_to_send_till += total;
                    return Err(());
                },
                Err(e) => {
                    panic!("{}", e);
                },
            };
        };
        Ok(())
    }

    fn get_handle(&self) -> &Self::Handle { &self.endpoint }
}

impl std::fmt::Debug for TcpChannel {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.endpoint)
    }
}
