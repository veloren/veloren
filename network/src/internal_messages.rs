use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use std::io::{Read, Write};
use tracing::*;

const VELOREN_MAGIC_NUMBER: &str = "VELOREN";
const VELOREN_NETWORK_VERSION_MAJOR: u16 = 0;
const VELOREN_NETWORK_VERSION_MINOR: u8 = 0;
const VELOREN_NETWORK_VERSION_PATCH: u8 = 1;

pub fn encode_handshake1<W: Write>(stream: &mut W, participant_id: u64) {
    stream.write_all(VELOREN_MAGIC_NUMBER.as_bytes()).unwrap();
    stream.write_u8('\n' as u8).unwrap();
    stream
        .write_u16::<BigEndian>(VELOREN_NETWORK_VERSION_MAJOR)
        .unwrap();
    stream.write_u8('.' as u8).unwrap();
    stream.write_u8(VELOREN_NETWORK_VERSION_MINOR).unwrap();
    stream.write_u8('.' as u8).unwrap();
    stream.write_u8(VELOREN_NETWORK_VERSION_PATCH).unwrap();
    stream.write_u8('\n' as u8).unwrap();
    stream.write_u64::<BigEndian>(participant_id).unwrap();
    stream.write_u8('\n' as u8).unwrap();
}

pub fn decode_handshake1<R: Read>(stream: &mut R) -> Result<(u16, u8, u8, u64), ()> {
    let mut veloren_buf: [u8; 7] = [0; 7];
    let mut major;
    let mut minor;
    let mut patch;
    let mut participant_id;
    match stream.read_exact(&mut veloren_buf) {
        Ok(()) if (veloren_buf == VELOREN_MAGIC_NUMBER.as_bytes()) => {},
        _ => {
            error!(?veloren_buf, "incompatible magic number");
            return Err(());
        },
    }
    match stream.read_u8().map(|u| u as char) {
        Ok('\n') => {},
        _ => return Err(()),
    }
    match stream.read_u16::<BigEndian>() {
        Ok(u) => major = u,
        _ => return Err(()),
    }
    match stream.read_u8().map(|u| u as char) {
        Ok('.') => {},
        _ => return Err(()),
    }
    match stream.read_u8() {
        Ok(u) => minor = u,
        _ => return Err(()),
    }
    match stream.read_u8().map(|u| u as char) {
        Ok('.') => {},
        _ => return Err(()),
    }
    match stream.read_u8() {
        Ok(u) => patch = u,
        _ => return Err(()),
    }
    match stream.read_u8().map(|u| u as char) {
        Ok('\n') => {},
        _ => return Err(()),
    }
    match stream.read_u64::<BigEndian>() {
        Ok(u) => participant_id = u,
        _ => return Err(()),
    }
    Ok((major, minor, patch, participant_id))
}

#[cfg(test)]
mod tests {
    use crate::{internal_messages::*, tests::test_tracing};

    #[test]
    fn handshake() {
        let mut data = Vec::new();
        encode_handshake1(&mut data, 1337);
        let dh = decode_handshake1(&mut data.as_slice());
        assert!(dh.is_ok());
        let (ma, mi, pa, p) = dh.unwrap();
        assert_eq!(ma, VELOREN_NETWORK_VERSION_MAJOR);
        assert_eq!(mi, VELOREN_NETWORK_VERSION_MINOR);
        assert_eq!(pa, VELOREN_NETWORK_VERSION_PATCH);
        assert_eq!(p, 1337);
    }

    #[test]
    fn handshake_decodeerror_incorrect() {
        let mut data = Vec::new();
        encode_handshake1(&mut data, 1337);
        data[3] = 'F' as u8;
        let dh = decode_handshake1(&mut data.as_slice());
        assert!(dh.is_err());
    }

    #[test]
    fn handshake_decodeerror_toless() {
        let mut data = Vec::new();
        encode_handshake1(&mut data, 1337);
        data.drain(9..);
        let dh = decode_handshake1(&mut data.as_slice());
        assert!(dh.is_err());
    }
}
