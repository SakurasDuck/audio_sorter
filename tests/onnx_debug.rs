//! Integration test to debug ONNX model input/output names and inference

#[cfg(feature = "genre-onnx")]
#[test]
fn test_onnx_model_io_names() {
    use ort::session::{builder::GraphOptimizationLevel, Session};
    use std::path::Path;

    // Initialize ONNX Runtime
    let _ = ort::init().with_name("test_classifier").commit();

    let model_dir = Path::new("./assets/models");
    let embedding_path = model_dir.join("discogs-effnet-bs64.onnx");
    let classifier_path = model_dir.join("mtg_jamendo_genre-discogs-effnet.onnx");

    println!("\n=== Testing ONNX Model I/O Names ===\n");

    // Check if models exist
    if !embedding_path.exists() {
        println!("ERROR: Embedding model not found at {:?}", embedding_path);
        return;
    }
    if !classifier_path.exists() {
        println!("ERROR: Classifier model not found at {:?}", classifier_path);
        return;
    }

    println!("Loading embedding model from {:?}...", embedding_path);
    let embedding_session = Session::builder()
        .unwrap()
        .with_optimization_level(GraphOptimizationLevel::Level3)
        .unwrap()
        .commit_from_file(&embedding_path)
        .expect("Failed to load embedding model");

    println!("\n--- Embedding Model (discogs-effnet-bs64.onnx) ---");
    println!("Inputs:");
    for (i, input) in embedding_session.inputs().iter().enumerate() {
        println!(
            "  [{}] Name: '{}', Type: {:?}",
            i,
            input.name(),
            input.dtype()
        );
    }
    println!("Outputs:");
    for (i, output) in embedding_session.outputs().iter().enumerate() {
        println!(
            "  [{}] Name: '{}', Type: {:?}",
            i,
            output.name(),
            output.dtype()
        );
    }

    println!("\nLoading classifier model from {:?}...", classifier_path);
    let classifier_session = Session::builder()
        .unwrap()
        .with_optimization_level(GraphOptimizationLevel::Level3)
        .unwrap()
        .commit_from_file(&classifier_path)
        .expect("Failed to load classifier model");

    println!("\n--- Classifier Model (mtg_jamendo_genre-discogs-effnet.onnx) ---");
    println!("Inputs:");
    for (i, input) in classifier_session.inputs().iter().enumerate() {
        println!(
            "  [{}] Name: '{}', Type: {:?}",
            i,
            input.name(),
            input.dtype()
        );
    }
    println!("Outputs:");
    for (i, output) in classifier_session.outputs().iter().enumerate() {
        println!(
            "  [{}] Name: '{}', Type: {:?}",
            i,
            output.name(),
            output.dtype()
        );
    }

    println!("\n=== Test Complete ===\n");
}

#[cfg(feature = "genre-onnx")]
#[test]
fn test_embedding_inference() {
    use ndarray::Array4;
    use ort::session::{builder::GraphOptimizationLevel, Session};
    use ort::value::Value;
    use std::path::Path;

    // Initialize ONNX Runtime
    let _ = ort::init().with_name("test_inference").commit();

    let model_dir = Path::new("./assets/models");
    let embedding_path = model_dir.join("discogs-effnet-bs64.onnx");

    if !embedding_path.exists() {
        println!("Skipping test: model not found");
        return;
    }

    println!("\n=== Testing Embedding Inference ===\n");

    let mut session = Session::builder()
        .unwrap()
        .with_optimization_level(GraphOptimizationLevel::Level3)
        .unwrap()
        .commit_from_file(&embedding_path)
        .expect("Failed to load model");

    // Get actual input name from the model
    let input_name = session.inputs()[0].name().to_string();
    println!("Using input name: '{}'", input_name);

    // Create a dummy input tensor (batch=1, channels=1, frames=128, mels=96)
    let input_tensor = Array4::<f32>::zeros((1, 1, 128, 96));
    let shape = input_tensor.shape().to_vec();
    let data = input_tensor.into_raw_vec();

    let input_value = Value::from_array((shape, data)).expect("Failed to create input value");

    // Use the actual input name from the model
    let inputs = ort::inputs![input_name.as_str() => &input_value];

    println!("Running inference...");
    match session.run(inputs) {
        Ok(outputs) => {
            println!("SUCCESS! Inference completed.");
            for (name, value) in outputs.iter() {
                if let Ok((shape, _data)) = value.try_extract_tensor::<f32>() {
                    println!("  Output '{}': shape = {:?}", name, shape);
                }
            }
        }
        Err(e) => {
            println!("FAILED: {:?}", e);
        }
    }
}
