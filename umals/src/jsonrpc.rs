use serde::{Deserialize, Serialize};

pub const VERSION: &str = "2.0";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IncomingMessage<P> {
    pub jsonrpc: String,
    pub method: String,
    pub id: Option<i32>,
    pub params: P,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Response<V, E> {
    pub jsonrpc: String,
    pub id: i32,
    #[serde(flatten)]
    pub value: ResponseValue<V, E>,
}

impl<V, E> Response<V, E> {
    pub fn ok(id: i32, value: V) -> Self {
        Self {
            jsonrpc: VERSION.to_string(),
            id,
            value: ResponseValue::Result(value),
        }
    }

    pub fn err(id: i32, error: ResponseError<E>) -> Self {
        Self {
            jsonrpc: VERSION.to_string(),
            id,
            value: ResponseValue::Error(error),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ResponseValue<V, E> {
    Result(V),
    Error(ResponseError<E>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResponseError<E> {
    pub code: i32,
    pub message: String,
    pub data: E,
}

pub mod error {
    pub const PARSE_ERROR: i32 = -32700;
    pub const INVALID_REQUEST: i32 = -32600;
    pub const METHOD_NOT_FOUND: i32 = -32601;
    pub const INVALID_PARAMS: i32 = -32602;
    pub const INTERNAL_ERROR: i32 = -32603;
}
