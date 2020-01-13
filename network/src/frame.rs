#[derive(Debug)]
pub enum TcpFrame {
    Header {
        id: u64,
        length: u64,
    },
    Data {
        id: u64,
        frame_no: u64,
        data: Vec<u8>,
    },
}
