//! TL-PX 0.2 authority skeleton.
//!
//! This crate is not a 0.1 rewrite and not a PEP. It owns contract types and
//! the Northstar JCS/hash oracle so Rust cannot drift from the 0.2 fixtures.

pub mod error;
pub mod hash;
pub mod jcs;
pub mod types;

pub use error::{Error, Result};
pub use hash::{
    approval_context_hash, assert_hash_string, authorized_action_hash, digest_hex,
    executed_action_hash, hash_canonical, hash_json_text, hash_value, intent_hash,
};
pub use jcs::{canonicalize, canonicalize_json_text, parse, utf8_hex, Canonical, Value};
pub use types::{
    bindings_match, ActionBinding, Adapter, AuthorizedAction, ExecutedAction, Risk, SubmittedIntent,
};
