use kyra_proto_spike::crypto::{CryptoError, DecryptStream, EncryptStream};
use std::pin::Pin;
use std::task::{Context, Poll};
use thiserror::Error;
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};

#[derive(Debug, Error)]
pub enum EncryptionError {
    #[error(transparent)]
    Crypto(#[from] CryptoError),
}

pub struct EncryptedStream<S> {
    inner: S,
    encryptor: EncryptStream,
    decryptor: DecryptStream,
    pending: Vec<u8>,
    pending_plain_len: usize,
}

impl<S> EncryptedStream<S> {
    pub fn new(inner: S, key: &[u8], iv: &[u8]) -> Result<Self, EncryptionError> {
        Ok(Self {
            inner,
            encryptor: EncryptStream::new(key, iv)?,
            decryptor: DecryptStream::new(key, iv)?,
            pending: Vec::new(),
            pending_plain_len: 0,
        })
    }

    pub fn into_inner(self) -> S {
        self.inner
    }
}

impl<S: AsyncRead + Unpin> AsyncRead for EncryptedStream<S> {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buffer: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        let before = buffer.filled().len();
        let result = Pin::new(&mut self.inner).poll_read(cx, buffer);
        if let Poll::Ready(Ok(())) = &result {
            let filled = buffer.filled_mut();
            self.decryptor.apply_keystream(&mut filled[before..]);
        }
        result
    }
}

impl<S: AsyncWrite + Unpin> AsyncWrite for EncryptedStream<S> {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        input: &[u8],
    ) -> Poll<std::io::Result<usize>> {
        if !self.pending.is_empty() {
            let pending = std::mem::take(&mut self.pending);
            match Pin::new(&mut self.inner).poll_write(cx, &pending) {
                Poll::Ready(Ok(written)) if written == pending.len() => {
                    let plain_len = self.pending_plain_len;
                    self.pending_plain_len = 0;
                    return Poll::Ready(Ok(plain_len));
                }
                Poll::Ready(Ok(written)) => {
                    self.pending = pending[written..].to_vec();
                    return Poll::Pending;
                }
                Poll::Ready(Err(error)) => {
                    self.pending = pending;
                    return Poll::Ready(Err(error));
                }
                Poll::Pending => {
                    self.pending = pending;
                    return Poll::Pending;
                }
            }
        }
        if input.is_empty() {
            return Poll::Ready(Ok(0));
        }
        let mut encrypted = input.to_vec();
        self.encryptor.apply_keystream(&mut encrypted);
        match Pin::new(&mut self.inner).poll_write(cx, &encrypted) {
            Poll::Ready(Ok(written)) if written == encrypted.len() => Poll::Ready(Ok(input.len())),
            Poll::Ready(Ok(written)) => {
                self.pending = encrypted[written..].to_vec();
                self.pending_plain_len = input.len();
                Poll::Pending
            }
            Poll::Ready(Err(error)) => Poll::Ready(Err(error)),
            Poll::Pending => {
                self.pending = encrypted;
                self.pending_plain_len = input.len();
                Poll::Pending
            }
        }
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        if !self.pending.is_empty() {
            match self.as_mut().poll_write(cx, &[]) {
                Poll::Ready(Ok(_)) => {}
                Poll::Ready(Err(error)) => return Poll::Ready(Err(error)),
                Poll::Pending => return Poll::Pending,
            }
        }
        Pin::new(&mut self.inner).poll_flush(cx)
    }

    fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        match self.as_mut().poll_flush(cx) {
            Poll::Ready(Ok(())) => Pin::new(&mut self.inner).poll_shutdown(cx),
            Poll::Ready(Err(error)) => Poll::Ready(Err(error)),
            Poll::Pending => Poll::Pending,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    #[tokio::test]
    async fn round_trips_encrypted_stream_bytes() {
        let (left, right) = tokio::io::duplex(1024);
        let key = [0x42; 16];
        let iv = [0x24; 16];
        let mut writer = EncryptedStream::new(left, &key, &iv).unwrap();
        let mut reader = EncryptedStream::new(right, &key, &iv).unwrap();
        let payload = b"encrypted protocol bytes";
        writer.write_all(payload).await.unwrap();
        writer.shutdown().await.unwrap();
        let mut received = vec![0; payload.len()];
        reader.read_exact(&mut received).await.unwrap();
        assert_eq!(received, payload);
    }

    #[test]
    fn rejects_invalid_key_material() {
        let result = EncryptedStream::new(tokio::io::empty(), &[0; 15], &[0; 16]);
        assert!(matches!(result, Err(EncryptionError::Crypto(_))));
    }
}
