//! This is a synchronous (blocking) client written purely in Rust, for interfacing with [Internet Archive](https://archive.org/) APIs.
//! 
//! # API List
//! |Supported|Name|Endpoint|
//! |:-------:|:--:|--------|
//! | Yes | IAS3 (S3-like) ([API docs](https://archive.org/developers/ias3.html)) |`https://s3.us.archive.org/{identifier}`|
//! | Read-only | Metadata ([API docs](https://archive.org/developers/metadata.html)) |`https://archive.org/metadata/{identifier}`|
//! | No | Views ([API docs](https://archive.org/developers/views_api.html)) |`https://be-api.us.archive.org/views/v1/short/{identifier}[,...]`|
//! | No | Reviews ([API docs](https://archive.org/developers/reviews.html)) |`https://archive.org/services/reviews.php`|
//! | No | Changes ([API docs](https://archive.org/developers/changes.html)) |`https://be-api.us.archive.org/changes/v1`|
//! | Partial | Tasks ([API docs](https://archive.org/developers/tasks.html)) |`https://archive.org/services/tasks.php`|
//! 
//! The IAS3, Metadata, Views, and Reviews APIs are accessible through the [`Item`] data type. The
//! remaining APIs are accessed via their respective module ([`changes`], and [`tasks`]).
//! 
//! # Authentication
//! Generally, any operations that modify or upload files to the Internet Archive will require authentication.
//! Access to hidden archive items will also require that you are the owner of the item.
//! 
//! Cookies are technically accepted by the Internet Archive for authentication, however they are intended
//! only for use in a browser environment. As this crate is intended for programmatic access of their APIs,
//! only key authentication is available.
//! 
//! To acquire your own S3-like keys, log into <https://archive.org/> and then proceed to the [API Key page](https://archive.org/account/s3.php).
//! 
//! # S3-like API
//! Also refered to the `ias3`, this API is responsible for providing read and write access to the
//! files that make up an item on the Internet Archive. It is refered to as S3-like because each item
//! is mapped to an "S3 bucket". You _do not_ need to know what S3 is, or how it works, in order to use
//! this API.
//! 
//! Critically, using an existing S3 crate, such as `rust-s3`, doesn't seem compatible with this API
//! (authentication doesn't work). The API differs in [several ways](https://archive.org/developers/ias3.html#how-this-is-different-from-normal-s3),
//! and includes numerous custom HTTP headers which affect the behavior of each request.
//! 
//! # Why not async?
//! Using async often severely increases the number of dependencies required to use a crate, while
//! simultaneously increasing the complexity of its development and usage.
//! 
//! For scenarios involving many IO or networking requests, such as web servers, async is definitely
//! useful in maximizing performance. However, using the Internet Archive's APIs as a client is unlikely
//! to benefit from async.
//! 
//! As such, all HTTP requests are performed using [ureq] which subscribes to [a similar mindset][ureq#blocking-io-for-simplicity].

use crate::headers::Header;

pub mod changes;
pub mod headers;
pub mod item;
pub mod tasks;

pub use item::{Item, ItemError};

/// `User-Agent` string used by default for all API requests.
pub const DEFAULT_USER_AGENT: &'static str = "iars <https://crates.io/crates/iars>";


/// Container for authentication keys required by portions of the Internet Archive API.
/// 
/// Users can get these API keys from <https://archive.org/account/s3.php>.
#[derive(Debug, Clone, PartialEq)]
pub struct Credentials {
    pub access: String,
    pub secret: String,
}
impl Credentials {
    /// Creates a new [`Credentials`] instance using an access key and a secret key.
    pub fn new(access: &str, secret: &str) -> Self {
        Self {
            access: access.into(),
            secret: secret.into(),
        }
    }
    
    /// Attempts to load credentials from environmental variables.
    /// 
    /// Variable names are the same as used in the `aws-creds` crate (since the Internet Archive uses
    /// an "S3-like" API.)
    /// 
    /// * Access Key: `AWS_ACCESS_KEY_ID`
    /// * Secret Key: `AWS_SECRET_ACCESS_KEY`
    /// 
    /// `None` will be returned if either of the env variables are not set, empty, or some other [`std::env::VarError`] is encountered.
    pub fn try_from_env() -> Option<Self> {
        let access = std::env::var("AWS_ACCESS_KEY_ID").ok()?;
        let secret = std::env::var("AWS_SECRET_ACCESS_KEY").ok()?;
        
        if access.is_empty() || secret.is_empty() {
            return None;
        }
        
        Some(Self {
            access,
            secret,
        })
    }
}
impl From<&Credentials> for Header {
    fn from(value: &Credentials) -> Self {
        Header::Authorization {
            access: value.access.clone(),
            secret: value.secret.clone(),
        }
    }
}


/// Checks if the identifier string is valid.
/// 
/// Identifiers are limited to only ASCII characters, underscores, dashes, and/or periods. The first
/// character must be alphanumeric. Identifiers also must not be larger than 100 characters in length.
/// 
/// Returns false if any of these requirements are not upheld.
pub fn validate_identifier(ident: &str) -> bool {
    if ident.is_empty() || ident.len() > 100 {
        return false;
    }
    
    let mut chars = ident.chars();
    if !chars.next().unwrap().is_ascii_alphanumeric() {
        return false;
    }
    
    for c in chars {
        if !(c.is_ascii_alphanumeric() || c == '_' || c == '-' || c == '.') {
            return false;
        }
    }
    
    true
}