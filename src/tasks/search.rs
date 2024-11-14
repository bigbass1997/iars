use std::cmp::min;
use std::collections::HashMap;
use serde::{Deserialize, Deserializer};
use crate::{Credentials, DEFAULT_USER_AGENT};
use crate::headers::RequestHeaderExt;
use crate::tasks::{Command, Status, TaskError};

/// Filters usable when requesting tasks.
/// 
/// Wildcards can be used in some filters by including any number of `*` or `%` within the string.
/// 
/// Any combination of filters are AND-ed together when searching tasks. No other logical operators are supported.
/// Meaning if a search uses both a [Server][`Filter::Server`] and a [Command][`Filter::Command`] filter, the search will only provide tasks where both filters
/// match the task's data.
#[derive(Debug, Clone, PartialEq)]
pub enum Filter {
    /// Item identifier.
    /// 
    /// Wildcards (`*` or `%`) can be used in this filter, unless the request asks for
    /// the `history` [category][`Request::with_categories`] of tasks.
    Identifier(String),
    
    /// Task identification number.
    TaskId(usize),
    
    /// Name of the Internet Archive server the task was or will be performed on.
    /// 
    /// Wildcards (`*` or `%`) can be used in this filter.
    /// 
    /// Some possible examples:
    /// * `ia600501.us.archive.org`
    /// * `ia601302.us.archive.org`
    /// * `*.archive.org`
    /// * `ia*.us.*`
    Server(String),
    
    /// The name of the [command][`Command`] that the task has or will have performed.
    /// 
    /// Wildcards (`*` or `%`) can be used in this filter.
    Command(String),
    
    //TODO: Args(String), // this seems to relate to the 'args' field used with each 'cmd' script, but it's not clear how it should be formatted
    
    /// Email address of the user that submitted the task.
    /// 
    /// Wildcards (`*` or `%`) can be used in this filter.
    Submitter(String),
    
    /// Priority of the task.
    /// 
    /// Typically a number from -10 to +10 (inclusive), with 0 as the default.
    Priority(isize),
    
    /// The current [state][Status] of the task.
    State(Status),
    
    /// All tasks submitted _after_ the provided date/time.
    SubmitTimeGt(String), //TODO: Improvement: Change SubmitTime* to use a "time" type rather than a String, and convert to the expected format in the query
    
    /// All tasks submitted _before_ the provided date/time.
    SubmitTimeLt(String),
    
    /// All tasks submitted _on or after_ the provided date/time.
    SubmitTimeGte(String),
    
    /// All tasks submitted _on or before_ the provided date/time.
    SubmitTimeLte(String),
}
impl From<Command> for Filter {
    fn from(value: Command) -> Self {
        Self::Command(value.name().to_string())
    }
}
impl From<Status> for Filter {
    fn from(value: Status) -> Self {
        Self::State(value)
    }
}

/// Request builder for performing task searches.
/// 
/// Refer to [`Request::call`] for an example.
#[derive(Debug, Clone, PartialEq)]
pub struct Request {
    credentials: Option<Credentials>,
    useragent: String,
    filters: HashMap<String, String>,
    summary: bool,
    catalog: bool,
    history: bool,
    limit: usize,
}
impl Default for Request {
    fn default() -> Self {
        Self {
            credentials: None,
            useragent: DEFAULT_USER_AGENT.to_string(),
            filters: Default::default(),
            summary: true,
            catalog: false,
            history: false,
            limit: 50,
        }
    }
}
impl Request {
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Provide authentication credentials to be used with this request.
    /// 
    /// These keys can be found [here](https://archive.org/account/s3.php).
    /// 
    /// Operations that require authentication but where none are provided, or when the keys are invalid,
    /// will result in a 403 Forbidden error.
    pub fn with_credentials(mut self, credentials: Option<Credentials>) -> Self {
        self.credentials = credentials;
        
        self
    }
    
    /// Configures the User-Agent string provided in this request.
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
    
