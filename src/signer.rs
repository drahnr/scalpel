use bytes::Bytes;
use untrusted;

use errors::*;
use failure::Fail;
use ring;
use ring::{rand, signature};
use std::fmt::Debug;
use std::fs::OpenOptions;
use std::io::Read;
use std::path::Path;

pub struct Signer {
    pub keypair: Option<signature::Ed25519KeyPair>,
}

impl Signer {
    /// generate a ed25519 keypair in pkcs8 format
    fn generate_ed25519_keypair() -> Option<signature::Ed25519KeyPair> {
        let rng = rand::SystemRandom::new();
        let bytes = match signature::Ed25519KeyPair::generate_pkcs8(&rng) {
            Ok(byt) => byt,
            Err(_) => return None,
        };
        let input = untrusted::Input::from(&bytes);
        match signature::Ed25519KeyPair::from_pkcs8(input) {
            Ok(key) => Some(key),
            Err(_) => None,
        }
    }

    pub fn random() -> Self {
        Self {
            keypair: Signer::generate_ed25519_keypair(),
        }
    }

    /// Create signer based on in memory pkcs8 key
    ///
    /// Expects raw bytes without encoding
    pub fn from_pkcs8(pkcs8: &Vec<u8>) -> Result<Signer> {
        // get keypair
        let pkcs8_keys = signature::Ed25519KeyPair::from_pkcs8(untrusted::Input::from(&pkcs8))
            .map_err(|err| ScalpelError::ParsePk8Error.context(err))?;
        // return
        Ok(Signer {
            keypair: Some(pkcs8_keys),
        })
    }

    /// create signer with key from pkcs8 file
    ///
    /// Expects raw bytes without encoding
    pub fn from_pkcs8_file(pkcs8_file_path: &Path) -> Result<Signer> {
        // open file
        let mut pkcs8_file = OpenOptions::new()
            .read(true)
            .open(pkcs8_file_path)
            .map_err(|err| ScalpelError::OpeningError.context(err))?;

        let mut content = Vec::new();
        pkcs8_file
            .read_to_end(&mut content)
            .map_err(|err| ScalpelError::ReadingError.context(err))?;
        Self::from_pkcs8(&content)
    }

    /// get signature for bytes
    pub fn calculate_signature(&self, file: &Bytes) -> Result<ring::signature::Signature> {
        if let Some(ref keypair) = self.keypair {
            Ok(keypair.sign(&file))
        } else {
            Err(ScalpelError::KeyInitError
                .context("No key in here yet")
                .into())
        }
    }

    /// get signature of file
    pub fn calculate_signature_of_file<P>(&self, path: P) -> Result<ring::signature::Signature>
    where
        P: AsRef<Path> + Debug,
    {
        let path: &Path = path.as_ref();

        let mut file = OpenOptions::new()
            .write(true)
            .read(true)
            .open(path)
            .map_err(|err| {
                ScalpelError::OpeningError
                    .context(err)
                    .context(format!("Failed to open {:?}", path))
            })?;

        let mut content = Vec::<u8>::new();
        file.read_to_end(&mut content)
            .map_err(|err| ScalpelError::ReadingError.context(err))?;

        let content = Bytes::from(content);
        let signature = self.calculate_signature(&content)?;

        Ok(signature)
    }

    /// verify bytes with provided signature bytes
    pub fn verify<B, C>(&self, bytes_data: B, bytes_signature: C) -> Result<()>
    where
        B: Into<Bytes>,
        C: Into<Bytes>,
    {
        if let Some(ref keypair) = self.keypair {
            let bytes_data = bytes_data.into();
            let bytes_signature = bytes_signature.into();

            ring::signature::verify(
                &ring::signature::ED25519,
                untrusted::Input::from(keypair.public_key_bytes()),
                untrusted::Input::from(&bytes_data),
                untrusted::Input::from(&bytes_signature),
            )?;
            Ok(())
        } else {
            Err(ScalpelError::KeyInitError
                .context("No key in here yet")
                .into())
        }
    }

    /// verify signature in file with actual signature
    pub fn verify_file<P>(&self, path: P) -> Result<()>
    where
        P: AsRef<Path>,
    {
        let path: &Path = path.as_ref();

        let mut file = OpenOptions::new()
            .write(true)
            .read(true)
            .open(path)
            .map_err(|err| ScalpelError::OpeningError.context(err))?;

        let mut content = Vec::<u8>::new();
        file.read_to_end(&mut content)
            .map_err(|err| ScalpelError::ReadingError.context(err))?;

        if content.len() > 64 {
            let (data, signature) = content.split_at(content.len() - 64);
            self.verify(data, signature)
                .map_err(|e| ScalpelError::ContentError.context(e))?;
            Ok(())
        } else {
            Err(ScalpelError::ContentError
                .context("File to short, no signature included")
                .into())
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use concat::*;

    #[test]
    fn test_keys_pk8() {
        let signer = Signer::from_pkcs8_file(Path::new("./tmp/ed25519_keypair.pk8"))
            .expect("Failed to read pk8 keys from file");

        let signature = signer
            .calculate_signature_of_file("./tmp/signme.bin")
            .expect("Signing failed");

        append_signature(Path::new("./tmp/signme.bin"), &signature)
            .expect("Failed to append signature");

        assert!(signer
            .verify_file(Path::new("./tmp/signme-signed.bin"))
            .is_ok());
    }
}
