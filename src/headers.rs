use ureq::Request;

#[derive(Debug, Clone, PartialEq)]
pub enum Header {
    /// Normally added automatically when sending bytes.
    ContentLength(usize),
    Authorization {
        access: String,
        secret: String,
    },
    ContentType(String),
    ContentMd5(String),
    
    XAutoMakeBucket(bool),
    XCascadeDelete(bool),
    XIgnorePreexistingBucket(bool),
    XKeepOldVersion(bool),
    XMeta {
        name: String,
        value: String,
    },
    XQueueDerive(bool),
    XSizeHint(usize),
    
    Custom(String, String),
}

pub trait RequestHeaderExt {
    /// Set a header field used by Internet Archive's S3-like API.
    fn set_header(self, header: Header) -> Self;
}
impl RequestHeaderExt for Request {
    fn set_header(self, header: Header) -> Self {
        use Header::*;
        
        match header {
            ContentLength(val) => self.set("content-length", &val.to_string()),
            Authorization { access, secret } => self.set("authorization", &format!("LOW {access}:{secret}")),
            ContentType(val) => self.set("content-type", &val),
            ContentMd5(val) => self.set("content-md5", &val),
            
            XAutoMakeBucket(val) => self.set("x-amz-auto-make-bucket", &(val as u8).to_string()),
            XCascadeDelete(val) => self.set("x-archive-cascade-delete", &(val as u8).to_string()),
            XIgnorePreexistingBucket(val) => self.set("x-archive-ignore-preexisting-bucket", &(val as u8).to_string()),
            XKeepOldVersion(val) => self.set("x-archive-keep-old-version", &(val as u8).to_string()),
            XMeta { name, value } => self.set(format!("x-archive-meta-{name}").as_str(), &value),
            XQueueDerive(val) => self.set("x-archive-queue-derive", &(val as u8).to_string()),
            XSizeHint(val) => self.set("x-archive-size-hint", &val.to_string()),
            
            Custom(key, val) => self.set(key.as_str(), &val),
        }
    }
}