    /// Configures which categories of results will be returned: [summary][`Summary`], [catalog][`CatalogEntry`], and [history][`HistoryEntry`].
    /// 
    /// Summary is enabled by default.
    /// 
    /// If `history` is true, the request _must_ provide at least an [item identifier][`Filter::Identifier`] or
    /// a [task ID][`Filter::TaskId`] filter. If neither type of filter is present, a `400 Bad Request` error response
    /// will be produced.
    /// 
    /// Additionally, if `history` is true, and the [item identifier][`Filter::Identifier`] filter is used, the filter
    /// must _not_ contain any wildcard characters (`*` and `%`).
    pub fn with_categories(mut self, summary: bool, catalog: bool, history: bool) -> Self {
        self.summary = summary;
        self.catalog = catalog;
        self.history = history;
        
        self
    }
    
    /// Sets the maximum number of tasks returned by each request [call][`crate::tasks::search::Request::call`].
    /// 
    /// This number is the combined total between both the catalog and history categories.
    /// 
    /// Any limit above 500 will be clamped to 500. Limits of 0 are permitted, but wasteful. If only the summary category
    /// is needed, the caller should use [with_categories][`Request::with_categories`] instead.
    /// 
    /// This limit does _not_ have any affect on the results of the summary category.
    pub fn with_limit(mut self, limit: usize) -> Self {
        self.limit = min(limit, 500);
        
        self
    }
    
    /// Adds a filter to the request.
    /// 
    /// Each filter variant can only be used once. If multiple of the same variant are provided, only the last will be used.
    /// 
    /// The search will AND all filters together, thus only returning tasks that match all of the provided filters.
    pub fn with_filter(mut self, filter: Filter) -> Self {
        use Filter::*;
        let (key, val) = match filter {
            Identifier(val) => ("identifier", val),
            TaskId(val) => ("task_id", val.to_string()),
            Server(val) => ("server", val),
            Command(val) => ("cmd", val.to_string()),
            Submitter(val) => ("submitter", val),
            Priority(val) => ("priority", val.to_string()),
            State(val) => ("wait_admin", val.to_string()),
            SubmitTimeGt(val) => ("submittime>", val),
            SubmitTimeLt(val) => ("submittime<", val),
            SubmitTimeGte(val) => ("submittime>=", val),
            SubmitTimeLte(val) => ("submittime<=", val),
        };
        
        self.filters.insert(key.to_string(), val);
        
        self
    }
    
    /// Performs the request query to the Internet Archive.
    /// 
    /// On success, returns the [`Response`] data.
    /// 
    /// The response _may_ contain a "cursor" string if there are more tasks that match this request's parameters, but
    /// were not included in this response. (This is the Internet Archive's method of pagination.)
    /// 
    /// The cursor can be provided in subsequent calls of this method, however, the request's parameters _must_ be the
    /// same as the request used to produce the cursor.
    /// 
    /// # Errors
    /// This may return [`TaskError::Ureq`] if a [`ureq::Error`] is encountered while performing the request. If the error
    /// is a 403 Forbidden, then [`TaskError::Forbidden`] is returned instead.
    /// 
    /// If any [I/O errors][`std::io::Error`] occur or the response fails to be deserialized, a [`TaskError::Io`] is returned.
    /// 
    /// # Example
    /// The following example creates a search request for all tasks from a specific item, then executes that request
    /// repeatedly until there are no more results.
    /// ```rust,no_run
    /// use iars::Credentials;
    /// use iars::tasks::search::Filter;
    /// 
    /// let request = iars::tasks::search()
    ///     .with_credentials(Some(Credentials::new("accesskey", "secretkey")))
    ///     .with_categories(true, true, true)
    ///     .with_filter(Filter::Identifier("test_item".into()));
    /// 
    /// let mut responses = vec![];
    /// let mut cursor = None;
    /// loop {
    ///     let resp = request.call(cursor)?; // perform the request
    ///     cursor = resp.cursor.clone(); // copy the optional cursor
    ///     
    ///     responses.push(resp); // store the response
    ///     
    ///     if cursor.is_none() { // break from the loop if no more requests are needed
    ///         break;
    ///     }
    /// }
    /// 
    /// // do something with the responses
    /// # Ok::<(), iars::tasks::TaskError>(())
    /// ```
    pub fn call(&self, cursor: Option<String>) -> Result<Response, TaskError> {
        let mut req = ureq::get("https://archive.org/services/tasks.php")
            .set("user-agent", &self.useragent)
            .query_pairs(self.filters.iter().map(|(key, val)| (key.as_str(), val.as_str())))
            .query("summary", &(self.summary as usize).to_string())
            .query("catalog", &(self.catalog as usize).to_string())
            .query("history", &(self.history as usize).to_string())
            .query("limit", &self.limit.to_string());
        
        if let Some(cursor) = cursor {
            req = req.query("cursor", &cursor);
        }
        
        if let Some(creds) = self.credentials.as_ref() {
            req = req.set_header(creds.into());
        }
        
        Ok(req.call()?.into_json()?)
    }
}

