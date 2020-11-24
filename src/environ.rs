use std::env;
use std::fmt;

/// The current environment, including secrets.
#[derive(Debug)]
pub struct Env {
    pub client_id: String,
    pub client_secret: String,
    pub username: String,
    pub password: String,
    pub is_sandbox: bool,
}

impl Env {
    /// Return the current environment, including secrets.
    pub fn new() -> Result<Self, Error> {
        let client_id = var("SFDC_CLIENT_ID")?;
        let client_secret = var("SFDC_CLIENT_SECRET")?;
        let username = var("SFDC_USERNAME")?;
        let password = var("SFDC_PASSWORD")? + &var("SFDC_SECRET_TOKEN")?;
        let is_sandbox = match env::var("SFDC_SANDBOX") {
            Ok(v) => ["1", "true", "yes"].iter().any(|&i| i == v.to_lowercase()),
            Err(_) => false,
        };
        Ok(Self {
            client_id,
            client_secret,
            username,
            password,
            is_sandbox,
        })
    }
}

/// Return the content of the environment variable with the given name.
fn var(name: &str) -> Result<String, Error> {
    match env::var(name) {
        Ok(v) => Ok(v),
        Err(_) => Err(Error {
            var: name.to_string(),
        }),
    }
}

/// A failure when fetching an environment variable.
#[derive(Debug)]
pub struct Error {
    var: String,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "missing environment variable {}", self.var)
    }
}

// TODO(frankban): add tests, possibly after introducing a trait for mocking
// env::var. As rust tests are run in parallel, actually setting env vars would
// break isolation.
