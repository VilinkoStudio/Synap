//! Command implementations for Synap CLI with implicit command detection support.

pub mod amend; // Immutable correction
pub mod capture; // Zero-friction capture
pub mod graph; // Graph visualization (backward compatibility)
pub mod list; // List thoughts (backward compatibility)
pub mod reply; // Thought extension
pub mod scrub; // ZFS-style pruning
pub mod search; // Search with #tag support
pub mod show; // Display with short ID support
pub mod stats; // Statistics
pub mod sync; // P2P synchronization
pub mod tag; // Tag management (backward compatibility)
pub mod trace; // Graph topology tracing
pub mod void; // Abandon thoughts
