use std::{
    collections::HashMap,
    io::{BufRead, Write},
};

use derive_more::{Display, Error};
use serde::Serialize;
use serde_json::Value;
use uma_core::{
    core::SourceFile,
    parser::{ParseError, UmaParser},
    scanner::Scanner,
};

use crate::{
    jsonrpc::{self, Request, Response},
    structs::{
        Diagnostic, DiagnosticSeverity, DidChangeTextDocumentParams, DidCloseTextDocumentParams,
        DidOpenTextDocumentParams, InitializeParams, InitializedParams, LogMessageParams,
        MessageType, OutNotification, Position, PositionEncodingKind, PublishDiagnosticsParams,
        Range, RequestError, RequestHandlerResult, RequestResult, ServerCapabilities,
        ShowMessageParams, TextDocumentSyncKind,
    },
};

macro_rules! match_handlers {
    ($self:expr, $raw_params:expr, $method_var:expr, $($method:literal => $handler:ident),*, _ => $fallback:expr) => {
        match $method_var {
            $(
                $method => $self.$handler(serde_json::from_value($raw_params.unwrap_or(Value::Null))?)?,
            )*
            _ => $fallback,
        }
    };
}

#[derive(Debug, Clone, Display, Error)]
pub enum FatalError {
    #[display("unsupported jsonrpc version `{_0}`")]
    UnsupportedJsonRpcVersion(#[error(ignore)] String),

    #[display("unsupported encoding `{_0}`")]
    UnsupportedContentType(#[error(ignore)] String),

    #[display("malformed header")]
    MalformedHeader,
}

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
            let req = self.recv_request()?;

            if req.jsonrpc != jsonrpc::VERSION {
                return Err(FatalError::UnsupportedJsonRpcVersion(req.jsonrpc).into());
            }

            self.handle_request(req)?;
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

            let (name, value) = trim.split_once(": ").ok_or(FatalError::MalformedHeader)?;

            headers.insert(name.to_string(), value.to_string());
        }
    }

    fn recv_request(&mut self) -> anyhow::Result<Request<Option<Value>>> {
        let headers = self.recv_headers()?;

        let content_type = headers
            .get("Content-Type")
            .map(String::as_str)
            .unwrap_or("utf-8");

        if content_type != "utf-8" && content_type != "utf8" {
            return Err(FatalError::UnsupportedContentType(content_type.to_string()).into());
        }

        let content_length: usize = headers
            .get("Content-Length")
            .expect("missing `Content-Length` header")
            .parse()?;

        let mut buf = vec![0; content_length];
        self.input.read_exact(&mut buf)?;

        // Redundantly converting `buf` to a utf-8 string makes sure `buf` is properly utf-8-encoded
        // as per the Content-Type header
        let buf_str = String::from_utf8(buf)?;
        Ok(serde_json::from_str(&buf_str)?)
    }

    fn handle_request(&mut self, req: Request<Option<Value>>) -> anyhow::Result<()> {
        let Some(id) = req.id else {
            return self.handle_notification(&req.method, req.params);
        };

        let result = match_handlers! {
            self,
            req.params,
            req.method.as_str(),
            "initialize" => handle_initialize,
            "shutdown" => handle_shutdown,
            _ => Err(RequestError::MethodNotFound)

        };

        let res_msg = match result {
            Ok(result) => Response::ok(id, serde_json::to_value(result)?),
            Err(error) => Response::err(id, error.into_response_error()),
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
            }
        }
        Ok(())
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

    // Utilities ==============================================================

    fn log(&mut self, r#type: MessageType, message: String) -> anyhow::Result<()> {
        let params = LogMessageParams { r#type, message };
        self.send_message(&OutNotification::LogMessage { params })
    }

    fn show(&mut self, r#type: MessageType, message: String) -> anyhow::Result<()> {
        let params = ShowMessageParams { r#type, message };
        self.send_message(&OutNotification::ShowMessage { params })
    }

    // Request handlers =======================================================

    fn handle_initialize(
        &mut self,
        params: InitializeParams,
    ) -> anyhow::Result<RequestHandlerResult> {
        // TODO: support other encodings, mainly utf-16
        let encoding_priority = [
            PositionEncodingKind::Utf8,
            PositionEncodingKind::Utf16,
            PositionEncodingKind::Utf32,
        ];

        let available_encodings = params
            .capabilities
            .general
            .map(|g| g.position_encodings)
            .unwrap_or_else(|| vec![PositionEncodingKind::Utf16]);

        let chosen_encoding_opt = encoding_priority
            .into_iter()
            .find(|enc| available_encodings.contains(enc));

        let Some(chosen_encoding) = chosen_encoding_opt else {
            return Ok(Err(RequestError::InitializeError { retry: false }));
        };

        Ok(Ok(RequestResult::Initialize {
            capabilities: ServerCapabilities {
                position_encoding: Some(chosen_encoding),
                text_document_sync: Some(TextDocumentSyncKind::Full),
            },
        }))
    }

    fn handle_shutdown(&mut self, _params: ()) -> anyhow::Result<RequestHandlerResult> {
        // TODO: set shutdown flag to reject further requests
        self.show(MessageType::Info, "Shutting down server...".to_string())?;
        Ok(Ok(RequestResult::Shutdown))
    }

    // Notification handlers ==================================================

    fn handle_initialized(&mut self, _params: InitializedParams) -> anyhow::Result<()> {
        self.show(MessageType::Info, "Server initialized.".to_string())?;
        Ok(())
    }

    fn handle_did_open(&mut self, params: DidOpenTextDocumentParams) -> anyhow::Result<()> {
        let doc = params.text_document;
        self.buffers.insert(doc.uri.clone(), doc.text);
        self.udpate_diagnostics(&doc.uri, doc.version)?;
        Ok(())
    }

    fn handle_did_change(&mut self, params: DidChangeTextDocumentParams) -> anyhow::Result<()> {
        let doc = params.text_document;

        for change in params.content_changes {
            assert!(change.range.is_none()); // not supported yet
            self.buffers.insert(doc.uri.clone(), change.text);
        }

        self.udpate_diagnostics(&doc.uri, doc.version)?;
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

    // Other ==================================================================

    fn udpate_diagnostics(&mut self, uri: &str, version: i32) -> anyhow::Result<()> {
        let Some(buf) = self.buffers.get(uri) else {
            return Ok(());
        };

        let text = SourceFile::from_contents(buf.clone());
        let mut scanner = Scanner::new(text.contents());
        let mut parser = UmaParser::new(&mut scanner);

        let diagnostics = match parser.program_to_end() {
            Ok(_) => vec![],
            Err(errors) => errors
                .into_iter()
                .map(|err| self.error_to_diagnostic(err, &text))
                .collect(),
        };

        let params = PublishDiagnosticsParams {
            uri: uri.to_string(),
            version: Some(version),
            diagnostics,
        };
        self.send_message(&OutNotification::PublishDiagnostics { params })?;

        Ok(())
    }

    fn error_to_diagnostic(&self, error: ParseError, text: &SourceFile) -> Diagnostic {
        let range = error.byte_range().map_or_else(
            || {
                let start_pos: Position = text.end_pos().into();
                let end_pos = Position::new(start_pos.line, start_pos.character + 1);
                Range::new(start_pos, end_pos)
            },
            |r| {
                let start_pos = text.byte_to_line(r.start);
                let end_pos = text.byte_to_line(r.end - 1);
                Range::new(start_pos.into(), end_pos.into())
            },
        );

        Diagnostic {
            range,
            code: None,
            message: error.fmt_with_src(text.contents()),
            severity: Some(DiagnosticSeverity::Error),
            source: None,
        }
    }
}
