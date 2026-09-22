use bytes::{Buf, Bytes, BytesMut};
use flate2::write::ZlibDecoder;
use std::io::Write;
use thiserror::Error;

const MAX_PACKET_LENGTH: usize = 8 * 1024 * 1024;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum FramingError {
    #[error("declared packet length {0} exceeds the {MAX_PACKET_LENGTH} byte sanity limit")]
    PacketTooLarge(usize),
    #[error("declared decompressed length {0} exceeds the {MAX_PACKET_LENGTH} byte sanity limit")]
    DecompressedTooLarge(usize),
    #[error("zlib decompression failed: {0}")]
    Decompress(String),
    #[error("decompressed {actual} bytes but the packet declared {declared}")]
    DecompressedLengthMismatch { declared: usize, actual: usize },
    #[error("decompressed length {size} is below the compression threshold {threshold}")]
    BelowCompressionThreshold { size: usize, threshold: usize },
    #[error("truncated varint")]
    TruncatedVarInt,
    #[error("varint exceeds five bytes")]
    VarIntTooLong,
    #[error("negative varint")]
    NegativeVarInt,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FramedPacket {
    pub wire: Bytes,
    pub payload: Bytes,
}

pub struct PacketReader {
    buffer: BytesMut,
    compression_threshold: Option<i32>,
}

impl PacketReader {
    pub fn new() -> Self {
        Self {
            buffer: BytesMut::new(),
            compression_threshold: None,
        }
    }

    pub fn set_compression(&mut self, threshold: Option<i32>) {
        self.compression_threshold = threshold;
    }

    pub fn feed(&mut self, data: &[u8]) {
        self.buffer.extend_from_slice(data);
    }

    pub fn next_packet(&mut self) -> Result<Option<FramedPacket>, FramingError> {
        let Some((declared_len, prefix_len)) = read_varint(&self.buffer)? else {
            return Ok(None);
        };
        let declared_len = nonnegative_length(declared_len)?;
        if declared_len > MAX_PACKET_LENGTH {
            return Err(FramingError::PacketTooLarge(declared_len));
        }
        let frame_end = prefix_len + declared_len;
        if self.buffer.len() < frame_end {
            return Ok(None);
        }
        let wire = self.buffer.split_to(frame_end).freeze();
        let frame = wire.slice(prefix_len..);
        let payload = match self.compression_threshold {
            Some(threshold) => decompress_frame(frame, threshold)?,
            None => frame,
        };
        Ok(Some(FramedPacket { wire, payload }))
    }
}

impl Default for PacketReader {
    fn default() -> Self {
        Self::new()
    }
}

fn nonnegative_length(value: i32) -> Result<usize, FramingError> {
    if value < 0 {
        return Err(FramingError::NegativeVarInt);
    }
    Ok(value as usize)
}

fn read_varint(input: &[u8]) -> Result<Option<(i32, usize)>, FramingError> {
    let mut value = 0i32;
    for (index, byte) in input.iter().copied().enumerate().take(5) {
        value |= ((byte & 0x7f) as i32) << (7 * index);
        if byte & 0x80 == 0 {
            return Ok(Some((value, index + 1)));
        }
    }
    if input.len() < 5 {
        Ok(None)
    } else {
        Err(FramingError::VarIntTooLong)
    }
}

fn decompress_frame(mut frame: Bytes, threshold: i32) -> Result<Bytes, FramingError> {
    let (data_length, prefix_len) = read_varint(&frame)?.ok_or(FramingError::TruncatedVarInt)?;
    let data_length = nonnegative_length(data_length)?;
    frame.advance(prefix_len);
    if data_length == 0 {
        return Ok(frame);
    }
    if threshold >= 0 && data_length < threshold as usize {
        return Err(FramingError::BelowCompressionThreshold {
            size: data_length,
            threshold: threshold as usize,
        });
    }
    if data_length > MAX_PACKET_LENGTH {
        return Err(FramingError::DecompressedTooLarge(data_length));
    }
    let mut decoder = ZlibDecoder::new(Vec::with_capacity(data_length.min(64 * 1024)));
    decoder
        .write_all(&frame)
        .map_err(|error| FramingError::Decompress(error.to_string()))?;
    let decompressed = decoder
        .finish()
        .map_err(|error| FramingError::Decompress(error.to_string()))?;
    if decompressed.len() != data_length {
        return Err(FramingError::DecompressedLengthMismatch {
            declared: data_length,
            actual: decompressed.len(),
        });
    }
    Ok(Bytes::from(decompressed))
}

#[cfg(test)]
mod tests {
    use super::*;
    use flate2::{write::ZlibEncoder, Compression};
    use std::io::Write;

    fn varint(value: usize) -> Vec<u8> {
        let mut value = value as u32;
        let mut result = Vec::new();
        loop {
            let mut byte = (value & 0x7f) as u8;
            value >>= 7;
            if value != 0 {
                byte |= 0x80;
            }
            result.push(byte);
            if value == 0 {
                return result;
            }
        }
    }

    #[test]
    fn retains_wire_bytes() {
        let mut reader = PacketReader::new();
        let mut wire = varint(3);
        wire.extend([1, 2, 3]);
        reader.feed(&wire);
        let packet = reader.next_packet().unwrap().unwrap();
        assert_eq!(packet.wire, Bytes::from(wire));
        assert_eq!(packet.payload, Bytes::from_static(&[1, 2, 3]));
    }

    #[test]
    fn retains_compressed_wire_and_decodes_payload() {
        let payload = vec![7; 32];
        let mut compressed = ZlibEncoder::new(Vec::new(), Compression::default());
        compressed.write_all(&payload).unwrap();
        let compressed = compressed.finish().unwrap();
        let mut body = varint(payload.len());
        body.extend(compressed);
        let mut wire = varint(body.len());
        wire.extend(&body);
        let mut reader = PacketReader::new();
        reader.set_compression(Some(1));
        reader.feed(&wire);
        let packet = reader.next_packet().unwrap().unwrap();
        assert_eq!(packet.wire, Bytes::from(wire));
        assert_eq!(packet.payload, Bytes::from(payload));
    }

    #[test]
    fn accepts_packets_up_to_protocol_maximum() {
        let payload = vec![7; 3 * 1024 * 1024];
        let mut compressed = ZlibEncoder::new(Vec::new(), Compression::default());
        compressed.write_all(&payload).unwrap();
        let compressed = compressed.finish().unwrap();
        let mut body = varint(payload.len());
        body.extend(compressed);
        let mut wire = varint(body.len());
        wire.extend(body);

        let mut reader = PacketReader::new();
        reader.set_compression(Some(1));
        reader.feed(&wire);
        let packet = reader.next_packet().unwrap().unwrap();
        assert_eq!(packet.payload.len(), payload.len());
    }
}
