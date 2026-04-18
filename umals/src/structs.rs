#![allow(unused)]

use derive_more::Constructor;
use serde::{Deserialize, Serialize, Serializer};
use serde_repr::{Deserialize_repr, Serialize_repr};
use uma_core::core::SourceFile;

use crate::jsonrpc::{self, ResponseError};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged, rename_all = "camelCase")]
pub enum RequestError {
    ParseError,
    InvalidRequest,
    MethodNotFound,
    InvalidParams,
    InternalError,

    ServerNotInitialized,
    UnknownErrorCode,

    RequestFailed,
    ServerCancelled,
    ContentModified,
    RequestCancelled,
    InitializeError { retry: bool },
}

impl RequestError {
    fn code(&self) -> i32 {
        match self {
            Self::ParseError => jsonrpc::error::PARSE_ERROR,
            Self::InvalidRequest => jsonrpc::error::INVALID_REQUEST,
            Self::MethodNotFound => jsonrpc::error::METHOD_NOT_FOUND,
            Self::InvalidParams => jsonrpc::error::INVALID_PARAMS,
            Self::InternalError => jsonrpc::error::INTERNAL_ERROR,

            Self::ServerNotInitialized => -32002,
            Self::UnknownErrorCode => -32001,

            Self::RequestFailed => -32803,
            Self::ServerCancelled => -32802,
            Self::ContentModified => -32801,
            Self::RequestCancelled => -32800,

            Self::InitializeError { .. } => 1,
        }
    }

    fn message(&self) -> String {
        match self {
            Self::ParseError => "parse error".to_string(),
            Self::InvalidRequest => "invalid request".to_string(),
            Self::MethodNotFound => "method not found".to_string(),
            Self::InvalidParams => "invalid params".to_string(),
            Self::InternalError => "internal error".to_string(),

            Self::ServerNotInitialized => "server not initialized".to_string(),
            Self::UnknownErrorCode => "unknown error code".to_string(),

            Self::RequestFailed => "request failed".to_string(),
            Self::ServerCancelled => "server cancelled".to_string(),
            Self::ContentModified => "content modified".to_string(),
            Self::RequestCancelled => "request cancelled".to_string(),

            Self::InitializeError { .. } => "initialize error".to_string(),
        }
    }

    pub fn into_response_error(self) -> ResponseError<Option<RequestError>> {
        ResponseError {
            code: self.code(),
            message: self.message(),
            data: Some(self),
        }
    }
}

