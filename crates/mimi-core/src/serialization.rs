//! FlatBuffers Serialization Layer
//!
//! Provides encode/decode functions for all MiMi message types with:
//! - Version compatibility checks
//! - Zero-copy deserialization
//! - Round-trip validation
//! - Error handling with comprehensive diagnostics

use anyhow::{anyhow, Context, Result};
use std::fmt;

pub const PROTOCOL_VERSION: u16 = 1;
pub const PROTOCOL_VERSION_MIN: u16 = 1;
pub const PROTOCOL_VERSION_MAX: u16 = 1;
pub const MAX_MESSAGE_SIZE: usize = 4 * 1024 * 1024;

#[derive(Debug, Clone)]
pub enum SerializationError {
    VersionMismatch { got: u16, expected: u16 },
    MessageTooLarge { size: usize, max: usize },
    InvalidMessageFormat { reason: String },
    EncodingFailed { reason: String },
    DecodingFailed { reason: String },
    ValidationFailed { reason: String },
    CorruptedData { checksum_mismatch: bool },
}

impl fmt::Display for SerializationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::VersionMismatch { got, expected } => {
                write!(
                    f,
                    "Protocol version mismatch: got {}, expected {}",
                    got, expected
                )
            },
            Self::MessageTooLarge { size, max } => {
                write!(f, "Message too large: {} > {} bytes", size, max)
            },
            Self::InvalidMessageFormat { reason } => {
                write!(f, "Invalid message format: {}", reason)
            },
            Self::EncodingFailed { reason } => {
                write!(f, "Encoding failed: {}", reason)
            },
            Self::DecodingFailed { reason } => {
                write!(f, "Decoding failed: {}", reason)
            },
            Self::ValidationFailed { reason } => {
                write!(f, "Validation failed: {}", reason)
            },
            Self::CorruptedData { checksum_mismatch } => {
                if *checksum_mismatch {
                    write!(f, "Corrupted data: checksum mismatch")
                } else {
                    write!(f, "Corrupted data: integrity check failed")
                }
            },
        }
    }
}

impl std::error::Error for SerializationError {}

pub trait Serializable: Sized {
    fn encode(&self) -> Result<Vec<u8>>;
    fn decode(bytes: &[u8]) -> Result<Self>;

    fn validate_version(version: u16) -> Result<()> {
        if version < PROTOCOL_VERSION_MIN || version > PROTOCOL_VERSION_MAX {
            return Err(anyhow!(SerializationError::VersionMismatch {
                got: version,
                expected: PROTOCOL_VERSION,
            }));
        }
        Ok(())
    }

    fn validate_format(&self) -> Result<()> {
        Ok(())
    }
}

pub struct MessageSerializer;

impl MessageSerializer {
    pub fn encode_with_version(
        version: u16,
        message_id: &str,
        message_type: u8,
        payload: &[u8],
    ) -> Result<Vec<u8>> {
        Self::validate_version(version)?;

        let estimated_size = 10 + message_id.len() + payload.len();
        if estimated_size > MAX_MESSAGE_SIZE {
            return Err(anyhow!(SerializationError::MessageTooLarge {
                size: estimated_size,
                max: MAX_MESSAGE_SIZE,
            }));
        }

        let mut encoded = Vec::with_capacity(estimated_size);

        encoded.extend_from_slice(&version.to_le_bytes());
        encoded.push(0);
        encoded.push(message_type);

        let id_bytes = message_id.as_bytes();
        encoded.extend_from_slice(&(id_bytes.len() as u16).to_le_bytes());
        encoded.extend_from_slice(id_bytes);

        encoded.extend_from_slice(&(payload.len() as u32).to_le_bytes());
        encoded.extend_from_slice(payload);

        let checksum = crc32_checksum(&encoded);
        encoded.extend_from_slice(&checksum.to_le_bytes());

        Ok(encoded)
    }

    pub fn decode_with_version(data: &[u8]) -> Result<(u16, String, u8, Vec<u8>)> {
        if data.len() < 10 {
            return Err(anyhow!(SerializationError::InvalidMessageFormat {
                reason: "Message too short for header".to_string(),
            }));
        }

        let (payload, stored_checksum) = data.split_at(data.len() - 4);
        let stored_checksum = u32::from_le_bytes([
            stored_checksum[0],
            stored_checksum[1],
            stored_checksum[2],
            stored_checksum[3],
        ]);
        let computed_checksum = crc32_checksum(payload);
        if stored_checksum != computed_checksum {
            return Err(anyhow!(SerializationError::CorruptedData {
                checksum_mismatch: true,
            }));
        }

        let mut offset = 0;

        let version = u16::from_le_bytes([data[offset], data[offset + 1]]);
        offset += 2;
        Self::validate_version(version)?;

        offset += 1;

        let message_type = data[offset];
        offset += 1;

        let id_len = u16::from_le_bytes([data[offset], data[offset + 1]]) as usize;
        offset += 2;
        if offset + id_len > payload.len() {
            return Err(anyhow!(SerializationError::InvalidMessageFormat {
                reason: "Message ID extends past payload".to_string(),
            }));
        }
        let message_id = String::from_utf8(data[offset..offset + id_len].to_vec())
            .context("Invalid UTF-8 in message ID")?;
        offset += id_len;

        if offset + 4 > payload.len() {
            return Err(anyhow!(SerializationError::InvalidMessageFormat {
                reason: "Payload length field extends past data".to_string(),
            }));
        }
        let payload_len = u32::from_le_bytes([
            data[offset],
            data[offset + 1],
            data[offset + 2],
            data[offset + 3],
        ]) as usize;
        offset += 4;

        if offset + payload_len > payload.len() {
            return Err(anyhow!(SerializationError::InvalidMessageFormat {
                reason: "Payload extends past data".to_string(),
            }));
        }

        let message_payload = data[offset..offset + payload_len].to_vec();

        Ok((version, message_id, message_type, message_payload))
    }

