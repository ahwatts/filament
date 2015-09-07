use mio::Token;
use mogilefs_common::{MogResult, Response};

#[derive(Debug)]
pub enum Notification {
    CloseConnection(Token),
    Shutdown,
    Response(Token, MogResult<Box<Response>>),
}

impl Notification {
    pub fn close_connection(token: Token) -> Notification {
        Notification::CloseConnection(token)
    }

    pub fn shutdown() -> Notification {
        Notification::Shutdown
    }
}
