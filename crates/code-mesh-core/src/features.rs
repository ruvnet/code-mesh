//! Feature flags for Code Mesh Core

/// Whether compression support is available
pub const HAS_COMPRESSION: bool = cfg!(feature = "compression");

/// Whether file watching support is available
pub const HAS_FILE_WATCHING: bool = cfg!(feature = "file-watching");

/// Whether advanced crypto support is available
pub const HAS_ADVANCED_CRYPTO: bool = cfg!(feature = "crypto");

/// Whether tokio runtime is available
pub const HAS_TOKIO: bool = cfg!(feature = "native");

/// Whether WASM support is available
pub const HAS_WASM: bool = cfg!(feature = "wasm");

/// Whether web support is available
pub const HAS_WEB: bool = cfg!(feature = "web");

/// Whether OpenAI support is available
pub const HAS_OPENAI: bool = cfg!(feature = "openai");

/// Whether Anthropic support is available
pub const HAS_ANTHROPIC: bool = cfg!(feature = "anthropic");

/// Whether Mistral support is available
pub const HAS_MISTRAL: bool = cfg!(feature = "mistral");