use std::error::Error;
use std::fmt::{Debug, Display};
use std::path::PathBuf;
use std::sync::Arc;

use fluent::FluentError;
use thiserror::Error;

pub type NResult = core::result::Result<(), LunarisError>;
pub type Result<T> = core::result::Result<T, LunarisError>;
#[derive(Debug, Error)]
pub enum LunarisError {
    /// Generic error.
    /// Refrain from using as much as possible.
    #[error("Unknown error occurred: {context:?}")]
    Unknown { context: Option<String> },

    /// Tried to use feature that was not supported.
    #[error("Feature not supported by plugin: {feature}")]
    Unsupported { feature: &'static str },

    /// Tried  to invoke a command with wrong arguments.
    #[error("Invalid argument: {name} - {reason:?}")]
    InvalidArgument {
        name: String,
        reason: Option<String>,
    },

    /// Element Property Enum mismatch
    #[error("Property Enum Type mismatch: Expected {expected_variant}, instead got {variant}")]
    PropertyTypeMismatch {
        expected_variant: String,
        variant: String,
    },

    /// Resource not initialized yet.
    #[error("Tried to access resource which was not initialized: {resource}")]
    Uninit { resource: String },

    /// Found a null pointer.
    /// This is pretty bad.
    #[error("Null pointer at {location}")]
    NullPointer { location: &'static str },

    /// Out of memory. Self-explanatory.
    #[error("Out of memory")]
    OutOfMemory,

    /// Command timed out.
    #[error("Timeout after {duration_ms}ms")]
    Timeout { duration_ms: u64 },

    /// some sort of interrupt
    #[error("Interrupted during {during}")]
    Interrupted { during: &'static str },

    /// Tried to access a resource but was denied.
    #[error("Resource busy: {resource}")]
    Busy { resource: String },

    /// Tried to do an operation but failed.
    #[error("Permission denied for operation: {operation}")]
    PermissionDenied { operation: String },

    /// Tried to do an operation that lacked support.
    #[error("Operation not supported: {operation}")]
    NotSupported { operation: &'static str },

    /// Tried to create a duplicate item
    #[error("Item already exists: {item}")]
    AlreadyExists { item: String },

    /// Tried to find resource from somewhere but failed.
    #[error("Item not found: {item}")]
    NotFound { item: String },

    /// Envelope failed to send.
    #[error("Invalid envelope, expected: {expected}")]
    InvalidEnvelope { expected: String },

    /// Message size was too large to hold. Maybe try to split it up.
    #[error("Message too large: {size} bytes")]
    MessageTooLarge { size: usize },

    /// Failed to find destination for envelope.
    #[error("Invalid destination ID: {id}")]
    InvalidDestination { id: u32 },

    // Kernel/System-level errors
    /// Failed to initialize kernel. Very bad news.
    #[error("Kernel initialization failed: {reason}")]
    KernelInitFailed { reason: String },

    /// Kernel internally panicked. Not a plugin's fault(probably)
    #[error("Kernel panic: {reason}")]
    KernelPanic { reason: String },

    /// Kernel contents are off. Maybe something is poking at its memory in a bad way.
    #[error("Invalid state: expected {expected}, found {found}")]
    InvalidState { expected: String, found: String },

    /// Found a deadlock. Will try to kill that command.
    #[error("Deadlock detected in {component}")]
    DeadlockDetected { component: String },

    /// Tried to shut down while shutting down.
    #[error("Shutdown already in progress")]
    ShutdownInProgress,

    // Renderer-related errors
    /// renderer failed to initialize
    #[error("Renderer initialization failed: {reason}")]
    RenderInitFailed { reason: String },

    /// GPU device or CPU device failed.
    #[error("Render device lost")]
    RenderDeviceLost,

    /// Ran out of VRAM.
    #[error("Render ran out of memory")]
    RenderOutOfMemory,

    /// Too much rendering queue contents.
    #[error("Render queue is full")]
    RenderQueueFull,

    #[error(
        "Tried to add frames of different size: {a:?} and {b:?}. NOTE: use the universal frame size."
    )]
    Dimensionmismatch {
        a: (usize, usize),
        b: (usize, usize),
    },

    /// Took too much time to render.
    #[error("Render timeout during: {stage}")]
    RenderTimeout { stage: &'static str },

    #[error("Plugin doesn not support feature: {feature}")]
    PluginFeatureUnsupported { feature: &'static str },

    #[error("Could not find plugin with name: {name}")]
    PluginNameNotFound { name: String },

    #[error("Plugin {id} crashed. {backtrace:?}")]
    PluginPanicked {
        id: String,
        backtrace: Option<String>,
    },

    #[error("Plugin {id} failed to acknowledge opcode {opcode}")]
    PluginAckTimeout { id: String, opcode: u32 },

    // File IO / Resource loading
    #[error("File not found: {path:?}")]
    FileNotFound { path: PathBuf },

    #[error("Failed to read file: {path:?}, reason: {reason}")]
    FileReadError { path: PathBuf, reason: String },

    #[error("Failed to write file: {path:?}, reason: {reason}")]
    FileWriteError { path: PathBuf, reason: String },

    #[error("File corrupted: {path:?}")]
    FileCorrupted { path: PathBuf },

    #[error("Invalid path: {reason}")]
    InvalidPath { reason: String },

    // Config / runtime environment
    #[error("Invalid config key: {key}, reason: {reason:?}")]
    ConfigInvalid { key: String, reason: Option<String> },

    #[error("Missing config key: {key}")]
    ConfigMissing { key: String },

    #[error("Config mismatch: expected {expected}, found {found}")]
    ConfigMismatch { expected: String, found: String },

    #[error("Missing environment variable: {name}")]
    EnvVariableMissing { name: String },

    #[error("Resource unavailable: {name}")]
    ResourceUnavailable { name: String },

    // Audio / MIDI backend
    #[error("Audio initialization failed: {reason}")]
    AudioInitFailed { reason: String },

    #[error("Audio device unavailable: {name:?}")]
    AudioDeviceUnavailable { name: Option<String> },

    #[error("Audio stream error: {reason}")]
    AudioStreamError { reason: String },

    // i18n(fluent) error
    #[error("Fluent failed: {0}")]
    FluentError(#[from] FluentErrorWrapper),

    // Dynamic plugin error wrapping
    #[error("Plugin {id} returned an error: {source:?}")]
    PluginError {
        id: String,
        #[source]
        source: Arc<dyn Error + Send + Sync>,
    },
}

#[derive(Debug, Error)]
#[repr(transparent)]
pub struct FluentErrorWrapper {
    inner: Vec<FluentError>,
}

impl Display for FluentErrorWrapper {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

impl From<Vec<FluentError>> for FluentErrorWrapper {
    fn from(value: Vec<FluentError>) -> Self {
        Self { inner: value }
    }
}
