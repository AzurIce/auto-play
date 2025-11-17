use thiserror::Error;

/// Unified ADB error type
#[derive(Error, Debug)]
pub enum AdbError {
    /// IO error
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Image processing error
    #[error("Image error: {0}")]
    Image(#[from] image::ImageError),

    /// Device not found
    #[error("Device not found: {0}")]
    DeviceNotFound(String),

    /// ADB server not connected
    #[error("ADB server not connected")]
    ServerNotConnected,

    /// ADB response error
    #[error("ADB response error: {0}")]
    ResponseError(String),

    /// Parse error
    #[error("Parse error: {0}")]
    ParseError(String),

    /// UTF-8 decode error
    #[error("UTF-8 decode error: {0}")]
    Utf8Error(#[from] std::str::Utf8Error),

    /// Device info parse error
    #[error("Failed to parse device info: {0}")]
    DeviceInfoParseError(String),

    /// Hex parse error
    #[error("Hex parse error: {0}")]
    HexParseError(#[from] std::num::ParseIntError),

    /// Unknown response status
    #[error("Unknown response status: {0}")]
    UnknownResponseStatus(String),

    /// Command execution failed
    #[error("Command execution failed: {0}")]
    CommandFailed(String),

    /// Timeout error
    #[error("Operation timed out")]
    Timeout,

    /// Protocol error
    #[error("Protocol error: {0}")]
    ProtocolError(String),
}

/// ADB result type alias
pub type AdbResult<T> = Result<T, AdbError>;