pub type RequestHandlerResult = Result<RequestResult, RequestError>;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClientCapabilities {
    pub text_document: Option<TextDocumentClientCapabilities>,
    pub window: Option<WindowClientCapabilities>,
    pub general: Option<GeneralClientCapabilities>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TextDocumentClientCapabilities {
    pub publish_diagnostics: Option<PublishDiagnosticsClientCapabilities>,
    pub document_symbol: Option<DocumentSymbolClientCapabilities>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DocumentSymbolClientCapabilities {
    pub symbol_kind: Option<DocumentSymbolClientCapabilitiesSymbolKind>,
    pub hierarchical_document_symbol_support: Option<bool>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DocumentSymbolClientCapabilitiesSymbolKind {
    pub value_set: Option<Vec<SymbolKind>>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PublishDiagnosticsClientCapabilities {}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WindowClientCapabilities {
    pub show_message: Option<ShowMessageRequestClientCapabilities>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ShowMessageRequestClientCapabilities {}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GeneralClientCapabilities {
    pub position_encodings: Vec<PositionEncodingKind>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InitializeParams {
    pub process_id: Option<i32>,
    pub capabilities: ClientCapabilities,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DocumentSymbolParams {
    pub text_document: TextDocumentIdentifier,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DocumentSymbol {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
    pub kind: SymbolKind,
    pub range: Range,
    pub selection_range: Range,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub children: Option<Vec<DocumentSymbol>>,
}

#[derive(Debug, Clone, Copy, Serialize_repr, Deserialize_repr)]
#[repr(i32)]
pub enum SymbolKind {
    File = 1,
    Module = 2,
    Namespace = 3,
    Package = 4,
    Class = 5,
    Method = 6,
    Property = 7,
    Field = 8,
    Constructor = 9,
    Enum = 10,
    Interface = 11,
    Function = 12,
    Variable = 13,
    Constant = 14,
    String = 15,
    Number = 16,
    Boolean = 17,
    Array = 18,
    Object = 19,
    Key = 20,
    Null = 21,
    EnumMember = 22,
    Struct = 23,
    Event = 24,
    Operator = 25,
    TypeParameter = 26,
}

#[derive(Debug, Clone, Copy, Serialize_repr)]
#[repr(i32)]
pub enum TextDocumentSyncKind {
    None = 0,
    Full = 1,
    Incremental = 2,
}

#[derive(Debug, Clone, PartialEq, Eq, Constructor, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Position {
    pub line: u32,
    pub character: u32,
}

impl From<(usize, usize)> for Position {
    fn from((line, col): (usize, usize)) -> Self {
        Self::new(line as u32, col as u32)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Constructor, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Range {
    pub start: Position,
    pub end: Position,
}

impl Range {
    pub fn from_span(span: &std::ops::Range<usize>, src: &SourceFile) -> Self {
        let start = src.byte_to_line(span.start);
        let end = src.byte_to_line(span.end);
        Self::new(start.into(), end.into())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PositionEncodingKind {
    #[serde(rename = "utf-8")]
    Utf8,
    #[serde(rename = "utf-16")]
    Utf16,
    #[serde(rename = "utf-32")]
    Utf32,
}

#[derive(Debug, Clone, Serialize)]
#[serde(untagged)]
pub enum Capability<T> {
    Supported(bool),
    WithOptions(T),
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerCapabilities {
    pub position_encoding: Option<PositionEncodingKind>,
    pub text_document_sync: Option<TextDocumentSyncKind>,
    pub document_symbol_provider: Option<Capability<DocumentSymbolOptions>>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DocumentSymbolOptions {
    label: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DidOpenTextDocumentParams {
    pub text_document: TextDocumentItem,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DidChangeTextDocumentParams {
    pub text_document: VersionedTextDocumentIdentifier,
    pub content_changes: Vec<TextDocumentContentChangeEvent>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TextDocumentContentChangeEvent {
    pub range: Option<Range>,
    pub text: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DidCloseTextDocumentParams {
    pub text_document: TextDocumentIdentifier,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TextDocumentItem {
    pub uri: String,
    pub language_id: String,
    pub version: i32,
    pub text: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TextDocumentIdentifier {
    pub uri: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VersionedTextDocumentIdentifier {
    pub uri: String,
    pub version: i32,
}

#[derive(Debug, Clone, Serialize_repr)]
#[repr(i32)]
pub enum DiagnosticSeverity {
    Error = 1,
    Warning = 2,
    Information = 3,
    Hint = 4,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Diagnostic {
    pub range: Range,
    pub severity: Option<DiagnosticSeverity>,
    pub code: Option<String>,
    pub source: Option<String>,
    pub message: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LogMessageParams {
    pub r#type: MessageType,
    pub message: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ShowMessageParams {
    pub r#type: MessageType,
    pub message: String,
}

#[derive(Debug, Clone, Serialize_repr)]
#[repr(i32)]
pub enum MessageType {
    Error = 1,
    Warning = 2,
    Info = 3,
    Log = 4,
    Debug = 5,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "method")]
pub enum OutNotification {
    #[serde(rename = "window/logMessage")]
    LogMessage { params: LogMessageParams },
    #[serde(rename = "window/showMessage")]
    ShowMessage { params: ShowMessageParams },
    #[serde(rename = "textDocument/publishDiagnostics")]
    PublishDiagnostics { params: PublishDiagnosticsParams },
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PublishDiagnosticsParams {
    pub uri: String,
    pub version: Option<i32>,
    pub diagnostics: Vec<Diagnostic>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InitializedParams {}

#[derive(Debug, Clone, Serialize)]
#[serde(untagged, rename_all = "camelCase")]
pub enum RequestResult {
    Initialize { capabilities: ServerCapabilities },
    Shutdown,
    DocumentSymbol(Option<Vec<DocumentSymbol>>),
}
