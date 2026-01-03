use anyhow::{Context, Result};
use std::path::Path;
use std::process::Command;

pub fn compute_fingerprint(path: &Path) -> Result<(f64, String)> {
    // Call fpcalc
    let output = Command::new("fpcalc").arg(path).output();

    let output = match output {
        Ok(o) => o,
        Err(e) => {
            if e.kind() == std::io::ErrorKind::NotFound {
                return Err(anyhow::anyhow!("'fpcalc' not found. Please install Chromaprint/fpcalc and add it to your PATH. Download from: https://acoustid.org/chromaprint"));
            }
            return Err(e.into());
        }
    };

    if !output.status.success() {
        return Err(anyhow::anyhow!(
            "fpcalc failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    let stdout = String::from_utf8(output.stdout).context("Invalid UTF-8 from fpcalc")?;

    let mut duration = 0.0;
    let mut fingerprint = String::new();

    for line in stdout.lines() {
        if let Some(val) = line.strip_prefix("DURATION=") {
            duration = val.parse().unwrap_or(0.0);
        } else if let Some(val) = line.strip_prefix("FINGERPRINT=") {
            fingerprint = val.to_string();
        }
    }

    if fingerprint.is_empty() {
        return Err(anyhow::anyhow!("No fingerprint returned by fpcalc"));
    }

    Ok((duration, fingerprint))
}
