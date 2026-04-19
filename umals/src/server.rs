use std::{
    collections::HashMap,
    io::{BufRead, Write},
};

use derive_more::{Display, Error};
use serde::Serialize;
use serde_json::Value;
use uma_core::{
    core::SourceFile,
    fmt::DisplayWithSrcExt,
    parser::{ParseError, UmaParser, ast::Program},
    scanner::Scanner,
    semantic::{self, SemanticModel},
};

use crate::{
    jsonrpc::{self, Request, Response},
    structs::{
        Capability, CompletionOptions, CompletionParams, DefinitionParams, Diagnostic,
        DiagnosticSeverity, DiagnosticTag, DidChangeTextDocumentParams, DidCloseTextDocumentParams,
        DidOpenTextDocumentParams, DocumentSymbol, DocumentSymbolParams, InitializeParams,
        InitializedParams, Location, LogMessageParams, MessageType, OutNotification, Position,
        PositionEncodingKind, PrepareRenameParams, PublishDiagnosticsParams, Range,
        ReferenceParams, RenameOptions, RenameParams, RequestError, RequestHandlerResult,
        RequestResult, ServerCapabilities, ShowMessageParams, SymbolKind, TextDocumentSyncKind,
        TextEdit, WorkspaceEdit,
    },
};

macro_rules! match_handlers {
    ($self:expr, $raw_params:expr, $method_var:expr, $($method:literal => $handler:ident),*, $fb_iden:ident => $fallback:expr) => {
        match $method_var {
            $(
                $method => $self.$handler(serde_json::from_value($raw_params.unwrap_or(Value::Null))?)?,
            )*
            $fb_iden => $fallback,
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

#[derive(Debug, Clone)]
struct Buffer {
    src: SourceFile,
    ast: Option<Program>,
    sem_model: Option<SemanticModel>,
}

impl Buffer {
    pub fn new(src: String) -> Self {
        Self {
            src: SourceFile::from_contents(src),
            ast: None,
            sem_model: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Server<I: BufRead, O: Write> {
    exit: bool,
    input: I,
    output: O,
    buffers: HashMap<String, Buffer>,
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

            self.log(MessageType::Debug, format!("Received request: {:?}", req))?;
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
            "textDocument/definition" => handle_definition,
            "textDocument/references" => handle_references,
            "textDocument/documentSymbol" => handle_document_symbol,
            "textDocument/prepareRename" => handle_prepare_rename,
            "textDocument/rename" => handle_rename,
            "textDocument/completion" => handle_completion,
            _method => Err(RequestError::MethodNotFound)

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
            method => {
                self.log(MessageType::Warning, format!("unknown notification: `{method}`"))?;
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

        let general_capabilities = params.capabilities.general.as_ref();
        let doc_capabilities = params.capabilities.text_document.as_ref();

        let available_encodings = general_capabilities
            .map(|g| g.position_encodings.clone())
            .unwrap_or_else(|| vec![PositionEncodingKind::Utf16]);

        let chosen_encoding_opt = encoding_priority
            .into_iter()
            .find(|enc| available_encodings.contains(enc));

        let Some(chosen_encoding) = chosen_encoding_opt else {
            return Ok(Err(RequestError::InitializeError { retry: false }));
        };

        let support_symbols = doc_capabilities
            .and_then(|doc| doc.document_symbol.as_ref())
            .and_then(|sym| sym.symbol_kind.as_ref())
            .is_some_and(|sym_kind| sym_kind.value_set.is_some());

        let supports_definition = doc_capabilities.is_some_and(|doc| doc.definition.is_some());
        let supports_references = doc_capabilities.is_some_and(|doc| doc.references.is_some());
        let supports_rename = doc_capabilities.is_some_and(|doc| doc.rename.is_some());
        let supports_prepare_rename = doc_capabilities
            .and_then(|doc| doc.rename.as_ref())
            .is_some_and(|rename| rename.prepare_support.is_some_and(|b| b));
        // let supports_completion = doc_capabilities.is_some_and(|doc| doc.completion.is_some());
        let supports_completion = false;

        Ok(Ok(RequestResult::Initialize {
            capabilities: ServerCapabilities {
                position_encoding: Some(chosen_encoding),
                text_document_sync: Some(TextDocumentSyncKind::Full),
                document_symbol_provider: Some(Capability::Supported(support_symbols)),
                definition_provider: Some(Capability::Supported(supports_definition)),
                references_provider: Some(Capability::Supported(supports_references)),
                rename_provider: supports_rename.then_some({
                    if supports_prepare_rename {
                        Capability::WithOptions(RenameOptions {
                            prepare_provider: Some(supports_prepare_rename),
                        })
                    } else {
                        Capability::Supported(true)
                    }
                }),
                completion_provider: supports_completion.then_some(CompletionOptions {}),
            },
        }))
    }

    fn handle_shutdown(&mut self, _params: ()) -> anyhow::Result<RequestHandlerResult> {
        // TODO: set shutdown flag to reject further requests
        self.show(MessageType::Info, "Shutting down server...".to_string())?;
        self.buffers.clear();
        Ok(Ok(RequestResult::Shutdown))
    }

    fn handle_definition(
        &mut self,
        params: DefinitionParams,
    ) -> anyhow::Result<RequestHandlerResult> {
        let Some(buf) = self.buffers.get(&params.pos.text_document.uri) else {
            return Ok(Err(RequestError::InvalidParams));
        };

        let model = buf.sem_model.as_ref();

        let Some(symbol) = model.and_then(|m| m.symbol_lookup(params.pos.position.into())) else {
            return Ok(Ok(RequestResult::DocumentSymbol(None)));
        };

        let Some(def_span) = symbol.span.as_ref() else {
            return Ok(Ok(RequestResult::Definition(None)));
        };

        let loc = Location {
            uri: params.pos.text_document.uri,
            range: def_span.clone().into(),
        };

        Ok(Ok(RequestResult::Definition(Some(loc))))
    }

    fn handle_references(
        &mut self,
        params: ReferenceParams,
    ) -> anyhow::Result<RequestHandlerResult> {
        let Some(buf) = self.buffers.get(&params.pos.text_document.uri) else {
            return Ok(Err(RequestError::InvalidParams));
        };

        let model = buf.sem_model.as_ref();

        let Some(symbol) = model.and_then(|m| m.symbol_lookup(params.pos.position.into())) else {
            return Ok(Ok(RequestResult::References(None)));
        };

        let uri = &params.pos.text_document.uri;

        let mut locs: Vec<_> = symbol
            .refs
            .iter()
            .map(|span| Location::new(uri.clone(), span.clone().into()))
            .collect();

        if let Some(def_span) = symbol.span.as_ref()
            && params.context.include_declaration
        {
            locs.push(Location::new(uri.clone(), def_span.clone().into()));
        }

        Ok(Ok(RequestResult::References(Some(locs))))
    }

    fn handle_document_symbol(
        &mut self,
        params: DocumentSymbolParams,
    ) -> anyhow::Result<RequestHandlerResult> {
        let Some(buf) = self.buffers.get(&params.text_document.uri) else {
            return Ok(Err(RequestError::InvalidParams));
        };

        let Some(ast) = &buf.ast else {
            return Ok(Ok(RequestResult::DocumentSymbol(None)));
        };

        let symbols = ast
            .funcs
            .iter()
            .map(|func| DocumentSymbol {
                name: func.val.name.val.clone(),
                detail: None,
                kind: SymbolKind::Function,
                range: func.span.clone().into(),
                selection_range: func.val.name.span.clone().into(),
                children: None,
            })
            .collect();

        Ok(Ok(RequestResult::DocumentSymbol(Some(symbols))))
    }

    fn handle_prepare_rename(
        &mut self,
        params: PrepareRenameParams,
    ) -> anyhow::Result<RequestHandlerResult> {
        let Some(buf) = self.buffers.get(&params.pos.text_document.uri) else {
            return Ok(Err(RequestError::InvalidParams));
        };

        let model = buf.sem_model.as_ref();

        let Some(symbol) = model.and_then(|m| m.symbol_lookup(params.pos.position.into())) else {
            return Ok(Ok(RequestResult::PrepareRename(None)));
        };

        Ok(Ok(RequestResult::PrepareRename(
            symbol.span.as_ref().map(|span| span.clone().into()),
        )))
    }

    fn handle_rename(&mut self, params: RenameParams) -> anyhow::Result<RequestHandlerResult> {
        let uri = &params.pos.text_document.uri;
        let Some(buf) = self.buffers.get(uri) else {
            return Ok(Err(RequestError::InvalidParams));
        };

        let Some(symbol) = buf
            .sem_model
            .as_ref()
            .and_then(|m| m.symbol_lookup(params.pos.position.into()))
        else {
            return Ok(Ok(RequestResult::Rename(None)));
        };

        let Some(def_span) = symbol.span.as_ref() else {
            return Ok(Ok(RequestResult::Rename(None)));
        };

        let mut locs: Vec<_> = symbol.refs.clone();
        locs.push(def_span.clone());

        let change_list = locs
            .into_iter()
            .map(|span| TextEdit::new(span.into(), params.new_name.clone()))
            .collect();

        let changes = HashMap::from([(uri.clone(), change_list)]);

        Ok(Ok(RequestResult::Rename(Some(WorkspaceEdit {
            changes: Some(changes),
        }))))
    }

    fn handle_completion(
        &mut self,
        _params: CompletionParams,
    ) -> anyhow::Result<RequestHandlerResult> {
        todo!()
    }

    // Notification handlers ==================================================

    fn handle_initialized(&mut self, _params: InitializedParams) -> anyhow::Result<()> {
        self.show(MessageType::Info, "Server initialized.".to_string())?;
        Ok(())
    }

    fn handle_did_open(&mut self, params: DidOpenTextDocumentParams) -> anyhow::Result<()> {
        let doc = params.text_document;
        self.update_buffer(doc.uri.clone(), doc.version, doc.text)?;
        Ok(())
    }

    fn handle_did_change(&mut self, params: DidChangeTextDocumentParams) -> anyhow::Result<()> {
        let doc = params.text_document;

        for change in params.content_changes {
            assert!(change.range.is_none()); // not supported yet
            self.update_buffer(doc.uri.clone(), doc.version, change.text)?;
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

    // Other ==================================================================

    fn update_buffer(&mut self, uri: String, version: i32, src: String) -> anyhow::Result<()> {
        self.buffers.remove(&uri);

        let mut buf = Buffer::new(src);
        let mut scanner = Scanner::new(&buf.src);
        let mut parser = UmaParser::new(&mut scanner);

        let mut diagnostics = vec![];

        match parser.program_to_end() {
            Ok(ast) => buf.ast = Some(ast),
            Err(errors) => {
                let err_iter = errors
                    .into_iter()
                    .map(|err| Self::error_to_diagnostic(err, &buf.src));

                diagnostics.extend(err_iter);
            }
        };

        buf.sem_model = buf.ast.as_ref().map(SemanticModel::from);

        if let Some(model) = &buf.sem_model {
            let unused_diags_iter = model
                .symbols()
                .iter()
                .filter(|sym| !sym.is_used())
                .filter_map(|sym| {
                    sym.span.as_ref().map(|span| Diagnostic {
                        range: span.clone().into(),
                        message: format!("unused {}: `{}`", sym.kind, sym.name),
                        severity: Some(DiagnosticSeverity::Hint),
                        tags: Some(vec![DiagnosticTag::Unnecessary]),
                        ..Default::default()
                    })
                });

            let not_mutated_diags_iter = model
                .symbols()
                .iter()
                .filter(|sym| sym.is_unnecessarily_mut())
                .map(|sym| {
                    let semantic::SymbolValue::MutableVariable { mut_span, .. } = &sym.kind else {
                        unreachable!()
                    };

                    Diagnostic {
                        range: mut_span.clone().into(),
                        message: format!("variable does not need to be mutable: `{}`", sym.name),
                        severity: Some(DiagnosticSeverity::Warning),
                        ..Default::default()
                    }
                });

            let sem_errors_iter = model.errors().iter().map(|err| Diagnostic {
                range: err.span().clone().into(),
                message: err.to_string(),
                severity: Some(DiagnosticSeverity::Error),
                ..Default::default()
            });

            let extra_hint_diagnostics = model.hints().iter().map(|hint| Diagnostic {
                range: hint.span().clone().into(),
                message: hint.to_string(),
                severity: Some(DiagnosticSeverity::Hint),
                tags: hint
                    .tag_unnecessary()
                    .then(|| vec![DiagnosticTag::Unnecessary]),
                ..Default::default()
            });

            diagnostics.extend(unused_diags_iter);
            diagnostics.extend(not_mutated_diags_iter);
            diagnostics.extend(sem_errors_iter);
            diagnostics.extend(extra_hint_diagnostics);
        }

        self.buffers.insert(uri.to_string(), buf);
        self.publish_diagnostics(&uri, version, diagnostics)?;
        Ok(())
    }

    fn publish_diagnostics(
        &mut self,
        uri: &str,
        version: i32,
        diagnostics: Vec<Diagnostic>,
    ) -> anyhow::Result<()> {
        let params = PublishDiagnosticsParams {
            uri: uri.to_string(),
            version: Some(version),
            diagnostics,
        };

        self.send_message(&OutNotification::PublishDiagnostics { params })
    }

    fn error_to_diagnostic(error: ParseError, src: &SourceFile) -> Diagnostic {
        let range = error.span().map_or_else(
            || {
                let start_pos: Position = src.end_pos().into();
                let end_pos = Position::new(start_pos.line, start_pos.character + 1);
                Range::new(start_pos, end_pos)
            },
            Range::from,
        );

        Diagnostic {
            range,
            code: None,
            message: format!("{}", error.with_src(src)),
            severity: Some(DiagnosticSeverity::Error),
            tags: None,
            source: None,
        }
    }
}
