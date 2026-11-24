//! Deterministic answer checking (`docs/02-layer1-content.md`, «Проверка
//! ответов на ввод»): normalization, the template language and the automata
//! it compiles to. content-build parses and compiles; the runtime only
//! normalizes answers and executes compiled automata.

pub mod dfa;
pub mod normalize;
pub mod template;
pub mod typo;
