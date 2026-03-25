// use derive_more::{Display, Error};
//
// #[derive(Debug, Clone)]
// pub struct FileRange {
//     pos: usize,
//     len: usize,
//     line: usize,
//     col: usize,
// }
//
// #[derive(Debug, Clone)]
// pub struct Error {
//     range: FileRange,
//     val: ErrorValue,
// }
//
// #[derive(Debug, Clone)]
// pub enum ErrorValue {
//     Scan(ScanError),
//     Parse(ParseError),
// }
//
// #[derive(Debug, Clone, Error, Display)]
// pub enum ScanError {
//     #[display("Unexpected char: '{_0}'")]
//     UnexpectedChar(#[error(ignore)] char),
//     #[display("Unexpected EOF")]
//     UnexpectedEof,
//     #[display("Integer overflow")]
//     IntegerOverflow,
// }
