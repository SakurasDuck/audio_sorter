//! Debug candidate ONNX model input shapes

#[cfg(feature = "genre-onnx")]
fn main() {
    use ort::session::{builder::GraphOptimizationLevel, Session};
    use std::path::Path;

    let _ = ort::init().with_name("debug").commit();

    let m1 = Path::new("./assets/models/candidate_model.onnx");

    if let Ok(s) = Session::builder()
        .and_then(|b| b.with_optimization_level(GraphOptimizationLevel::Level1))
        .and_then(|b| b.commit_from_file(m1))
    {
        eprintln!("=== Candidate Embedding Model ===");
        for input in s.inputs() {
            eprintln!("Input: name='{}', dtype={:?}", input.name(), input.dtype());
        }
        for output in s.outputs() {
            eprintln!(
                "Output: name='{}', dtype={:?}",
                output.name(),
                output.dtype()
            );
        }
    }
}

#[cfg(not(feature = "genre-onnx"))]
fn main() {
    eprintln!("Need genre-onnx feature");
}
