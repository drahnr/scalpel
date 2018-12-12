#[macro_use]
extern crate honggfuzz;

extern crate bytes;

use bytes::BytesMut;

extern crate scalpel;
use std::time::Instant;

struct FuzzData {
    start : usize,
    size : usize,
    fillpattern : scalpel::FillPattern,
}

impl honggfuzz::arbitrary::Arbitrary for FuzzData {
    fn arbitrary<U: honggfuzz::arbitrary::Unstructured + ?Sized>(u: &mut U) -> Result<Self, U::Error> {
        Ok(FuzzData {
            start : <usize as honggfuzz::arbitrary::Arbitrary>::arbitrary(u)?,
            size : <usize as honggfuzz::arbitrary::Arbitrary>::arbitrary(u)?,
            fillpattern : scalpel::FillPattern::Zero,
        })
    }
}

fn main() {
    // Here you can parse `std::env::args and
    // setup / initialize your project

    // You have full control over the loop but
    // you're supposed to call `fuzz` ad vitam aeternam

    // let mut padding = vec![0; size - replace.len()];
    // ::rand::thread_rng().try_fill(&mut padding[..])?;

    // fill 4999 bytes with the characters equiv to the number 0 to 255
    let mut source = BytesMut::with_capacity(5000);
    unsafe {
        source.set_len(5000 - 1);
    }
    let mut q: u8 = 0;
    source.iter_mut().for_each(|x: &mut u8| {
        *x = q;
        q = q.wrapping_add(1);
    });

    let start = Instant::now();
    while start.elapsed().as_secs() < 20 {
        let workinginstance : BytesMut = source.clone();

        // The fuzz macro gives an arbitrary object (see `arbitrary crate`)
        // to a closure-like block of code.
        // For performance reasons, it is recommended that you use the native type
        // `&[u8]` when possible.
        // Here, this slice will contain a "random" quantity of "random" data.

        let mut out = BytesMut::with_capacity(source.capacity());
        fuzz!(|fuzz : FuzzData| {
            let _ = scalpel::replace(
                workinginstance,
                out,
                fuzz.start,
                fuzz.size,
                fuzz.fillpattern,
            );
            }
        );
    }
}
