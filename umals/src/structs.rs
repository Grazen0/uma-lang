use serde::{Deserialize, Serialize};
use serde_repr::Serialize_repr;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClientCapabilities {
    text_document: Option<TextDocumentClientCapabilities>,
    window: Option<WindowClientCapabilities>,
    general: Option<GeneralClientCapabilities>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TextDocumentClientCapabilities {
    publish_diagnostics: Option<PublishDiagnosticsClientCapabilities>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PublishDiagnosticsClientCapabilities {}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WindowClientCapabilities {
    show_message: Option<ShowMessageRequestClientCapabilities>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ShowMessageRequestClientCapabilities {}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GeneralClientCapabilities {
    position_encodings: Vec<PositionEncodingKind>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InitializeParams {
    process_id: Option<i32>,
    capabilities: ClientCapabilities,
}

#[derive(Debug, Clone, Copy, Serialize_repr)]
#[repr(i32)]
pub enum TextDocumentSyncKind {
    None = 0,
    Full = 1,
    Incremental = 2,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Position {
    line: u32,
    character: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Range {
    start: Position,
    end: Position,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PositionEncodingKind {
    #[serde(rename = "utf-8")]
    Utf8,
    #[serde(rename = "utf-16")]
    Utf16,
    #[serde(rename = "utf-32")]
    Utf32,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerCapabilities {
    pub text_document_sync: Option<TextDocumentSyncKind>,
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
    PublishDiagnostics { params: ShowMessageParams },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InitializeError {
    pub retry: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InitializedParams {}

#[derive(Debug, Clone, Serialize)]
#[serde(untagged, rename_all = "camelCase")]
pub enum ResponseResult {
    Initialize { capabilities: ServerCapabilities },
    Shutdown,
}
