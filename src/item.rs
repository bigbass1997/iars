//! Access and creation of archive items and metadata.
//! 
//! On the Internet Archive, each individual "archive" is known as an item, identified using a unique
//! [string][crate::validate_identifier]. Archive items can contain one or more files and directories,
//! as well as associated metadata.
//! 
//! In `iars` an [`Item`] represents one of these archives. After instantiating an `Item`, the caller
//! can perform operations relevant to that particular item, such as retrieving metadata or uploading
//! new files.
//! 
//! Creating a new archive on IA is as simple as creating an `Item` using an unused identifier, and
//! [uploading a file][Item::upload_file] to it.

use std::collections::HashMap;
use std::io::{Read, Write};
use std::string::ToString;
use serde::Deserialize;
use crate::{Credentials, DEFAULT_USER_AGENT, validate_identifier};
use crate::headers::Header::{XAutoMakeBucket, XKeepOldVersion, XMeta, XQueueDerive, XSizeHint};
use crate::headers::RequestHeaderExt;

#[derive(Debug)]
pub enum ItemError {
    /// An error while performing [`std::io`] operations.
    Io(std::io::Error),
    
    /// An error while processing a [`ureq`] request.
    Ureq(ureq::Error),
    
    /// An error while attempting to parse XML.
    XmlParseFailed(serde_xml_rs::Error),
    
    /// A [`ureq`] request was successful, but returned a 403 Forbidden error code.
    /// 
    /// This is usually caused by not having valid [authentication][`Item`].
    Forbidden(ureq::Response),
    
    /// Item identifier is invalid according to [`validate_identifier`].
    InvalidIdentifier(String),
}
impl From<std::io::Error> for ItemError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}
impl From<ureq::Error> for ItemError {
    fn from(value: ureq::Error) -> Self {
        match value {
            ureq::Error::Status(403, resp) => Self::Forbidden(resp),
            _ => Self::Ureq(value)
        }
    }
}
impl From<serde_xml_rs::Error> for ItemError {
    fn from(value: serde_xml_rs::Error) -> Self {
        Self::XmlParseFailed(value)
    }
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(rename_all = "PascalCase")]
struct ListBucketResult {
    contents: Vec<FileEntry>
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct FileEntry {
    #[serde(rename = "Key")]
    pub path: String,
    #[serde(rename = "LastModified")]
    pub last_modified: String,
    #[serde(rename = "Size")]
    pub len: usize,
}

/// Contains the metadata for an item and additional meta-metadata.
#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct MetadataResponse {
    /// UNIX epoch timestamp of when this [metadata record][`MetadataRecord`] was created.
    /// 
    /// To determine the item's creation time, check for `"addeddate"` in the [`field@metadata`] field instead.
    created: isize,
    
    /// Pseudo-random value used internally by the Metadata API.
    uniq: isize,
    
    /// URL of the primary data server the item is stored on.
    /// 
    /// # Caution
    /// The item may be moved to another server at any time. Constructing URLs using this
    /// value is **not recommended**.
    /// 
    /// For download URLs, use `https://archive.org/download/{identifier}/{relative_file_path}` instead.
    d1: String,
    
    /// URL of the secondary (backup) data server the item is stored on. May be `None` or empty in rare cases.
    /// 
    /// # Caution
    /// The item may be moved to another server at any time. Constructing URLs using this
    /// value is **not recommended**.
    /// 
    /// For download URLs, use `https://archive.org/download/{identifier}/{relative_file_path}` instead.
    d2: Option<String>,
    
    /// Absolute path of the item located on both data nodes.
    /// 
    /// # Caution
    /// The item may be moved to another location at any time. Constructing URLs using this
    /// value is **not recommended**.
    /// 
    /// For download URLs, use `https://archive.org/download/{identifier}/{relative_file_path}` instead.
    dir: String,
    
    /// Preferred server (domain only) for reading the item's contents.
    server: String,
    
