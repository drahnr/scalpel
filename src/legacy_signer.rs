// add external crate pem to main.rs
use pem::{Pem,parse_many};
// add this error to errors.rs
    #[fail(display = "Failed to parse Keys from .pem")]
    ParsePemError,


    /// Function from signature::signer to get Ed25519 Keypair from a .pem file
    /// read key from file and return a Signature
    pub fn read_pem(path_file: &Path) -> Result<Signer> {
        // open file
        let mut file = OpenOptions::new()
            .read(true)
            .open(path_file)
            .map_err(|err| SigningError::OpeningError.context(err))?;

        let mut content = Vec::new();
        file.read_to_end(&mut content)
            .map_err(|err| SigningError::ReadingError.context(err))?;
        // get content and tag from pem, does the base64 decoding
        // probably returns DER encoded content
        let pems : Vec<Pem> = parse_many( &content );
        // concatenat the private and public for signature::Ed25519KeyPair::from_pkcs8
        // is not valid format for from_pkcs8, not sure what it expects
        let concatenated = pems.iter().fold(Vec::<u8>::new(), |mut acc, pem| {
            println!("Tag: {}", &pem.tag);
            acc.append(&mut pem.contents.clone());
            acc
        });

        // get Ed25519 keypair
        // seed and public_key in unkown format, docs recommends using just from_pkcs8, but the pem
        // crate does not provide pkcs8 format
        let pkcs8_keys = signature::Ed25519KeyPair::from_seed_and_public_key( untrusted::Input::from(&pems[0].contents), untrusted::Input::from(&pems[1].contents))
                .map_err(|err| SigningError::ParsePemError.context("Failed to create keypair from pkcs8").context(err))?;
        
                //from_pkcs8(untrusted::Input::from(concatenated.as_slice()))
                //.map_err(|err| SigningError::ParsePemError.context("Failed to create keypair from pkcs8").context(err))?;

        Ok(Signer{ keypair: Some(pkcs8_keys) })
    } 