/// Response data returned from a successful task [search request][`Request`].
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(from = "InterimResponse")]
pub struct Response {
    pub success: bool,
    
    /// List of active tasks (queued, running, errored, or paused).
    pub catalog: Vec<CatalogEntry>,
    
    /// List of completed tasks.
    pub history: Vec<HistoryEntry>,
    
    /// Total counts of catalog tasks. Completed tasks are _not_ counted.
    pub summary: Option<Summary>,
    
    /// Pagination token string for use in subsequent request calls.
    /// 
    /// If `None`, there is no more data to retrieve.
    /// 
    /// See also: [API Docs](https://archive.org/developers/tasks.html#limits-and-the-cursor)
    pub cursor: Option<String>,
}
impl From<InterimResponse> for Response {
    fn from(resp: InterimResponse) -> Self {
        Self {
            success: resp.success,
            catalog: resp.value.catalog,
            history: resp.value.history,
            summary: resp.value.summary,
            cursor: resp.value.cursor,
        }
    }
}

#[derive(Debug, Deserialize)]
struct InterimResponse {
    success: bool,
    #[serde(deserialize_with = "InnerValue::try_deserialize")]
    value: InnerValue,
}

#[derive(Debug, Deserialize, Default)]
struct InnerValue {
    #[serde(default)]
    catalog: Vec<CatalogEntry>,
    #[serde(default)]
    history: Vec<HistoryEntry>,
    summary: Option<Summary>,
    cursor: Option<String>,
}
impl InnerValue {
    pub fn try_deserialize<'de, D: Deserializer<'de>>(de: D) -> Result<Self, D::Error> {
        Ok(InnerValue::deserialize(de).unwrap_or_else(|_| Default::default()))
    }
}

/// Contains the data of a single active task.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct CatalogEntry {
    pub args: HashMap<String, String>,
    pub cmd: String,
    pub identifier: String,
    pub priority: isize,
    pub server: Option<String>,
    pub status: Status,
    pub submitter: String,
    #[serde(rename = "submittime")]
    pub submit_time: String,
    pub task_id: usize,
}

/// Contains the data of a single completed task.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct HistoryEntry {
    pub args: HashMap<String, String>,
    pub cmd: String,
    /// Unknown use
    /// 
    /// _If you know what this field is used for, please open an issue or PR on github._
    pub finished: usize,
    pub identifier: String,
    pub priority: isize,
    pub server: String,
    pub submitter: String,
    #[serde(rename = "submittime")]
    pub submit_time: String,
    pub task_id: usize,
}

/// Total counts of active tasks matched in a search request, organized by the current [status][`Status`] of each task.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Summary {
    pub queued: usize,
    pub running: usize,
    pub error: usize,
    pub paused: usize,
}