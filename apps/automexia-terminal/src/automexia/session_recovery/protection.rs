//! Native protection stays on the recovery worker, outside terminal hot paths.
use super::{StoreError, MAX_BYTES, MAX_PLAIN_BYTES};
use base64::{engine::general_purpose::STANDARD, Engine};
use serde::{Deserialize, Serialize};
use std::io::{Read, Write};
use zeroize::Zeroizing;

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Envelope {
    version: u32,
    protection: String,
    payload: String,
}

pub(super) struct Protection {
    #[cfg(unix)]
    key_name: String,
    #[cfg(unix)]
    key: Option<Zeroizing<Vec<u8>>>,
}
impl Protection {
    pub fn new(root: &std::path::Path) -> Self {
        #[cfg(unix)]
        {
            use sha2::{Digest, Sha256};
            Self {
                key_name: base64::engine::general_purpose::URL_SAFE_NO_PAD
                    .encode(Sha256::digest(root.as_os_str().as_encoded_bytes())),
                key: None,
            }
        }
        #[cfg(not(unix))]
        {
            let _ = root;
            Self {}
        }
    }
    pub fn seal(&mut self, bytes: &[u8]) -> Result<Vec<u8>, StoreError> {
        if bytes.len() > MAX_PLAIN_BYTES {
            return Err(StoreError::Invalid);
        }
        let mut compressed =
            flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::fast());
        compressed.write_all(bytes).map_err(|_| StoreError::Io)?;
        let compressed = Zeroizing::new(compressed.finish().map_err(|_| StoreError::Io)?);
        let payload = self.encrypt(&compressed)?;
        let encoded = serde_json::to_vec(&Envelope {
            version: 2,
            protection: Self::algorithm().into(),
            payload: STANDARD.encode(payload),
        })
        .map_err(|_| StoreError::Invalid)?;
        if encoded.len() > MAX_BYTES {
            return Err(StoreError::Invalid);
        }
        Ok(encoded)
    }
    pub fn open(&mut self, bytes: &[u8]) -> Result<Zeroizing<Vec<u8>>, StoreError> {
        if bytes.len() > MAX_BYTES {
            return Err(StoreError::Invalid);
        }
        let version: super::SchemaVersion =
            serde_json::from_slice(bytes).map_err(|_| StoreError::Invalid)?;
        match version.version {
            1 => return Ok(Zeroizing::new(bytes.to_vec())), // topology-only migration
            2 => {}
            _ => return Err(StoreError::Version),
        }
        let envelope: Envelope =
            serde_json::from_slice(bytes).map_err(|_| StoreError::Invalid)?;
        if envelope.protection != Self::algorithm() {
            return Err(StoreError::Protection);
        }
        let ciphertext = STANDARD
            .decode(envelope.payload)
            .map_err(|_| StoreError::Invalid)?;
        let compressed = Zeroizing::new(self.decrypt(&ciphertext)?);
        let mut decoded = Zeroizing::new(Vec::new());
        flate2::read::ZlibDecoder::new(compressed.as_slice())
            .take((MAX_PLAIN_BYTES + 1) as u64)
            .read_to_end(&mut decoded)
            .map_err(|_| StoreError::Invalid)?;
        if decoded.len() > MAX_PLAIN_BYTES {
            return Err(StoreError::Invalid);
        }
        Ok(decoded)
    }
}

