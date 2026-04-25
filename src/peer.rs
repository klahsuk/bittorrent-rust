use crate::peer::MessageTag::*;
use bytes::BufMut;
use bytes::{Buf, BytesMut};
use tokio_util::codec::Decoder;
use tokio_util::codec::Encoder;

#[repr(C)]
#[derive(Debug)]
pub struct Handshake {
    pub length: u8,
    pub protocol: [u8; 19],
    pub reserved_bytes: [u8; 8],
    pub info_hash: [u8; 20],
    pub peer_id: [u8; 20],
}

impl Handshake {
    pub fn new(info_hash: [u8; 20], peer_id: [u8; 20]) -> Self {
        Handshake {
            length: 19,
            protocol: b"BitTorrent protocol".clone(),
            reserved_bytes: [0; 8],
            info_hash,
            peer_id,
        }
    }

    pub fn as_bytes_mut(&mut self) -> &mut [u8] {
        //casting Handshake -> HAndshake as bytes [u8] 
        let handshake_bytes =
            self as *mut Self as *mut [u8; std::mem::size_of::<Self>()];

        let handshake_bytes: &mut [u8; std::mem::size_of::<Self>()] =
            unsafe { &mut *handshake_bytes };

        handshake_bytes
    }
}

#[repr(C)]
pub struct Request {
    index: [u8; 4],
    begin: [u8; 4],
    length: [u8; 4]
}

impl Request {
    pub fn new(index: u32, begin: u32, length:u32) -> Self {
        Self { 
            index: index.to_be_bytes(), 
            begin: begin.to_be_bytes(), 
            length: length.to_be_bytes() }
    }

    pub fn index(&self) -> u32 {
        u32::from_be_bytes(self.index)
    }

    pub fn begin(&self) -> u32 {
        u32::from_be_bytes(self.begin)
    }

    pub fn length(&self) -> u32 {
        u32::from_be_bytes(self.length)
    }

    pub fn as_bytes_mut(&mut self) -> &mut [u8]{
        //casting request to byte array repr(c)
         let request_bytes =
            self as *mut Self as *mut [u8; std::mem::size_of::<Self>()];

        let request_bytes: &mut [u8; std::mem::size_of::<Self>()] =
            unsafe { &mut *request_bytes };

        request_bytes
    }
}


#[repr(C)]
pub struct Piece {
    index: [u8; 4],
    begin: [u8; 4],
    block: [u8]
}

impl Piece {
   
    pub fn index(&self) -> u32 {
        u32::from_be_bytes(self.index)
    }

    pub fn begin(&self) -> u32 {
        u32::from_be_bytes(self.begin)
    }

    pub fn block(&self) -> &[u8] {
        &self.block
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum MessageTag {
    Choke = 0,
    Unchoke = 1,
    Interested = 2,
    NotInterested = 3,
    Have = 4,
    BitField = 5,
    Request = 6,
    Piece = 7,
    Cancel = 8,
}

pub struct PeerMessage {
    pub tag: MessageTag,
    pub payload: Vec<u8>,
}

pub struct MessageFramer;

const MAX: usize = 1 << 16;

impl Encoder<PeerMessage> for MessageFramer {
    type Error = std::io::Error;

    fn encode(&mut self, item: PeerMessage, dst: &mut BytesMut) -> Result<(), Self::Error> {
        // Don't send a message if it is longer than the other end will
        // accept.
        if item.payload.len() + 1 > MAX {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("Frame of length {} is too large.", item.payload.len()),
            ));
        }

        // Convert the length into a byte array.
        // The cast to u32 cannot overflow due to the length check above.
        let len_slice = u32::to_be_bytes(item.payload.len() as u32 + 1);

        // Reserve space in the buffer.
        dst.reserve(4 /*length marker*/+ /*tag*/ 1 + item.payload.len());

        // Write the length and string to the buffer.
        dst.extend_from_slice(&len_slice);
        dst.put_u8(item.tag as u8);
        dst.extend_from_slice(&item.payload);
        Ok(())
    }
}

impl Decoder for MessageFramer {
    type Item = PeerMessage;
    type Error = std::io::Error;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        // Read length marker.
        
        if src.len() < 4 {
            // Not enough data to read length marker.
            return Ok(None);
        }

        if src.len() < 5 {
            // Not enough data to tag
            return Ok(None);
        }
        
        let mut length_bytes = [0u8; 4];
        length_bytes.copy_from_slice(&src[..4]);
        let length = u32::from_be_bytes(length_bytes) as usize;

        if length == 0 {
            //heartbeat to keep alive just discard
            src.advance(4);
            //trying again
            return self.decode(src);
        }

        // Check that the length is not too large to avoid a denial of
        // service attack where the server runs out of memory.
        if length > MAX {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("Frame of length decoded {} is too large.", length),
            ));
        }

        if src.len() < 4 + length {
            // The full string has not yet arrived.
            //
            // We reserve more space in the buffer. This is not strictly
            // necessary, but is a good idea performance-wise.
            src.reserve(4 + length - src.len());

            // We inform the Framed that we need more bytes to form the next
            // frame.
            return Ok(None);
        }

        // Use advance to modify src such that it no longer contains
        // this frame.

        let tag = match src[4] {
            0 => Choke,
            1 => Unchoke,
            2 => Interested,
            3 => NotInterested,
            4 => Have,
            5 => BitField,
            6 => Request,
            7 => Piece,
            8 => Cancel,
            _tag => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!("unexpected message type {}.", _tag),
                ))
            }
        };

        let data = if src.len() > 5 {
            src[5..4 + length - 1].to_vec()
        } else {
            Vec::new()
        };
        src.advance(4 + length);

        Ok(Some(PeerMessage { tag, payload: data }))
    }
}
