[![License: MIT](https://img.shields.io/badge/License-MIT-blue?style=flat-square)](MIT)
[![License: APACHE2.0](https://img.shields.io/badge/License-APACHE2.0-blue?style=flat-square)](APACHE2.0)
[![Crates.io](https://img.shields.io/crates/v/iars?style=flat-square)](https://crates.io/crates/iars)
[![Documentation](https://img.shields.io/docsrs/iars?style=flat-square)](https://docs.rs/iars)

### Description
`iars` is a synchronous (blocking) client written purely in Rust, for interfacing with [Internet Archive](https://archive.org/) APIs.

Refer to the [docs](https://docs.rs/iars) for which APIs are currently supported.

### Getting Started
Add the `iars` crate to your project's Cargo.toml file:
```TOML
[dependencies]
iars = "0.1"
```
Most operations are available via the `Item` data structure. Here's an example of uploading a file
to an Internet Archive "item":
```Rust
use iars::{Credentials, Item};

fn main() {
    // Authentication keys are required for uploading files.
    let creds = Credentials::new("abcdefghijklmnop", "1234567890123456");
    
    let item = Item::new("test_identifier")
        .with_credentials(Some(creds));
    
    item.upload_file(true, &[("collection", "test_collection")], "a_directory/myfile.txt", "Hello World!".as_bytes()).unwrap();
}
```

#### Authentication
Some of the Internet Archive's API queries require authentication using API keys. To get your own API keys, create or log into an
account on https://archive.org/. Then go to https://archive.org/account/s3.php.

### License
The contents of this repository are dual-licensed under the _MIT OR Apache
2.0_ License. That means you can chose either the MIT licence or the
Apache-2.0 licence when you re-use this code. See the `MIT` or `APACHE2.0` files for more
information on each specific licence.

Any submissions to this project (e.g. as Pull Requests) must be made available
under these terms.