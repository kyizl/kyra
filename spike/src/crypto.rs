use aes::Aes128;
use cfb8::cipher::KeyIvInit;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum CryptoError {
    #[error("AES key/IV must be exactly 16 bytes, got {0}")]
    InvalidKeyOrIvLength(usize),
}

pub struct EncryptStream(cfb8::Encryptor<Aes128>);

pub struct DecryptStream(cfb8::Decryptor<Aes128>);

fn to_key_iv(key: &[u8], iv: &[u8]) -> Result<([u8; 16], [u8; 16]), CryptoError> {
    let key: [u8; 16] = key
        .try_into()
        .map_err(|_| CryptoError::InvalidKeyOrIvLength(key.len()))?;
    let iv: [u8; 16] = iv
        .try_into()
        .map_err(|_| CryptoError::InvalidKeyOrIvLength(iv.len()))?;
    Ok((key, iv))
}

impl EncryptStream {
    pub fn new(key: &[u8], iv: &[u8]) -> Result<Self, CryptoError> {
        let (key, iv) = to_key_iv(key, iv)?;
        Ok(Self(cfb8::Encryptor::new(&key.into(), &iv.into())))
    }

    pub fn apply_keystream(&mut self, buf: &mut [u8]) {
        self.0.encrypt(buf);
    }
}

impl DecryptStream {
    pub fn new(key: &[u8], iv: &[u8]) -> Result<Self, CryptoError> {
        let (key, iv) = to_key_iv(key, iv)?;
        Ok(Self(cfb8::Decryptor::new(&key.into(), &iv.into())))
    }

    pub fn apply_keystream(&mut self, buf: &mut [u8]) {
        self.0.decrypt(buf);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const KEY: [u8; 16] = [0x42; 16];
    const IV: [u8; 16] = [0x24; 16];
    const PLAINTEXT: &[u8] = b"hello world! this is my plaintext.";
    const EXPECTED_CIPHERTEXT_PER_CFB8_CRATE_DOCTEST: [u8; 34] = [
        0x33, 0xb3, 0x56, 0xce, 0x91, 0x84, 0x29, 0x0c, 0x4c, 0x8f, 0xac, 0xc1, 0xc0, 0xb1, 0xf9,
        0x18, 0xd5, 0x47, 0x5a, 0xeb, 0x75, 0xb8, 0x8c, 0x16, 0x1c, 0xa6, 0x5b, 0xdf, 0x05, 0xc7,
        0x13, 0x7f, 0xf4, 0xb0,
    ];

    #[test]
    fn matches_known_answer_test_vector() {
        let mut enc = EncryptStream::new(&KEY, &IV).unwrap();
        let mut buf = PLAINTEXT.to_vec();
        enc.apply_keystream(&mut buf);
        assert_eq!(buf, EXPECTED_CIPHERTEXT_PER_CFB8_CRATE_DOCTEST);
    }

    #[test]
    fn round_trips_through_separate_encrypt_and_decrypt_streams() {
        let mut enc = EncryptStream::new(&KEY, &IV).unwrap();
        let mut dec = DecryptStream::new(&KEY, &IV).unwrap();

        let mut buf = PLAINTEXT.to_vec();
        enc.apply_keystream(&mut buf);
        dec.apply_keystream(&mut buf);

        assert_eq!(buf, PLAINTEXT);
    }

    #[test]
    fn keystream_state_survives_across_many_small_calls() {
        let mut enc_whole = EncryptStream::new(&KEY, &IV).unwrap();
        let mut whole = PLAINTEXT.to_vec();
        enc_whole.apply_keystream(&mut whole);

        let mut enc_chunked = EncryptStream::new(&KEY, &IV).unwrap();
        let mut chunked = PLAINTEXT.to_vec();
        for chunk in chunked.chunks_mut(3) {
            enc_chunked.apply_keystream(chunk);
        }

        assert_eq!(whole, chunked);
    }

    #[test]
    fn rejects_wrong_length_key() {
        assert!(matches!(
            EncryptStream::new(&[0u8; 15], &IV),
            Err(CryptoError::InvalidKeyOrIvLength(15))
        ));
    }
}
