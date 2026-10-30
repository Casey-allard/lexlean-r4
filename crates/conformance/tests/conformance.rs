//! Complete R4 conformance test suites.

mod support;
mod suites {
    pub mod admission;
    pub mod binder;
    pub mod blake3;
    pub mod codebook_words;
    pub mod configuration;
    pub mod digest;
    pub mod engine;
    pub mod evidence;
    pub mod flock;
    pub mod geometry;
    pub mod hash;
    pub mod json_number;
    pub mod learning;
    pub mod lexlean;
    pub mod object;
    pub mod objective;
    pub mod prediction;
    pub mod route;
    pub mod snapshot_json;
    pub mod tokenizer_json;
    pub mod tokenizer_kernel;
    pub mod tokenizer_text;
    pub mod words;
}
