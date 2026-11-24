//! Pure learning core of refl.
//!
//! This crate performs no I/O and reads no clock or randomness: every input,
//! including time, arrives as an argument. The default build is `no_std` +
//! `alloc`, which enforces that (and leaves only `BTreeMap`/`Vec` for
//! collections and `libm` for float math), so native and WASM builds compute
//! bit-identical results. See `docs/01-architecture.md` and
//! `docs/03-layer2-learner-model.md` ("Реализация").
//!
//! Domain types follow `docs/02-layer1-content.md`.

#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

pub mod answer;
pub mod bundle;
pub mod content;
pub mod event;
pub mod ids;
pub mod item;
pub mod lesson;
pub mod unlock;
pub mod value;

pub use event::{CURRENT_SCHEMA_VERSION, DecodeError, Event, decode_event, encode_event};
