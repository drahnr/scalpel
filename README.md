# scalpel

A scalpel and stitch tool for binaries. Maybe also a signing tool, maybe.


### Snip around, stich up and/or sign binaries

This is mostly used for the case where parts of the binary need to be extracted or replaced.

#### Use Cases

* cut firmware into pieces from an all-in-one blob
* stitch firmware pieces together such as bootloader and application
* [alpha] sign firmware for authenticity

```
scalpel --start 0 --size 4096 --output winc_booloader.bin xdk-asf-3.36.2/common/components/wifi/winc1500/firmware_update_project/firmware/firmware/m2m_aio_3a0.bin
scalpel --start 40960 --size 241664 --output winc_part_A.bin xdk-asf-3.36.2/common/components/wifi/winc1500/firmware_update_project/firmware/firmware/m2m_aio_3a0.bin
scalpel --start 282624 --size 241664 --output winc_part_B.bin xdk-asf-3.36.2/common/components/wifi/winc1500/firmware_update_project/firmware/firmware/m2m_aio_3a0.bin
```

#### Features 

- [x] cut off a binary at specific start and end/size
- [x] Add signature verification and appendix features (using preferably [ring] and [webpki] or as an alternative [sodiumoxide] (linking it statically))
- [ ] Handle endianness of checksums properly
- [ ] Replace parts (i.e. cert files or non volatile memory and/or sections) (with resigning if necessary)
- [ ] Allow hexadecimal input
- [x] Allow multipile input scales (K = 1000, Ki = 1024, M = 1e6, Mi = 1024*1024, ...)
- [ ] Add verifier option for alignment to given sector/page size


#### Common / Hints

- You need th extracted binary as include? Use `xxd -i sliced.bin > sliced_binary.h` to create a header file out of the result.

- Convert RSA keys in .pem format to pkcs8 format via openssl (see `ring` doc [doc-ring] ), `openssl` supports Ed25519 algorithm currently only on `master` branch

    ```
    openssl pkcs8 -toppk8 -nocrypt -outform der -in [key.pem] > [pkcs8_key.pk8]
    ```

- Generate valid Ed25519 Keypair use small tool from `ring` author:

    ```
    cargo install kt
    kt generate ed25519 --out=FILE
    ```



[ring]: https://crates.io/crates/ring
[doc-ring]: https://docs.rs/ring/0.13.0-alpha/ring/signature/struct.RSAKeyPair.html
