//! Content-Defined Chunking (CDC) algorithms.

mod fastcdc;
mod tables;

pub(crate) use fastcdc::FastCdc;
