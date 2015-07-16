use mio::Token;
use super::super::Message;

#[derive(Debug)]
pub enum Notification {
    CloseConnection(Token),
    Shutdown,
    Response(Token, Message),
}

impl Notification {
    pub fn close_connection(token: Token) -> Notification {
        Notification::CloseConnection(token)
    }

    pub fn shutdown() -> Notification {
        Notification::Shutdown
    }
}
