use crate::varint::{try_read_varint, VarIntError};
use bytes::{Buf, Bytes, BytesMut};
use flate2::write::ZlibDecoder;
use std::io::Write;
use thiserror::Error;

const MAX_PACKET_LENGTH: usize = 2 * 1024 * 1024;

#[derive(Debug, Error)]
pub enum FramingError {
    #[error("declared packet length {0} exceeds the {MAX_PACKET_LENGTH} byte sanity limit")]
    PacketTooLarge(usize),
    #[error("declared decompressed length {0} exceeds the {MAX_PACKET_LENGTH} byte sanity limit")]
    DecompressedTooLarge(usize),
    #[error("zlib decompression failed: {0}")]
    Decompress(#[from] std::io::Error),
    #[error("decompressed {actual} bytes but the packet declared {declared}")]
    DecompressedLengthMismatch { declared: usize, actual: usize },
    #[error(transparent)]
    VarInt(#[from] VarIntError),
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

    pub fn next_packet(&mut self) -> Result<Option<Bytes>, FramingError> {
        let Some((declared_len, prefix_len)) = try_read_varint(&self.buffer)? else {
            return Ok(None);
        };

        if declared_len < 0 {
            return Err(FramingError::PacketTooLarge(0));
        }
        let declared_len = declared_len as usize;
        if declared_len > MAX_PACKET_LENGTH {
            return Err(FramingError::PacketTooLarge(declared_len));
        }

        let frame_end = prefix_len + declared_len;
        if self.buffer.len() < frame_end {
            return Ok(None);
        }

        self.buffer.advance(prefix_len);
        let frame = self.buffer.split_to(declared_len).freeze();

        let payload = match self.compression_threshold {
            None => frame,
            Some(_) => decompress_frame(frame)?,
        };

        Ok(Some(payload))
    }
}

impl Default for PacketReader {
    fn default() -> Self {
        Self::new()
    }
}

fn decompress_frame(mut frame: Bytes) -> Result<Bytes, FramingError> {
    let (data_length, prefix_len) = try_read_varint(&frame)?.ok_or(VarIntError::Truncated)?;
    let data_length = data_length.max(0) as usize;
    frame.advance(prefix_len);

    if data_length == 0 {
        return Ok(frame);
    }

    if data_length > MAX_PACKET_LENGTH {
        return Err(FramingError::DecompressedTooLarge(data_length));
    }

    let mut decoder = ZlibDecoder::new(Vec::with_capacity(data_length.min(64 * 1024)));
    decoder.write_all(&frame)?;
    let decompressed = decoder.finish()?;

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
    use crate::varint::write_varint;
    use flate2::{write::ZlibEncoder, Compression};

    fn framed_uncompressed(payload: &[u8]) -> Vec<u8> {
        let mut out = Vec::new();
        write_varint(&mut out, payload.len() as i32);
        out.extend_from_slice(payload);
        out
    }

    fn framed_compressed(payload: &[u8], threshold: usize) -> Vec<u8> {
        let mut inner = Vec::new();
        if payload.len() >= threshold {
            write_varint(&mut inner, payload.len() as i32);
            let mut enc = ZlibEncoder::new(Vec::new(), Compression::default());
            enc.write_all(payload).unwrap();
            inner.extend(enc.finish().unwrap());
        } else {
            write_varint(&mut inner, 0);
            inner.extend_from_slice(payload);
        }
        let mut out = Vec::new();
        write_varint(&mut out, inner.len() as i32);
        out.extend(inner);
        out
    }

    #[test]
    fn reads_single_uncompressed_packet() {
        let mut reader = PacketReader::new();
        reader.feed(&framed_uncompressed(&[0x01, 0xAA, 0xBB]));
        let packet = reader.next_packet().unwrap().unwrap();
        assert_eq!(&packet[..], &[0x01, 0xAA, 0xBB]);
        assert!(reader.next_packet().unwrap().is_none());
    }

    #[test]
    fn waits_for_more_data_on_partial_packet() {
        let mut reader = PacketReader::new();
        let full = framed_uncompressed(&[0x01, 0xAA, 0xBB, 0xCC]);
        reader.feed(&full[..2]);
        assert!(reader.next_packet().unwrap().is_none());
        reader.feed(&full[2..]);
        let packet = reader.next_packet().unwrap().unwrap();
        assert_eq!(&packet[..], &[0x01, 0xAA, 0xBB, 0xCC]);
    }

    #[test]
    fn reads_multiple_packets_fed_in_one_chunk() {
        let mut reader = PacketReader::new();
        let mut both = framed_uncompressed(&[0x01, 0x02]);
        both.extend(framed_uncompressed(&[0x03, 0x04, 0x05]));
        reader.feed(&both);

        let first = reader.next_packet().unwrap().unwrap();
        assert_eq!(&first[..], &[0x01, 0x02]);
        let second = reader.next_packet().unwrap().unwrap();
        assert_eq!(&second[..], &[0x03, 0x04, 0x05]);
        assert!(reader.next_packet().unwrap().is_none());
    }

    #[test]
    fn reads_compressed_packet_above_threshold() {
        let mut reader = PacketReader::new();
        reader.set_compression(Some(2));
        let payload: Vec<u8> = (0..500).map(|i| (i % 251) as u8).collect();
        reader.feed(&framed_compressed(&payload, 2));
        let packet = reader.next_packet().unwrap().unwrap();
        assert_eq!(&packet[..], &payload[..]);
    }

    #[test]
    fn reads_below_threshold_packet_marked_uncompressed_via_zero_data_length() {
        let mut reader = PacketReader::new();
        reader.set_compression(Some(256));
        let payload = [0x01, 0x02, 0x03];
        reader.feed(&framed_compressed(&payload, 256));
        let packet = reader.next_packet().unwrap().unwrap();
        assert_eq!(&packet[..], &payload[..]);
    }

    #[test]
    fn rejects_oversized_declared_length_without_allocating() {
        let mut reader = PacketReader::new();
        let mut out = Vec::new();
        write_varint(&mut out, i32::MAX);
        reader.feed(&out);
        assert!(matches!(
            reader.next_packet(),
            Err(FramingError::PacketTooLarge(_))
        ));
    }

    #[test]
    fn rejects_decompressed_length_mismatch() {
        let mut reader = PacketReader::new();
        reader.set_compression(Some(0));
        let payload = [0x01, 0x02, 0x03];
        let mut inner = Vec::new();
        let false_declared_length = 999;
        write_varint(&mut inner, false_declared_length);
        let mut enc = ZlibEncoder::new(Vec::new(), Compression::default());
        enc.write_all(&payload).unwrap();
        inner.extend(enc.finish().unwrap());
        let mut out = Vec::new();
        write_varint(&mut out, inner.len() as i32);
        out.extend(inner);
        reader.feed(&out);
        assert!(matches!(
            reader.next_packet(),
            Err(FramingError::DecompressedLengthMismatch { .. })
        ));
    }
}
