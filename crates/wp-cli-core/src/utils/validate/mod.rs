//! Validation utility functions
//!
//! This module provides functions for validating data integrity,
//! checking thresholds, and other validation operations.

mod validate;
pub use validate::{validate_groups, validate_with_stats};