#[cfg(windows)]
impl Protection {
    fn algorithm() -> &'static str {
        "dpapi-user-zlib-v1"
    }
    fn encrypt(&mut self, bytes: &[u8]) -> Result<Vec<u8>, StoreError> {
        dpapi(bytes, true)
    }
    fn decrypt(&mut self, bytes: &[u8]) -> Result<Vec<u8>, StoreError> {
        dpapi(bytes, false)
    }
}
#[cfg(windows)]
fn dpapi(bytes: &[u8], encrypt: bool) -> Result<Vec<u8>, StoreError> {
    use windows_sys::Win32::{
        Foundation::LocalFree,
        Security::Cryptography::{
            CryptProtectData, CryptUnprotectData, CRYPTPROTECT_UI_FORBIDDEN,
            CRYPT_INTEGER_BLOB,
        },
    };
    let input = CRYPT_INTEGER_BLOB {
        cbData: bytes.len().try_into().map_err(|_| StoreError::Invalid)?,
        pbData: bytes.as_ptr().cast_mut(),
    };
    let mut output = CRYPT_INTEGER_BLOB {
        cbData: 0,
        pbData: std::ptr::null_mut(),
    };
    // SAFETY: both input buffer and output descriptor remain valid for the
    // synchronous API. Current-user scope only; no UI or machine-wide key.
    let ok = unsafe {
        if encrypt {
            CryptProtectData(
                &input,
                std::ptr::null(),
                std::ptr::null(),
                std::ptr::null(),
                std::ptr::null(),
                CRYPTPROTECT_UI_FORBIDDEN,
                &mut output,
            )
        } else {
            CryptUnprotectData(
                &input,
                std::ptr::null_mut(),
                std::ptr::null(),
                std::ptr::null(),
                std::ptr::null(),
                CRYPTPROTECT_UI_FORBIDDEN,
                &mut output,
            )
        }
    };
    if ok == 0 {
        return Err(StoreError::Protection);
    }
    // A recovery payload always contains a nonempty compressed record. Avoid
    // creating a Rust slice from a null/empty native allocation on invalid data.
    if output.pbData.is_null() || output.cbData == 0 {
        // SAFETY: LocalFree accepts NULL or the allocation returned by DPAPI.
        unsafe { LocalFree(output.pbData.cast()) };
        return Err(StoreError::Protection);
    }
    // SAFETY: successful DPAPI returns this allocation and byte extent. Copy
    // before clearing and releasing it exactly once using the matching API.
    let result = unsafe {
        let buffer =
            std::slice::from_raw_parts_mut(output.pbData, output.cbData as usize);
        let result = buffer.to_vec();
        zeroize::Zeroize::zeroize(buffer);
        LocalFree(output.pbData.cast());
        result
    };
    Ok(result)
}

#[cfg(unix)]
impl Protection {
    fn algorithm() -> &'static str {
        "platform-xchacha20poly1305-zlib-v1"
    }
    fn cipher(
        &mut self,
        create: bool,
    ) -> Result<chacha20poly1305::XChaCha20Poly1305, StoreError> {
        use chacha20poly1305::{aead::KeyInit, XChaCha20Poly1305};
        if self.key.is_none() {
            let key = match read_platform_key(&self.key_name)? {
                Some(key) => Zeroizing::new(key),
                None if create => {
                    let mut key = Zeroizing::new(vec![0; 32]);
                    getrandom::fill(&mut key).map_err(|_| StoreError::Protection)?;
                    write_platform_key(&self.key_name, &key)?;
                    key
                }
                None => return Err(StoreError::Protection),
            };
            if key.len() != 32 {
                return Err(StoreError::Protection);
            }
            self.key = Some(key);
        }
        XChaCha20Poly1305::new_from_slice(
            self.key.as_ref().ok_or(StoreError::Protection)?,
        )
        .map_err(|_| StoreError::Protection)
    }
    fn encrypt(&mut self, bytes: &[u8]) -> Result<Vec<u8>, StoreError> {
        use chacha20poly1305::{
            aead::{Aead, Payload},
            XNonce,
        };
        let cipher = self.cipher(true)?;
        let mut nonce = [0; 24];
        getrandom::fill(&mut nonce).map_err(|_| StoreError::Protection)?;
        let encrypted = cipher
            .encrypt(
                &XNonce::from(nonce),
                Payload {
                    msg: bytes,
                    aad: Self::algorithm().as_bytes(),
                },
            )
            .map_err(|_| StoreError::Protection)?;
        let mut result = nonce.to_vec();
        result.extend(encrypted);
        Ok(result)
    }
    fn decrypt(&mut self, bytes: &[u8]) -> Result<Vec<u8>, StoreError> {
        use chacha20poly1305::{
            aead::{Aead, Payload},
            XNonce,
        };
        if bytes.len() < 40 {
            return Err(StoreError::Invalid);
        }
        let nonce: [u8; 24] = bytes[..24].try_into().map_err(|_| StoreError::Invalid)?;
        self.cipher(false)?
            .decrypt(
                &XNonce::from(nonce),
                Payload {
                    msg: &bytes[24..],
                    aad: Self::algorithm().as_bytes(),
                },
            )
            .map_err(|_| StoreError::Protection)
    }
}