    /// List of data servers that are currently available for accessing the item's contents.
    workable_servers: Vec<String>,
    
    /// True if one or both of the primary and backup servers are unavailable/inaccessible.
    #[serde(default)]
    servers_unavailable: bool,
    
    /// The metadata of the item itself.
    /// 
    /// This data is also stored in the `<identifier>_meta.xml` file within the item. Keep in mind,
    /// recent metadata changes may not have yet been written to disk, but will be available
    /// in this field.
    /// 
    /// Metadata is usually made up of string key-value pairs, but some keys may correspond to a
    /// list of values.
    metadata: HashMap<String, serde_json::Value>,
    
    /// Total size (bytes) of all files within the item.
    item_size: usize,
    
    /// UNIX epoch timestamp of when the item was last modified.
    item_last_updated: isize,
    
    /// Total number of files in the item.
    files_count: usize,
    
    /// The metadata of each file within the item.
    /// 
    /// This data is also stored in the `<identifier>_files.xml` file. Keep in mind,
    /// recent metadata changes may not have yet been written to disk, but will be available
    /// in this field.
    /// 
    /// # Common Keys
    /// * `name`: Filename
    /// * `crc32`: CRC32 checksum
    /// * `md5`: MD5 checksum
    /// * `sha1`: SHA1 checksum
    /// * `format`: Textual name of the file's format (e.g. "MPEG4", "Thumbnail")
    /// * `mtime`: UNIX epoch timestamp of when the file was last modified
    /// * `size`: Size of the file in bytes
    /// * `source`: Where the file came from (e.g. "original", "derivative")
    files: Vec<HashMap<String, String>>,
    
    /// True if one or more catalog [tasks][`crate::tasks`] are queued or running.
    #[serde(default)]
    pending_tasks: bool,
    
    /// True if one or more catalog [tasks][`crate::tasks`] were halted due to an error.
    #[serde(default)]
    has_redrow: bool,
    
    //TODO: tasks: ?, // List of queued tasks https://archive.org/developers/md-record.html#catalog-fields
    
    //TODO: reviews: ?, // List of reviews given by IA users https://archive.org/developers/md-record.html#reviews-field
    
    /// True if the item is darked (hidden) and unavailable.
    #[serde(default)]
    is_dark: bool,
    
    /// True if the item is not ready for downloading yet.
    #[serde(default)]
    nodownload: bool,
    
    /// True if the item is a collection.
    #[serde(default)]
    is_collection: bool
    
    //TODO: simplelists: SimpleLists, // Holds the SimpleLists structure for the item https://archive.org/developers/simplelists.html
    
    //TODO: User JSON fields https://archive.org/developers/md-record.html#user-json-fields
}

