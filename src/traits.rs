//! Just an idea
//! 
//! Introduce traits and stuff


/// Maybe this is better, to convert 
pub trait ToBytes {
    type Error;
    pub fn to_bytes(&self, dest : mut BytesMut) -> Result<BytesMut, Self::Error>;
}


/// and the reverse
pub trait FromBytes {
    type Error;
    /// returns the remaining bytes too
    pub fn from_bytes(dest : mut BytesMut) -> Result<(Self,BytesMut), Self::Error>;
}



pub trait Ops : MergeOperation + CutOperation + FillOperation + ReplaceOperation {
}



/// Formatter in IntelHex
pub struct IntelHex {

}


impl ToBytes for IntelHex {
    type Error;
}

// TODO imple all the traits

impl Ops for IntelHex {}


/// there might be elf binaries too which 
/// are target and loader specific
/// 
/// Formatter for BareMetal target arch
pub struct BareMetalBinary {
}

impl MergeOperation for BareMetalBinary {
    type Error;
    fn merge() -> Result<Self,Self::Error> {
        //...
    }
}

impl ReplaceOperation for BareMetalBinary {
    type Error;
    fn replace() -> Result<Self,Self::Error> {
        //...
    }
}

impl FillOperation for BareMetalBinary {
    type Error;
    fn fill() -> Result<Self,Self::Error> {
        //...
    }
}

impl CutOperation for BareMetalBinary {
    type Error;
    fn cut() -> Result<Self,Self::Error> {
        //...
    }
}

// marker trade
impl Ops for BareMetalBinary {}

impl ToBytes for BareMetalBinary {
    type Error = ();
    fn to_bytes(&self, dest : mut Bytes) -> Result<BytesMut, Self::Error> {
        //...
    }
}


fn main() {
    // how we want it to be used afterwards
    let bytes = read_from_file();
    // boxing is unavoidable here
    // maybe there is a better way instead of this pseudo OOP bullshit,
    // could be smarter to do something on a per type level with generics and type trait
    // bound requirements instead of this
    let x : Box<Ops> = match magic_numerbs(bytes) {
        BinaryFormat::Elf => ...,
        BinaryFormat::IntelHex => ...,
    }

    let y = match args.ops {
        CUT => x.cut(...)?
        REPLACE => x.replace()?
    }

    // get destination fmt
    let y = y.into();
}