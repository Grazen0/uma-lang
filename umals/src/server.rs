use std::{
    collections::HashMap,
    io::{BufRead, Write},
};

use derive_more::{Display, Error};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use serde_json::Value;
use serde_repr::Serialize_repr;

use crate::jsonrpc::{self, IncomingMessage, Response, ResponseError};

macro_rules! match_handlers {
    ($self:expr, $raw_params:expr, $method_var:expr, $($method:literal => $handler:ident),*, _ => $fallback:expr) => {
        match $method_var {
            $(
                $method => $self.$handler(Self::parse_raw_params($raw_params.unwrap_or(Value::Null))?),
            )*
            _ => $fallback,
        }
    };
}

#[derive(Debug, Clone, Display, Error)]
pub enum LspError {
    #[display("unsupported jsonrpc version `{_0}`")]
    UnsupportedJsonRpcVersion(#[error(ignore)] String),
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ClientCapabilities {
    text_document: Option<TextDocumentClientCapabilities>,
    window: Option<WindowClientCapabilities>,
    general: Option<GeneralClientCapabilities>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TextDocumentClientCapabilities {
    publish_diagnostics: Option<PublishDiagnosticsClientCapabilities>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PublishDiagnosticsClientCapabilities {}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct WindowClientCapabilities {
    show_message: Option<ShowMessageRequestClientCapabilities>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ShowMessageRequestClientCapabilities {}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GeneralClientCapabilities {
    position_encodings: Vec<PositionEncodingKind>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct InitializeParams {
    process_id: Option<i32>,
    capabilities: ClientCapabilities,
}

#[derive(Debug, Clone, Copy, Serialize_repr)]
#[repr(i32)]
enum TextDocumentSyncKind {
    None = 0,
    Full = 1,
    Incremental = 2,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Position {
    line: u32,
    character: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Range {
    start: Position,
    end: Position,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
enum PositionEncodingKind {
    #[serde(rename = "utf-8")]
    Utf8,
    #[serde(rename = "utf-16")]
    Utf16,
    #[serde(rename = "utf-32")]
    Utf32,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ServerCapabilities {
    text_document_sync: Option<TextDocumentSyncKind>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DidOpenTextDocumentParams {
    text_document: TextDocumentItem,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DidChangeTextDocumentParams {
    text_document: VersionedTextDocumentIdentifier,
    content_changes: Vec<TextDocumentContentChangeEvent>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TextDocumentContentChangeEvent {
    range: Option<Range>,
    text: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DidCloseTextDocumentParams {
    text_document: TextDocumentIdentifier,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TextDocumentItem {
    uri: String,
    language_id: String,
    version: i32,
    text: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TextDocumentIdentifier {
    uri: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct VersionedTextDocumentIdentifier {
    uri: String,
    version: i32,
}

#[derive(Debug, Clone, Serialize_repr)]
#[repr(i32)]
enum DiagnosticSeverity {
    Error = 1,
    Warning = 2,
    Information = 3,
    Hint = 4,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct Diagnostic {
    range: Range,
    severity: Option<DiagnosticSeverity>,
    code: Option<String>,
    source: Option<String>,
    message: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct LogMessageParams {
    r#type: MessageType,
    message: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ShowMessageParams {
    r#type: MessageType,
    message: String,
}

#[derive(Debug, Clone, Serialize_repr)]
#[repr(i32)]
enum MessageType {
    Error = 1,
    Warning = 2,
    Info = 3,
    Log = 4,
    Debug = 5,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "method")]
enum OutNotification {
    #[serde(rename = "window/logMessage")]
    LogMessage { params: LogMessageParams },
    #[serde(rename = "window/showMessage")]
    ShowMessage { params: ShowMessageParams },
    #[serde(rename = "textDocument/publishDiagnostics")]
    PublishDiagnostics { params: ShowMessageParams },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct InitializeError {
    retry: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct InitializedParams {}

#[derive(Debug, Clone, Serialize)]
#[serde(untagged, rename_all = "camelCase")]
enum ResponseResult {
    Initialize { capabilities: ServerCapabilities },
    Shutdown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
enum RequestError {
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
    InitializeError(InitializeError),
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

            Self::InitializeError(..) => "initialize error".to_string(),
        }
    }

    fn to_response_error(self) -> ResponseError<Option<RequestError>> {
        ResponseError {
            code: self.code(),
            message: self.message(),
            data: Some(self),
        }
    }
}

type RequestResult = Result<ResponseResult, RequestError>;

pub struct Server<I: BufRead, O: Write> {
    exit: bool,
    input: I,
    output: O,
    buffers: HashMap<String, String>,
}

impl<I: BufRead, O: Write> Server<I, O> {
    pub fn new(input: I, output: O) -> Self {
        Self {
            exit: false,
            input,
            output,
            buffers: HashMap::new(),
        }
    }

    pub fn run(&mut self) -> anyhow::Result<()> {
        while !self.exit {
            let message = self.recv_message()?;

            if message.jsonrpc != jsonrpc::VERSION {
                return Err(LspError::UnsupportedJsonRpcVersion(message.jsonrpc).into());
            }

            match message.id {
                Some(id) => self.handle_request(id, &message.method, message.params)?,
                None => self.handle_notification(&message.method, message.params)?,
            }
        }

        Ok(())
    }

    fn recv_headers(&mut self) -> anyhow::Result<HashMap<String, String>> {
        let mut headers = HashMap::new();

        loop {
            let mut buf = String::new();
            self.input.read_line(&mut buf)?;

            let trim = buf.trim();
            if trim.is_empty() {
                break Ok(headers);
            }

            let (name, value) = trim.split_once(": ").unwrap();
            headers.insert(name.to_string(), value.to_string());
        }
    }

    fn recv_message(&mut self) -> anyhow::Result<IncomingMessage<Option<Value>>> {
        let headers = self.recv_headers()?;

        let content_type = headers
            .get("Content-Type")
            .map(String::as_str)
            .unwrap_or("utf-8");

        assert!(content_type == "utf-8" || content_type == "utf8");

        let content_length: usize = headers
            .get("Content-Length")
            .expect("missing Content-Length header")
            .parse()?;

        let mut buf = vec![0; content_length];
        self.input.read_exact(&mut buf)?;

        // Redundantly converting `buf` to a utf-8 string makes sure `buf` is properly utf-8-encoded
        // as per the Content-Type header
        let buf_str = String::from_utf8(buf)?;
        Ok(serde_json::from_str(&buf_str)?)
    }

    fn handle_initialize(&mut self, params: InitializeParams) -> anyhow::Result<RequestResult> {
        self.log(MessageType::Error, format!("{:?}", params))?;
        Ok(Ok(ResponseResult::Initialize {
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncKind::Full),
            },
        }))
    }

    fn handle_shutdown(&mut self, _params: ()) -> anyhow::Result<RequestResult> {
        // TODO: set shutdown flag to reject further requests
        self.show(MessageType::Info, "Shutting down server...".to_string())?;
        Ok(Ok(ResponseResult::Shutdown))
    }

    fn parse_raw_params<P: DeserializeOwned>(raw_params: Value) -> anyhow::Result<P> {
        Ok(serde_json::from_value(raw_params)?)
    }

    fn handle_initialized(&mut self, _params: InitializedParams) -> anyhow::Result<()> {
        self.show(MessageType::Info, "Server initialized.".to_string())?;
        Ok(())
    }

    fn handle_did_open(&mut self, params: DidOpenTextDocumentParams) -> anyhow::Result<()> {
        let doc = params.text_document;
        self.buffers.insert(doc.uri, doc.text.clone());
        Ok(())
    }

    fn handle_did_change(&mut self, params: DidChangeTextDocumentParams) -> anyhow::Result<()> {
        let uri = params.text_document.uri;

        for change in params.content_changes {
            assert!(change.range.is_none()); // not supported yet
            self.buffers.insert(uri.clone(), change.text);
        }

        Ok(())
    }

    fn handle_did_close(&mut self, params: DidCloseTextDocumentParams) -> anyhow::Result<()> {
        self.buffers.remove(&params.text_document.uri);
        Ok(())
    }

    fn handle_exit(&mut self, _params: ()) -> anyhow::Result<()> {
        self.exit = true;
        Ok(())
    }

    fn handle_request(
        &mut self,
        id: i32,
        method: &str,
        raw_params: Option<Value>,
    ) -> anyhow::Result<()> {
        let result = match_handlers! {
            self,
            raw_params,
            method,
            "initialize" => handle_initialize,
            "shutdown" => handle_shutdown,
            _ => Ok(Err(RequestError::MethodNotFound))

        };

        let res_msg = match result? {
            Ok(result) => Response::ok(id, serde_json::to_value(result)?),
            Err(error) => Response::err(id, error.to_response_error()),
        };

        self.send_message(&res_msg)
    }

    fn handle_notification(
        &mut self,
        method: &str,
        raw_params: Option<Value>,
    ) -> anyhow::Result<()> {
        match_handlers! {
            self,
            raw_params,
            method,
            "initialized" => handle_initialized,
            "exit" => handle_exit,
            "textDocument/didOpen" => handle_did_open,
            "textDocument/didChange" => handle_did_change,
            "textDocument/didClose" => handle_did_close,
            _ => {
                self.show(MessageType::Warning, format!("unknown notification: `{method}`"))?;
                Ok(())
            }
        }
    }

    fn log(&mut self, r#type: MessageType, message: String) -> anyhow::Result<()> {
        let notif = OutNotification::LogMessage {
            params: LogMessageParams { message, r#type },
        };

        self.send_message(&notif)
    }

    fn show(&mut self, r#type: MessageType, message: String) -> anyhow::Result<()> {
        let notif = OutNotification::ShowMessage {
            params: ShowMessageParams { message, r#type },
        };

        self.send_message(&notif)
    }

    fn send_message(&mut self, data: &impl Serialize) -> anyhow::Result<()> {
        let data_str = serde_json::to_string(data)?;

        write!(self.output, "Content-Length: {}\r\n", data_str.len())?;
        write!(self.output, "Content-Type: utf-8\r\n")?;
        write!(self.output, "\r\n")?;

        self.output.write_all(data_str.as_bytes())?;
        self.output.flush()?;
        Ok(())
    }
}
