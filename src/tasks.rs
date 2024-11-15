//! Search and submission utilities for tasks.
//! 
//! Tasks are the underlying operations of the Internet Archive. There are a collection of task [commands][`Command`]
//! that represent each distinct operation a task might perform. Most tasks are queued automatically after certain
//! actions, such as uploading a file to an [Item][`crate::Item`] or modifying an item's metadata.
//! 
//! The [Tasks API](https://archive.org/developers/tasks.html) provides three utilities:
//! * [Searching tasks][`search()`] based on some criteria.
//! * [Retrieving a log][`log()`] of a task's activities.
//! * [Submitting][`submit()`] new tasks to the queue.

use std::collections::HashMap;
use std::fmt;
use std::fmt::Formatter;
use serde::Deserialize;
use crate::{Credentials, DEFAULT_USER_AGENT};
use crate::headers::RequestHeaderExt;

pub mod search;
pub mod submit;

/// Creates a new task [search request][`search::Request`].
pub fn search() -> search::Request {
    search::Request::new()
}

/// Retrieves the log for an individual task.
/// 
/// These logs are plaintext strings produced by Internet Archive's servers as they process a task.
/// 
/// # Authentication
/// Task logs are only available to:
/// * the owner of the item the task is associated with, or
/// * users with privileged access
pub fn log(task_id: usize, creds: &Credentials, useragent: Option<String>) -> Result<ureq::Response, ureq::Error> {
    ureq::get("https://catalogd.archive.org/services/tasks.php")
        .query("task_log", &task_id.to_string())
        .set("user-agent", &useragent
            .and_then(|s| if s.is_empty() { None } else { Some(s) })
            .unwrap_or(DEFAULT_USER_AGENT.into())
        )
        .set_header(creds.into())
        .call()
}

/// Creates a new task [submission request][`submit::Request`].
pub fn submit() -> submit::Request {
    submit::Request::new()
}

#[derive(Debug)]
pub enum TaskError {
    /// An error while performing [`std::io`] operations.
    Io(std::io::Error),
    
    /// An error while processing a [`ureq`] request.
    Ureq(ureq::Error),
    
    /// A [`ureq`] request was successful, but returned a 403 Forbidden error code.
    /// 
    /// This is usually caused by not having valid [authentication][`crate::Credentials`].
    Forbidden(ureq::Response),
}
impl From<std::io::Error> for TaskError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}
impl From<ureq::Error> for TaskError {
    fn from(value: ureq::Error) -> Self {
        match value {
            ureq::Error::Status(403, resp) => Self::Forbidden(resp),
            _ => Self::Ureq(value)
        }
    }
}

/// Task commands available on the Internet Archive.
#[derive(Debug, Clone, PartialEq)]
pub enum Command {
    /// Unknown use.
    Archive,
    
    /// Operations related to books. Each operation is a numeral associated with some argument.
    /// 
    /// Available operations are unknown.
    BookOp {
        operations: HashMap<usize, String>,
    },
    
    /// Schedules a task to backup the "primary" copy of the item to its "secondary" server.
    /// 
    /// Usually unnecessary, as all tasks will automatically perform this backup once each finishes.
    Bup,
    
    /// Deletes the item and all of its files.
    /// 
    /// **This cannot be reversed!** Once deleted, it cannot be restored.
    Delete,
    
    /// Performs a derive on the item.
    /// 
    /// The derive task is not well documented and may perform a wide varienty of different operations depending on
    /// metadata already found in the item and what kinds of files have been uploaded.
    /// 
    /// This task may take a long time to complete.
    Derive {
        /// Specifies previously-derived files to be removed before the derive task is performed. Wildcards permitted using `*`.
        /// 
        /// Any files originally uploaded to the item will _not_ be deleted even if their name matches this argument.
        /// 
        /// # Examples:
        /// * `*` removes all derived files in the root directory
        /// * `*.jpg` removes all derived files in the root directory that end with `.jpg`
        /// * `{*.gif,*thumbs/*.jpg}` removes all derived GIFs and thumbnails
        remove_derived: String,
    },
    
    /// A miscellaneous operation, usually to correct an issue. Valid arguments are unknown.
    Fixer {
        args: HashMap<String, String>
    },
    
