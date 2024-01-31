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

/// Retrieves the task log for a task.
/// 
/// These logs are plaintext strings produced by Internet Archive's servers as they process a task.
/// 
/// # Authentication
/// Task logs are only available to the owner of the item the task is associated with, or users with privileged access.
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

/// Filter by the task's command argument.
/// 
/// [`Command::Custom`] may be wildcarded using any number of `*` or `%` within the string.
#[derive(Debug, Clone, PartialEq)]
pub enum Command {
    Archive,
    BookOp,
    Bup,
    Delete,
    Derive,
    Fixer,
    MakeDark,
    MakeUndark,
    ModifyXml,
    Rename,
    Custom(String),
}
impl fmt::Display for Command {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        use Command::*;
        write!(f, "{}", match self {
            Archive => "archive.php",
            BookOp => "book_op.php",
            Bup => "bup.php",
            Delete => "delete.php",
            Derive => "derive.php",
            Fixer => "fixer.php",
            MakeDark => "make_dark.php",
            MakeUndark => "make_undark.php",
            ModifyXml => "modify_xml.php",
            Rename => "rename.php",
            Custom(ref val) => val,
        })
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
