//! This is a synchronous (blocking) client written purely in Rust, for interfacing with [Internet Archive](https://archive.org/) APIs.
//! 
//! # API List
//! |Supported|Name|Endpoint|
//! |:-------:|----|--------|
//! | Partial | IAS3 (S3-like) ([API docs](https://archive.org/developers/ias3.html)) |`https://s3.us.archive.org/{identifier}`|
//! | No | Item Metadata ([API docs](https://archive.org/developers/metadata.html)) |`https://archive.org/metadata/{identifier}`|
//! | No | Item Views ([API docs](https://archive.org/developers/views_api.html)) |`https://be-api.us.archive.org/views/v1/short/{identifier}[,...]`|
//! | No | Item Reviews ([API docs](https://archive.org/developers/reviews.html)) |`https://archive.org/services/reviews.php`|
//! | No | Item Changes ([API docs](https://archive.org/developers/changes.html)) |`https://be-api.us.archive.org/changes/v1`|
//! | No | Item Tasks ([API docs](https://archive.org/developers/tasks.html)) |`https://archive.org/services/tasks.php`|
//! 
//! The IAS3, Metadata, Views, and Reviews APIs are accessible through the [`Item`] data type. The
//! remaining APIs are accessed via their respective module ([`changes`], and [`tasks`]).
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
//! Using async often severely increases the number of dependencies required to use a crate, and
//! increases the complexity of its development and usage.
//! 
//! For scenarios involving many IO or networking requests, such as web servers, async is definitely
//! useful in maximizing performance and throughput. Making use of the Internet Archive's APIs however,
//! should not require many simutaneous connections. Plus, throughput for expensive requests, such as
//! uploading/downloading large files, will be cumulatively limited by either the user's internet
//! service or Internet Archive. So performing multiple uploads/downloads at the same time will likely
//! not yield any significant benefit.

use std::string::ToString;
use crate::headers::Header::{XAutoMakeBucket, XKeepOldVersion, XMeta, XQueueDerive, XSizeHint};
use crate::headers::{Header, RequestHeaderExt};

pub mod changes;
pub mod headers;
pub mod tasks;

pub const DEFAULT_USER_AGENT: &'static str = "iars <https://crates.io/crates/iars>";

#[derive(Debug)]
pub enum ItemError {
    Ureq(ureq::Error),
    Forbidden(ureq::Response),
    InvalidIdentifier(String),
}
impl From<ureq::Error> for ItemError {
    fn from(value: ureq::Error) -> Self {
        match value {
            ureq::Error::Status(403, resp) => Self::Forbidden(resp),
            _ => Self::Ureq(value)
        }
    }
}


/// Container for the access and secret keys required for some actions in the Internet Archive API.
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

/// Represents a particular item on the Internet Archive.
/// 
/// An item could be a book, a song, a movie, a file or set of files, etc. Each item uses an identifier
/// which is unique across the entire Internet Archive. Identifiers must follow a set of rules to
/// ensure they are valid. [`validate_identifier`] can be used to determine if an identifier is valid.
/// 
/// Some actions involving an item may require authentication by making use of an access key and a
/// secret key. Users can get these API keys from <https://archive.org/account/s3.php> and are provided
/// to this representation using the [`Credentials`] type.
#[derive(Debug, Clone, PartialEq)]
pub struct Item {
    identifier: String,
    credentials: Option<Credentials>,
    keep_old_versions: bool,
    auto_make_bucket: bool,
    use_test_collection: bool,
    useragent: String,
}
impl Item {
    /// Creates a new reference to an item on the Internet Archive.
    /// 
    /// Item identifiers _should_ be validated by the caller using [`validate_identifier`]. While
    /// creation of the [`Item`] object will not fail, queries making use of an invalid identifier
    /// will return a [`ItemError::InvalidIdentifier`] error.
    /// 
    /// Some actions on this item may require authentication. [`Credentials`] can be provided using
    /// [`Self::with_credentials`].
    pub fn new(ident: &str) -> Self {
        Self {
            identifier: ident.to_string(),
            credentials: None,
            keep_old_versions: false,
            auto_make_bucket: true,
            use_test_collection: false,
            useragent: DEFAULT_USER_AGENT.to_string(),
        }
    }
    