    /// Darking an item makes it unavailable to any user, including the item owner and the Internet Archive's internal
    /// subsystems (e.g. its Metadata API and search engine).
    MakeDark {
        /// A reasonable explanation for why the item is being darked.
        comment: String,
    },
    
    /// Undarking an item makes a previously [darked][`Command::MakeDark`] item available to users again.
    MakeUndark {
        /// A reasonable explanation for why the item is being undarked.
        comment: String,
    },
    
    /// Unknown use.
    ModifyXml,
    
    /// Attempts to rename the item's identifier.
    /// 
    /// If the new identifier with an existing item, a `409 Conflict` error will be returned.
    Rename {
        new_identifier: String,
    },
    
    /// Specifies a custom command constructed by the caller.
    Custom {
        /// Name of the command (e.g. `derive.php`).
        /// 
        /// Wildcards are permitted using any number of `*` or `%`.
        name: String,
        
        /// Optional key-value arguments to be included with command payload.
        args: HashMap<String, String>,
    },
}
impl Command {
    /// Returns the name of the command.
    pub fn name(&self) -> &str {
        use Command::*;
        match self {
            Archive => "archive.php",
            BookOp { .. } => "book_op.php",
            Bup => "bup.php",
            Delete => "delete.php",
            Derive { .. } => "derive.php",
            Fixer { .. } => "fixer.php",
            MakeDark { .. } => "make_dark.php",
            MakeUndark { .. } => "make_undark.php",
            ModifyXml => "modify_xml.php",
            Rename { .. } => "rename.php",
            Custom { name, .. } => name,
        }
    }
    
    /// Creates the argument list for [task submission requests][`submit::Request`].
    pub fn args(&self) -> HashMap<String, String> {
        #[inline(always)]
        fn pair(key: &str, val: &str) -> HashMap<String, String> {
            [(key.to_string(), val.to_string())].into()
        }
        
        use Command::*;
        match self {
            Archive => HashMap::new(), // TODO: No documentation; need to research further
            BookOp { operations } => operations
                .iter()
                .map(|(key, val)| (format!("op{key}"), val.clone()))
                .collect(),
            Bup => HashMap::new(),
            Delete => HashMap::new(),
            Derive { remove_derived } => pair("remove_derived", remove_derived),
            Fixer { args } => args.clone(),
            MakeDark { comment } => pair("comment", comment),
            MakeUndark { comment } => pair("comment", comment),
            ModifyXml => HashMap::new(), // TODO: No documentation; need to research further
            Rename { new_identifier } => pair("new_identifier", new_identifier),
            Custom { args, .. } => args.clone(),
        }
    }
}

/// The current status of a catalogued task.
/// 
/// See also: [API Docs](https://archive.org/developers/tasks.html#wait-admin-and-run-states)
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub enum Status {
    /// Task is queued
    /// ```text
    /// color: green
    /// wait_admin: 0
    /// ```
    #[serde(rename="queued")]
    Queued,
    
    /// Task is running
    /// ```text
    /// color: blue
    /// wait_admin: 1
    /// ```
    #[serde(rename="running")]
    Running,
    
    /// Task has thrown an error
    /// ```text
    /// color: red
    /// wait_admin: 2
    /// ```
    #[serde(rename="error")]
    Error,
    
    /// Task is currently paused
    /// ```text
    /// color: brown
    /// wait_admin: 9
    /// ```
    #[serde(rename="paused")]
    Paused,
}
impl fmt::Display for Status {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        use Status::*;
        write!(f, "{}", match self {
            Queued => 0,
            Running => 1,
            Error => 2,
            Paused => 9,
        })
    }
}
impl Status {
    /// Returns the color associated with this status.
    pub fn color(&self) -> String {
        use Status::*;
        match self {
            Queued => "green",
            Running => "blue",
            Error => "red",
            Paused => "brown",
        }.to_string()
    }
    
    /// Returns the `wait_admin` value associated with this status.
    pub fn wait_admin(&self) -> usize {
        use Status::*;
        match self {
            Queued => 0,
            Running => 1,
            Error => 2,
            Paused => 9,
        }
    }
}