/// Represents a particular item on the Internet Archive.
/// 
/// An item could be a book, a song, a movie, a file or set of files, etc. Each item uses an identifier
/// which is unique across the entire Internet Archive. Identifiers must follow a set of rules to
/// ensure they are valid. [`validate_identifier`] can be used to determine if an identifier is valid.
/// 
/// # Authentication
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
    /// Some actions on this item may require authentication. [`Credentials`] can be provided using
    /// [`Self::with_credentials`].
    /// 
    /// # Errors
    /// If [`validate_identifier`] determines the provided identifier is invalid, an
    /// [`ItemError::InvalidIdentifier`] error will be returned.
    pub fn new(ident: &str) -> Result<Self, ItemError> {
        let identifier = ident.to_string();
        if !validate_identifier(&identifier) {
            return Err(ItemError::InvalidIdentifier(identifier))
        }
        
        Ok(Self {
            identifier,
            credentials: None,
            keep_old_versions: false,
            auto_make_bucket: true,
            use_test_collection: false,
            useragent: DEFAULT_USER_AGENT.to_string(),
        })
    }
    
    /// Provide authentication credentials to be used with all queries for this item.
    /// 
    /// Many operations on the Internet Archive, such as uploading or deleting files, require
    /// authentication using both an access key and a secret key. These keys can be found
    /// [here](https://archive.org/account/s3.php).
    /// 
    /// Operations that require authentication but where none are provided, or when the keys are invalid,
    /// will result in a 403 Forbidden error.
    pub fn with_credentials(mut self, credentials: Option<Credentials>) -> Self {
        self.credentials = credentials;
        
        self
    }
    
    /// Configures the User-Agent string provided in all API queries for this item.
    /// 
    /// If `None` or if the string is empty, a [default][`DEFAULT_USER_AGENT`] will be used.
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
    /// After uploads are completed, the files may not be immediately available on Internet Archive.
    /// Use the [tasks][`crate::tasks`] module to check the status of the uploaded files.
    /// 
    /// # Derivation
    /// Normally, file uploads will cause the Internet Archive to queue a "derive" process on the item.
    /// This process produces secondary files to improve usability of the uploaded data. Setting the
    /// `derive` argument to `false` will prevent this process.
    /// 
    /// # Metadata
    /// Item metadata can be provided in key-value pairs. **If the Internet
    /// Archive item already exists, or is not [automatically created][`Item::with_auto_make`],
    /// this metadata will be silently discarded.**
    /// 
    /// Use [TODO] to add metadata to existing items.
    /// 
    /// # Data Transfer
    /// The data is read using any [reader][`Read`] implementation. However, the `size` (number of
    /// bytes to be transfered) **must** be known before the upload begins. The Internet Archive
    /// _requires_ a `Content-Length` and this length _must_ be accurate.
    /// 
    /// Sizes larger than what the `reader` can provide will stall the upload. Also, no more bytes
    /// than the specified size will be transfered (meaning if the caller wishes to upload "Hello World!"
    /// but provides a size of 5, only "Hello" will be uploaded).
    /// 
    /// # Example
    /// ```rust,no_run
    /// use iars::{Credentials, Item};
    ///
    /// let item = Item::new("test_item")
    ///     .with_credentials(Some(Credentials::new("abcdefghijklmnop", "1234567890123456")));
    /// 
    /// let data = "Hello World!".as_bytes();
    /// 
    /// item.upload_file(true, &[("foo", "bar")], "a_directory/myfile.txt", data, data.len())?;
    /// # Ok::<(), iars::ItemError>(())
    /// ```
    /// If successful, the file will be viewable at `https://archive.org/download/test_item/a_directory/myfile.txt`,
    /// and if the archive item didn't already exist, its metadata will include `foo: "bar"`.
    /// 
    /// # Errors
    /// Possibly returns [`ItemError::Ureq`] if a [`ureq::Error`] is encountered while uploading.
    pub fn upload_file(&self, derive: bool, initial_meta: &[(&str, &str)], filepath: &str, reader: impl Read, size: usize) -> Result<ureq::Response, ItemError> {
        let mut req = ureq::put(&format!("https://s3.us.archive.org/{}/{filepath}", self.identifier))
            .set("user-agent", &self.useragent)
            .set_header(XKeepOldVersion(self.keep_old_versions))
            .set_header(XAutoMakeBucket(self.auto_make_bucket))
            .set_header(XQueueDerive(derive))
            .set_header(XSizeHint(size))
            .set("content-length", &size.to_string());
        
        for (key, val) in initial_meta {
            req = req.set_header(XMeta { name: key.to_string(), value: val.to_string() });
        }
        
        if let Some(creds) = self.credentials.as_ref() {
            req = req.set_header(creds.into());
        }
        
        Ok(req.send(reader)?)
    }
    
    /// Retrieves a list of all files contained in this item.
    /// 
    /// # Errors
    /// Possibly returns [`ItemError::Ureq`] if a [`ureq::Error`] is encountered while downloading
    /// the list of files (an XML string).
    /// 
    /// If the query succeeds but the response cannot be parsed, an [`ItemError::XmlParseFailed`]
    /// is returned.
    /// 
    /// # Panics
    /// Upon requesting the file list, if the `Content-Length` of the response is larger than 1 GiB,
    /// this method will panic. Please open a Github issue if this is a concern for your use-case.
    pub fn list(&self) -> Result<Vec<FileEntry>, ItemError> {
        let mut req = ureq::get(&format!("https://s3.us.archive.org/{}", self.identifier))
            .set("user-agent", &self.useragent);
        
        if let Some(creds) = self.credentials.as_ref() {
            req = req.set_header(creds.into());
        }
        
        let resp = req.call()?;
        
        const MAX_LEN: usize = 1 * 1024 * 1024 * 1024; // 1 GiB
        let len: usize = resp
            .header("content-length")
            .unwrap_or("")
            .parse()
            .unwrap_or(MAX_LEN);
        
        if len > MAX_LEN {
            todo!("Response body is over size limit of {MAX_LEN} bytes!");
        }
        
        let result: ListBucketResult = serde_xml_rs::from_reader(resp.into_reader())?;
        
        Ok(result.contents)
    }
    
    /// Downloads a file from this item.
    /// 
    /// The `filepath` corresponds to the location of the file within the item. Use [`Item::list`] to
    /// get a list of all available files in the item.
    /// 
    /// The data will be streamed into the `writer` (via [`std::io::copy`]). This method does _not_
    /// provide any size restictions or safeguards on downloads. If the `writer` is resizable and stores
    /// data in system memory (e.g. [`Vec`]), be sure the file is not larger than available memory or
    /// else use another [writer][`Write`] implementation.
    /// 
    /// On success, the number of bytes written (size of the file) is returned.
    /// 
    /// # Errors
    /// Possibly returns [`ItemError::Ureq`] if a [`ureq::Error`] is encountered while downloading.
    /// 
    /// If any [I/O errors][`std::io::Error`] occur while transfering data into the `writer`,
    /// an [`ItemError::Io`] is returned.
    /// 
    /// # Example
    /// ```rust,no_run
    /// use std::fs::File;
    /// use iars::Item;
    ///
    /// let item = Item::new("test_item");
    ///
    /// let mut file = File::create("download.txt")?;
    /// item.download_file("path/to/archived/file.txt", &mut file)?;
    /// # Ok::<(), iars::ItemError>(())
    /// ```
    pub fn download_file(&self, filepath: &str, mut writer: impl Write) -> Result<u64, ItemError> {
        let mut req = ureq::get(&format!("https://archive.org/download/{}/{filepath}", self.identifier))
            .set("user-agent", &self.useragent);
        
        if let Some(creds) = self.credentials.as_ref() {
            req = req.set_header(creds.into());
        }
        
        let resp = req.call()?;
        
        Ok(std::io::copy(&mut resp.into_reader(), &mut writer)?)
    }
    
    /// Retrieves the item's metadata.
    /// 
    /// Any recent changes submitted via the Metadata API will be present in the response, even if
    /// the changes have not been written to disk yet.
    pub fn metadata(&self) -> Result<MetadataResponse, ItemError> {
        let mut req = ureq::get(&format!("https://archive.org/metadata/{}", self.identifier))
            .set("user-agent", &self.useragent);
        
        if let Some(creds) = self.credentials.as_ref() {
            req = req.set_header(creds.into());
        }
        
        let resp = req.call()?;
        
        const MAX_LEN: usize = 1 * 1024 * 1024 * 1024; // 1 GiB
        let len: usize = resp
            .header("content-length")
            .unwrap_or("")
            .parse()
            .unwrap_or(MAX_LEN);
        
        if len > MAX_LEN {
            todo!("Response body is over size limit of {MAX_LEN} bytes!");
        }
        
        Ok(resp.into_json()?)
    }
}