//! Spec vector tests — run deterministic test vectors from spec_tests/test_vectors.json
//! and export results for cross-validation against the Python reference implementation.

use std::collections::HashMap;
use tesseract::{Coord, Dimension, Field};

fn parse_dimension(s: &str) -> Dimension {
    match s {
        "Temporal" => Dimension::Temporal,
        "Context" => Dimension::Context,
        "Origin" => Dimension::Origin,
        "Verification" => Dimension::Verification,
        _ => panic!("unknown dimension: {s}"),
    }
}

#[derive(serde::Deserialize)]
struct TestVectors {
    version: String,
    tests: Vec<TestCase>,
}

#[derive(serde::Deserialize)]
struct TestCase {
    id: String,
    name: String,
    input: TestInput,
    expected: HashMap<String, serde_json::Value>,
}

#[derive(serde::Deserialize)]
struct TestInput {
    field_size: usize,
    #[serde(default)]
    attestations: Vec<AttestationInput>,
    coord_a: Option<Vec<usize>>,
    coord_b: Option<Vec<usize>>,
    #[serde(default)]
    causal_graph: Option<serde_json::Value>,
}

#[derive(serde::Deserialize)]
struct AttestationInput {
    coord: Vec<usize>,
    event_id: String,
    dimension: String,
    validator_id: String,
}

fn coord_from_vec(v: &[usize]) -> Coord {
    Coord { t: v[0], c: v[1], o: v[2], v: v[3] }
}

#[derive(serde::Serialize)]
struct TestResult {
    id: String,
    results: HashMap<String, serde_json::Value>,
    pass_: bool,
}

fn run_test(test: &TestCase) -> (HashMap<String, serde_json::Value>, Vec<String>) {
    let mut results: HashMap<String, serde_json::Value> = HashMap::new();
    let mut field = Field::new(test.input.field_size);

    // Apply attestations
    for att in &test.input.attestations {
        let coord = coord_from_vec(&att.coord);
        let dim = parse_dimension(&att.dimension);
        field.attest(coord, &att.event_id, dim, &att.validator_id);
    }

    // Distance tests
    if let (Some(a), Some(b)) = (&test.input.coord_a, &test.input.coord_b) {
        let ca = coord_from_vec(a);
        let cb = coord_from_vec(b);
        let d = tesseract::distance(ca, cb, test.input.field_size);
        results.insert("distance".into(), serde_json::json!(d));
    }

    // Center checks
    if !test.input.attestations.is_empty() {
        let center = coord_from_vec(&test.input.attestations[0].coord);
        let cell = field.get(center);
        results.insert("sigma_at_center".into(), serde_json::json!(cell.sigma_independence()));
        results.insert("crystallized_at_center".into(), serde_json::json!(cell.crystallized));
        results.insert("probability_at_center".into(), serde_json::json!(cell.probability));
        results.insert("raw_sigma".into(), serde_json::json!(cell.sigma_independence()));

        // sigma_eff
        let sigma_eff = tesseract::adversarial::effective_sigma(&field, center, None);
        results.insert("sigma_eff".into(), serde_json::json!(sigma_eff.sigma_eff));
    }

    results.insert("active_cells".into(), serde_json::json!(field.active_cells()));
    results.insert("crystallized_count".into(), serde_json::json!(field.crystallized_count()));

    // Check expected
    let mut failures = Vec::new();
    for (key, exp_val) in &test.expected {
        if key.ends_with("_gt") {
            let actual_key = &key[..key.len() - 3];
            if let Some(actual) = results.get(actual_key) {
                if let (Some(a), Some(e)) = (actual.as_f64(), exp_val.as_f64()) {
                    if a <= e {
                        failures.push(format!("  {key}: expected > {e}, got {a}"));
                    }
                } else if let (Some(a), Some(e)) = (actual.as_u64(), exp_val.as_u64()) {
                    if a <= e {
                        failures.push(format!("  {key}: expected > {e}, got {a}"));
                    }
                }
            }
        } else if key.ends_with("_lte") {
            let actual_key = &key[..key.len() - 4];
            if let Some(actual) = results.get(actual_key) {
                if let (Some(a), Some(e)) = (actual.as_u64(), exp_val.as_u64()) {
                    if a > e {
                        failures.push(format!("  {key}: expected <= {e}, got {a}"));
                    }
                }
            }
        } else if key.ends_with("_tolerance") {
            continue;
        } else if let Some(actual) = results.get(key.as_str()) {
            let tolerance_key = format!("{key}_tolerance");
            let tolerance = test.expected.get(&tolerance_key)
                .and_then(|v| v.as_f64()).unwrap_or(0.0);

            if let (Some(a), Some(e)) = (actual.as_f64(), exp_val.as_f64()) {
                if (a - e).abs() > tolerance + 1e-10 {
                    failures.push(format!("  {key}: expected {e} (+/-{tolerance}), got {a}"));
                }
            } else if let (Some(a), Some(e)) = (actual.as_bool(), exp_val.as_bool()) {
                if a != e {
                    failures.push(format!("  {key}: expected {e}, got {a}"));
                }
            } else if let (Some(a), Some(e)) = (actual.as_u64(), exp_val.as_u64()) {
                if a != e {
                    failures.push(format!("  {key}: expected {e}, got {a}"));
                }
            }
        }
    }

    (results, failures)
}

#[test]
fn spec_vectors_all_pass() {
    let vectors_path = concat!(env!("CARGO_MANIFEST_DIR"), "/spec_tests/test_vectors.json");
    let data: TestVectors = serde_json::from_str(
        &std::fs::read_to_string(vectors_path).expect("test_vectors.json not found")
    ).expect("invalid JSON");

    println!("Tesseract Rust Implementation");
    println!("Test vectors version: {}", data.version);
    println!("Running {} tests...\n", data.tests.len());

    let mut passed = 0;
    let mut failed = 0;
    let mut all_results = Vec::new();

    for test in &data.tests {
        let (results, failures) = run_test(test);
        let ok = failures.is_empty();

        all_results.push(TestResult {
            id: test.id.clone(),
            results,
            pass_: ok,
        });

        if ok {
            println!("  OK {}: {}", test.id, test.name);
            passed += 1;
        } else {
            println!("FAIL {}: {}", test.id, test.name);
            for f in &failures {
                println!("{f}");
            }
            failed += 1;
        }
    }

    println!("\n{passed} passed, {failed} failed, {} total", passed + failed);

    // Export for cross-validation
    let out_path = concat!(env!("CARGO_MANIFEST_DIR"), "/spec_tests/rust_results.json");
    let json = serde_json::to_string_pretty(&all_results).unwrap();
    std::fs::write(out_path, &json).expect("failed to write rust_results.json");
    println!("Results exported to {out_path}");

    assert_eq!(failed, 0, "{failed} spec vector tests failed");
}
