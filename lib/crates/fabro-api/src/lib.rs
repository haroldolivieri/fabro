#[allow(
    clippy::absolute_paths,
    clippy::all,
    clippy::derivable_impls,
    clippy::disallowed_methods,
    clippy::disallowed_types,
    clippy::needless_lifetimes,
    clippy::unwrap_used,
    unreachable_pub,
    unused_imports,
    reason = "Generated OpenAPI client code intentionally preserves codegen output."
)]
mod generated {
    include!(concat!(env!("OUT_DIR"), "/codegen.rs"));
}
pub use generated::{Client as ApiClient, types};