// Unit tests use a deterministic in-memory key, but the real authenticated
// encryption path. They never touch a developer's credential store.
#[cfg(all(unix, test))]
fn read_platform_key(name: &str) -> Result<Option<Vec<u8>>, StoreError> {
    use sha2::{Digest, Sha256};
    Ok(Some(Sha256::digest(name.as_bytes()).to_vec()))
}
#[cfg(all(unix, test))]
fn write_platform_key(_: &str, _: &[u8]) -> Result<(), StoreError> {
    Err(StoreError::Protection)
}
// Unsupported platforms must not silently persist plaintext.
#[cfg(all(
    unix,
    not(test),
    not(any(
        target_os = "linux",
        target_os = "freebsd",
        target_os = "openbsd",
        target_os = "macos"
    ))
))]
fn read_platform_key(_: &str) -> Result<Option<Vec<u8>>, StoreError> {
    Err(StoreError::Protection)
}
#[cfg(all(
    unix,
    not(test),
    not(any(
        target_os = "linux",
        target_os = "freebsd",
        target_os = "openbsd",
        target_os = "macos"
    ))
))]
fn write_platform_key(_: &str, _: &[u8]) -> Result<(), StoreError> {
    Err(StoreError::Protection)
}
#[cfg(not(any(unix, windows)))]
impl Protection {
    fn algorithm() -> &'static str {
        "unavailable"
    }
    fn encrypt(&mut self, _: &[u8]) -> Result<Vec<u8>, StoreError> {
        Err(StoreError::Protection)
    }
    fn decrypt(&mut self, _: &[u8]) -> Result<Vec<u8>, StoreError> {
        Err(StoreError::Protection)
    }
}

#[cfg(all(
    any(target_os = "linux", target_os = "freebsd", target_os = "openbsd"),
    not(test)
))]
fn read_platform_key(name: &str) -> Result<Option<Vec<u8>>, StoreError> {
    use secret_service::{blocking::SecretService, EncryptionType};
    let service =
        SecretService::connect(EncryptionType::Dh).map_err(|_| StoreError::Protection)?;
    let found = service
        .search_items(std::collections::HashMap::from([
            ("application", "automexia-recovery"),
            ("store", name),
        ]))
        .map_err(|_| StoreError::Protection)?;
    if !found.locked.is_empty() || found.unlocked.len() > 1 {
        return Err(StoreError::Protection);
    }
    found
        .unlocked
        .first()
        .map(|item| item.get_secret().map_err(|_| StoreError::Protection))
        .transpose()
}
#[cfg(all(
    any(target_os = "linux", target_os = "freebsd", target_os = "openbsd"),
    not(test)
))]
fn write_platform_key(name: &str, key: &[u8]) -> Result<(), StoreError> {
    use secret_service::{blocking::SecretService, EncryptionType};
    let service =
        SecretService::connect(EncryptionType::Dh).map_err(|_| StoreError::Protection)?;
    let collection = service
        .get_default_collection()
        .map_err(|_| StoreError::Protection)?;
    if collection.is_locked().map_err(|_| StoreError::Protection)? {
        return Err(StoreError::Protection);
    }
    collection
        .create_item(
            "Automexia recovery key",
            std::collections::HashMap::from([
                ("application", "automexia-recovery"),
                ("store", name),
            ]),
            key,
            false,
            "application/octet-stream",
        )
        .map_err(|_| StoreError::Protection)?;
    Ok(())
}
#[cfg(all(target_os = "macos", not(test)))]
mod macos;
#[cfg(all(target_os = "macos", not(test)))]
use macos::{read_platform_key, write_platform_key};

#[cfg(all(test, any(windows, unix)))]
mod tests {
    use super::*;
    #[cfg(unix)]
    #[test]
    fn unix_nonces_are_fresh_and_another_store_key_cannot_open_history() {
        let mut protection = Protection::new(std::path::Path::new("fixture"));
        let first = protection.seal(b"saved output").unwrap();
        let second = protection.seal(b"saved output").unwrap();
        assert_ne!(first, second);
        let mut reopened = Protection::new(std::path::Path::new("fixture"));
        assert_eq!(reopened.open(&first).unwrap().as_slice(), b"saved output");
        let mut different = Protection::new(std::path::Path::new("other-fixture"));
        assert!(different.open(&first).is_err());
    }
    #[test]
    fn protected_roundtrip_rejects_tampering_and_does_not_store_plaintext() {
        let mut protection = Protection::new(std::path::Path::new("fixture"));
        let bytes = protection.seal(b"private history sentinel").unwrap();
        assert!(!String::from_utf8_lossy(&bytes).contains("private history sentinel"));
        assert_eq!(
            protection.open(&bytes).unwrap().as_slice(),
            b"private history sentinel"
        );
        let mut envelope: Envelope = serde_json::from_slice(&bytes).unwrap();
        let mut ciphertext = STANDARD.decode(envelope.payload).unwrap();
        let last = ciphertext.len() - 1;
        ciphertext[last] ^= 1;
        envelope.payload = STANDARD.encode(ciphertext);
        assert!(protection
            .open(&serde_json::to_vec(&envelope).unwrap())
            .is_err());
    }
}
