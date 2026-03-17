use thiserror::Error;

#[derive(Debug, Error)]
pub enum GraphvizError {
    #[error("Parse error: {0}")]
    Parse(String),

    #[error("Stylesheet error: {0}")]
    Stylesheet(String),
}
