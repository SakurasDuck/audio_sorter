//! Test classifier with 2D input

#[cfg(feature = "genre-onnx")]
fn main() {
    use ndarray::Array2;
    use ort::session::{builder::GraphOptimizationLevel, Session};
    use ort::value::Value;
    use std::io::Write;
    use std::path::Path;

    let _ = ort::init().with_name("test").commit();

    let model_path = Path::new("./assets/models/mtg_jamendo_genre-discogs-effnet.onnx");

    let mut session = Session::builder()
        .and_then(|b| b.with_optimization_level(GraphOptimizationLevel::Level1))
        .and_then(|b| b.commit_from_file(model_path))
        .expect("Failed to load model");

    // Create test input with random-ish values (2D: [1, 1280])
    let mut input_2d = Array2::<f32>::zeros((1, 1280));
    for i in 0..1280 {
        input_2d[[0, i]] = ((i as f32 * 3.14159).sin() + 1.0) / 2.0;
    }
    let shape = input_2d.shape().to_vec();
    let data = input_2d.into_raw_vec();

    println!("Input shape: {:?}", shape);

    let input_value = Value::from_array((shape, data)).unwrap();
    let inputs = ort::inputs!["embeddings" => &input_value];

    let mut file = std::fs::File::create("classifier_output.txt").unwrap();

    match session.run(inputs) {
        Ok(out) => {
            if let Some(val) = out.get("activations") {
                if let Ok((shape, data)) = val.try_extract_tensor::<f32>() {
                    writeln!(file, "Output shape: {:?}", shape).unwrap();
                    writeln!(file, "Output length: {}", data.len()).unwrap();

                    // Count non-trivial predictions
                    let non_zero: Vec<(usize, f32)> = data
                        .iter()
                        .enumerate()
                        .filter(|(_, v)| **v > 0.001)
                        .map(|(i, v)| (i, *v))
                        .collect();
                    writeln!(file, "Predictions > 0.001: {}", non_zero.len()).unwrap();

                    writeln!(file, "\nAll {} values:", data.len()).unwrap();
                    for (i, v) in data.iter().enumerate() {
                        writeln!(file, "  [{}] = {:.6}", i, v).unwrap();
                    }
                }
            } else {
                writeln!(file, "No 'activations' output found").unwrap();
                for (name, _) in out.iter() {
                    writeln!(file, "  Found: '{}'", name).unwrap();
                }
            }
        }
        Err(e) => writeln!(file, "Error: {:?}", e).unwrap(),
    }

    println!("Written to classifier_output.txt");
}

#[cfg(not(feature = "genre-onnx"))]
fn main() {}
