use base64::{engine::general_purpose::STANDARD, Engine};
use ring::aead::{Aad, LessSafeKey, Nonce, UnboundKey, AES_256_GCM};
use ring::rand::{SecureRandom, SystemRandom};
use std::fs::{self, DirBuilder, OpenOptions};
use std::io::Write;
use std::path::Path;

const PREFIX: &str = "local:v1:";

fn encryption_key(directory: &Path, create: bool) -> Result<[u8; 32], String> {
    let secret_directory = directory.join("traffic-secrets");
    let path = secret_directory.join("encryption.key");
    match fs::read(&path) {
        Ok(key) => {
            return key
                .try_into()
                .map_err(|_| "Invalid local encryption key.".to_owned())
        }
        Err(error) if create && error.kind() == std::io::ErrorKind::NotFound => {}
        Err(_) => return Err("Could not read the local encryption key.".to_owned()),
    }
    let mut builder = DirBuilder::new();
    builder.recursive(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt;
        builder.mode(0o700);
    }
    builder
        .create(&secret_directory)
        .map_err(|_| "Could not create the API key storage directory.")?;
    let mut key = [0u8; 32];
    SystemRandom::new()
        .fill(&mut key)
        .map_err(|_| "Could not generate an encryption key.")?;
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    match options.open(path) {
        Ok(mut file) => {
            file.write_all(&key)
                .and_then(|_| file.sync_all())
                .map_err(|_| "Could not save the local encryption key.")?;
            Ok(key)
        }
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
            encryption_key(directory, false)
        }
        Err(_) => Err("Could not save the local encryption key.".to_owned()),
    }
}

fn cipher(directory: &Path, create: bool) -> Result<LessSafeKey, String> {
    let key = encryption_key(directory, create)?;
    UnboundKey::new(&AES_256_GCM, &key)
        .map(LessSafeKey::new)
        .map_err(|_| "Invalid local encryption key.".to_owned())
}

pub fn encrypt(directory: &Path, value: &str) -> Result<String, String> {
    let key = cipher(directory, true)?;
    let mut nonce = [0u8; 12];
    SystemRandom::new()
        .fill(&mut nonce)
        .map_err(|_| "Could not generate an encryption nonce.")?;
    let mut ciphertext = value.as_bytes().to_vec();
    key.seal_in_place_append_tag(
        Nonce::assume_unique_for_key(nonce),
        Aad::empty(),
        &mut ciphertext,
    )
    .map_err(|_| "Could not encrypt the API key.")?;
    let payload = [nonce.as_slice(), &ciphertext].concat();
    Ok(format!("{PREFIX}{}", STANDARD.encode(payload)))
}

pub fn decrypt(directory: &Path, value: &str) -> Result<String, String> {
    let encoded = value
        .strip_prefix(PREFIX)
        .ok_or("Invalid encrypted API key.")?;
    let payload = STANDARD
        .decode(encoded)
        .map_err(|_| "Invalid encrypted API key.")?;
    if payload.len() < 29 {
        return Err("Invalid encrypted API key.".to_owned());
    }
    let nonce = Nonce::try_assume_unique_for_key(&payload[..12])
        .map_err(|_| "Invalid encrypted API key.")?;
    let mut ciphertext = payload[12..].to_vec();
    let key = cipher(directory, false)?;
    let plaintext = key
        .open_in_place(nonce, Aad::empty(), &mut ciphertext)
        .map_err(|_| "Could not decrypt the saved API key. Replace it in Settings → Traffic.")?;
    String::from_utf8(plaintext.to_vec()).map_err(|_| "Invalid decrypted API key.".to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encrypts_persists_and_authenticates_without_a_system_keyring() {
        let directory = tempfile::tempdir().unwrap();
        let value = "test-api-secret";
        let encrypted = encrypt(directory.path(), value).unwrap();
        assert!(!encrypted.contains(value));
        assert_eq!(decrypt(directory.path(), &encrypted).unwrap(), value);
        let replacement = encrypt(directory.path(), "replacement-secret").unwrap();
        assert_eq!(
            decrypt(directory.path(), &replacement).unwrap(),
            "replacement-secret"
        );
        assert_ne!(encrypted, encrypt(directory.path(), value).unwrap());
        let mut payload = STANDARD
            .decode(encrypted.strip_prefix(PREFIX).unwrap())
            .unwrap();
        payload[12] ^= 1;
        assert!(decrypt(
            directory.path(),
            &format!("{PREFIX}{}", STANDARD.encode(payload))
        )
        .is_err());
        let key_path = directory.path().join("traffic-secrets/encryption.key");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                fs::metadata(&key_path).unwrap().permissions().mode() & 0o777,
                0o600
            );
            assert_eq!(
                fs::metadata(key_path.parent().unwrap())
                    .unwrap()
                    .permissions()
                    .mode()
                    & 0o777,
                0o700
            );
        }
        fs::remove_file(&key_path).unwrap();
        assert!(decrypt(directory.path(), &encrypted).is_err());
        assert!(!key_path.exists());
    }
}
