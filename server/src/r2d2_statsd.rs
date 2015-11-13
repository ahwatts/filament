use r2d2::ManageConnection;
use statsd::client::{Client as StatsdClient, StatsdError};

pub struct StatsdConnectionManager {
    host: String,
    prefix: String,
}

impl StatsdConnectionManager {
    pub fn new(host: &str, prefix: &str) -> StatsdConnectionManager {
        StatsdConnectionManager {
            host: host.to_string(),
            prefix: prefix.to_string(),
        }
    }
}

impl ManageConnection for StatsdConnectionManager {
    type Connection = StatsdClient;
    type Error = StatsdError;

    fn connect(&self) -> Result<StatsdClient, StatsdError> {
        StatsdClient::new(&self.host, &self.prefix)
    }

    fn is_valid(&self, _conn: &mut StatsdClient) -> Result<(), StatsdError> {
        Ok(())
    }

    fn has_broken(&self, _conn: &mut StatsdClient) -> bool {
        false
    }
}