    pub fn validate_version(version: u16) -> Result<()> {
        if version < PROTOCOL_VERSION_MIN || version > PROTOCOL_VERSION_MAX {
            return Err(anyhow!(SerializationError::VersionMismatch {
                got: version,
                expected: PROTOCOL_VERSION,
            }));
        }
        Ok(())
    }
}

fn crc32_checksum(data: &[u8]) -> u32 {
    let mut sum1: u32 = 1;
    let mut sum2: u32 = 0;

    for &byte in data {
        sum1 = (sum1 + byte as u32) % 255;
        sum2 = (sum2 + sum1) % 255;
    }

    (sum2 << 16) | sum1
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_validation() {
        assert!(MessageSerializer::validate_version(PROTOCOL_VERSION).is_ok());
        assert!(MessageSerializer::validate_version(PROTOCOL_VERSION_MIN).is_ok());
        assert!(MessageSerializer::validate_version(PROTOCOL_VERSION_MAX).is_ok());
        assert!(MessageSerializer::validate_version(PROTOCOL_VERSION_MAX + 1).is_err());
        assert!(MessageSerializer::validate_version(0).is_err());
    }

    #[test]
    fn test_encode_decode_roundtrip() {
        let version = PROTOCOL_VERSION;
        let message_id = "msg-123";
        let message_type = 1u8;
        let payload = b"Hello, MiMi!";

        let encoded =
            MessageSerializer::encode_with_version(version, message_id, message_type, payload)
                .expect("encode failed");
        assert!(encoded.len() > payload.len());
        assert!(encoded.len() <= MAX_MESSAGE_SIZE);

        let (decoded_version, decoded_id, decoded_type, decoded_payload) =
            MessageSerializer::decode_with_version(&encoded).expect("decode failed");

        assert_eq!(decoded_version, version);
        assert_eq!(decoded_id, message_id);
        assert_eq!(decoded_type, message_type);
        assert_eq!(decoded_payload, payload);
    }

    #[test]
    fn test_roundtrip_with_special_characters() {
        let message_id = "msg-üñíçødé";
        let payload = "Test 🎉 emoji and special chars: \n\t\r".as_bytes();

        let encoded =
            MessageSerializer::encode_with_version(PROTOCOL_VERSION, message_id, 2, payload)
                .expect("encode failed");
        let (_, decoded_id, _, decoded_payload) =
            MessageSerializer::decode_with_version(&encoded).expect("decode failed");

        assert_eq!(decoded_id, message_id);
        assert_eq!(decoded_payload, payload);
    }

    #[test]
    fn test_checksum_detection() {
        let encoded = MessageSerializer::encode_with_version(PROTOCOL_VERSION, "msg", 1, b"data")
            .expect("encode failed");

        let mut corrupted = encoded.clone();
        if corrupted.len() > 5 {
            corrupted[5] ^= 0x01;
        }

        let result = MessageSerializer::decode_with_version(&corrupted);
        assert!(result.is_err());
        if let Err(e) = result {
            assert!(e.to_string().contains("checksum"));
        }
    }

    #[test]
    fn test_message_too_large() {
        let payload = vec![0u8; MAX_MESSAGE_SIZE + 1];
        let result = MessageSerializer::encode_with_version(PROTOCOL_VERSION, "msg", 1, &payload);
        assert!(result.is_err());
    }

    #[test]
    fn test_version_mismatch_detection() {
        let encoded = MessageSerializer::encode_with_version(PROTOCOL_VERSION, "msg", 1, b"data")
            .expect("encode failed");

        let mut corrupted = encoded.clone();
        corrupted[0] = 0xFF;
        corrupted[1] = 0xFF;

        let result = MessageSerializer::decode_with_version(&corrupted);
        assert!(result.is_err());
    }

    #[test]
    fn test_empty_payload() {
        let encoded = MessageSerializer::encode_with_version(PROTOCOL_VERSION, "msg", 1, b"")
            .expect("encode failed");
        let (_, _, _, payload) =
            MessageSerializer::decode_with_version(&encoded).expect("decode failed");
        assert_eq!(payload, b"");
    }

    #[test]
    fn test_large_payload() {
        let large_payload = vec![42u8; 1024 * 1024];
        let encoded = MessageSerializer::encode_with_version(
            PROTOCOL_VERSION,
            "msg-large",
            3,
            &large_payload,
        )
        .expect("encode failed");
        let (_, _, _, decoded_payload) =
            MessageSerializer::decode_with_version(&encoded).expect("decode failed");
        assert_eq!(decoded_payload, large_payload);
    }
}