    /// Provide authentication credentials to be used with all queries.
    /// 
    /// Many operations on the Internet Archive, such as uploading or deleting files, require
    /// authentication using both an access key and a secret key. These keys can be found
    /// [here](https://archive.org/account/s3.php).
    /// 
    /// Operations where valid keys are not provided, will result in a 403 Forbidden error.
    pub fn with_credentials(mut self, credentials: Option<Credentials>) -> Self {
        self.credentials = credentials;
        
        self
    }
    
    /// Configures the User-Agent string provided in all API queries.
    /// 
    /// If `None` or if the string is empty, a [default][`DEFAULT_USER_AGENT`] will be used.
    /// 
    /// A User-Agent string may also be provided. If no user agent is given, or the string is empty,
    /// a [default][`DEFAULT_USER_AGENT`] will be used.
    pub fn with_useragent(mut self, useragent: Option<String>) -> Self {
        if useragent.is_none() || useragent.as_ref().unwrap().is_empty() {
            self.useragent = DEFAULT_USER_AGENT.to_string();
        } else {
            self.useragent = useragent.unwrap();
        }
        
        self
    }
    
    /// Configures whether or not file creation or deletion operations should backup the old version
    /// of the file.
    /// 
    /// This is false (disabled) by default.
    /// 
    /// The old version of the file will be moved by the Internet Archive into `history/files/{filename}.~N~`.
    pub fn with_keep_old_versions(mut self, keep_old_versions: bool) -> Self {
        self.keep_old_versions = keep_old_versions;
        
        self
    }
    
    /// Configures whether or not the Internet Archive item will be created automatically when uploading
    /// a file, if the item doesn't already exist.
    /// 
    /// This is true (enabled) by default.
    pub fn with_auto_make(mut self, auto_make_bucket: bool) -> Self {
        self.auto_make_bucket = auto_make_bucket;
        
        self
    }
    
    /// Uploads a file to this item.
    /// 
    /// Normally, file uploads will cause the Internet Archive to queue a "derive" process on the item.
    /// This process produces secondary files to improve usability of the uploaded data. Setting the
    /// `derive` argument to `false` will prevent this process.
    /// 
    /// Metadata can be provided as a slice of (key, value) tuples for newly created items.
    /// **If the Internet Archive item already exists or is not automatically created upon upload,
    /// this metadata will be silently discarded.**
    /// 
    /// The `filepath` denotes both the filename and the path within the item where the file should
    /// be stored.
    /// 
    /// Uploaded files may not be immediated available on Internet Archive, depending on how busy
    /// the site is when the file is uploaded.
    /// 
    /// # Example
    /// ```rust
    /// use iars::{Credentials, Item};
    ///
    /// let item = Item::new("test_item")
    ///     .with_credentials(Some(Credentials::new("abcdefghijklmnop", "1234567890123456")));
    /// 
    /// item.upload_file(true, &[("collection", "test_collection")], "a_directory/myfile.txt", "Hello World!".as_bytes())?;
    /// ```
    /// If successful, file will be viewable at `https://archive.org/download/test_item/a_directory/myfile.txt`.
    /// 
    /// If this upload creates a new Internet Archive item, then the metadata `<collection>test_collection</collection>`
    /// will be included in the item's metadata. 
    pub fn upload_file(&self, derive: bool, initial_meta: &[(&str, &str)], filepath: &str, data: &[u8]) -> Result<ureq::Response, ItemError> {
        let mut req = ureq::put(&format!("https://s3.us.archive.org/{}/{filepath}", self.identifier))
            .set("user-agent", &self.useragent)
            .set_header(XKeepOldVersion(self.keep_old_versions))
            .set_header(XAutoMakeBucket(self.auto_make_bucket))
            .set_header(XQueueDerive(derive))
            .set_header(XSizeHint(data.len()));
        
        for (key, val) in initial_meta {
            req = req.set_header(XMeta { name: key.to_string(), value: val.to_string() });
        }
        
        if let Some(creds) = self.credentials.as_ref() {
            req = req.set_header(creds.into());
        }
        
        println!("REQUEST:");
        for header in req.header_names() {
            if let Some(value) = req.header(&header) {
                println!("    {header}: {value}");
            }
        }
        
        Ok(req.send_bytes(data)?)
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