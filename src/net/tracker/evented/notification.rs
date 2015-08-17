use mio::Token;
use super::super::super::super::tracker::Response;

#[derive(Debug)]
pub enum Notification {
    CloseConnection(Token),
    Shutdown,
    Response(Token, Response),
}

impl Notification {
    pub fn close_connection(token: Token) -> Notification {
        Notification::CloseConnection(token)
    }

    pub fn shutdown() -> Notification {
        Notification::Shutdown
    }
}
