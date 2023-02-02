use crate::Error;

pub use self::value::Value;
pub mod value;

mod message_channel;
mod method_channel;
mod sender;
mod standard_codec;

pub use message_channel::*;
pub use method_channel::*;
pub use sender::*;
pub use standard_codec::*;

pub struct MethodCall<V> {
    pub method: String,
    pub args: V,
}

pub type MethodCallResult<V> = Result<V, MethodCallError<V>>;

#[derive(Debug, Clone)]
pub struct MethodCallError<V> {
    pub code: String,
    pub message: Option<String>,
    pub details: V,
}

impl<T> std::fmt::Display for MethodCallError<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.message {
            Some(message) => {
                write!(f, "{} ({})", message, self.code)
            }
            None => write!(f, "{}", self.code),
        }
    }
}

impl<V> MethodCallError<V> {
    pub fn from_code_message(code: &str, message: &str) -> Self
    where
        V: Default,
    {
        Self {
            code: code.into(),
            message: Some(message.into()),
            details: Default::default(),
        }
    }
}

impl<V> From<Error> for MethodCallError<V>
where
    V: Default,
{
    fn from(e: Error) -> Self {
        Self {
            code: format!("{e:?}"),
            message: Some(format!("{e}")),
            details: Default::default(),
        }
    }
}

pub trait MessageCodec<V>: Send + Sync {
    /// Methods for plain messages
    fn encode_message(&self, v: &V) -> Vec<u8>;
    fn decode_message(&self, buf: &[u8]) -> Option<V>;
}

pub trait MethodCodec<V>: Send + Sync {
    fn decode_method_call(&self, buf: &[u8]) -> Option<MethodCall<V>>;
    fn encode_success_envelope(&self, v: &V) -> Vec<u8>;
    fn encode_error_envelope(&self, code: &str, message: Option<&str>, details: &V) -> Vec<u8>;

    fn encode_method_call_result(&self, response: &MethodCallResult<V>) -> Vec<u8> {
        match response {
            MethodCallResult::Ok(data) => self.encode_success_envelope(data),
            MethodCallResult::Err(err) => {
                self.encode_error_envelope(&err.code, err.message.as_deref(), &err.details)
            }
        }
    }

    /// Methods for calling into dart
    fn encode_method_call(&self, v: &MethodCall<V>) -> Vec<u8>;
    fn decode_envelope(&self, buf: &[u8]) -> Option<MethodCallResult<V>>;
}
