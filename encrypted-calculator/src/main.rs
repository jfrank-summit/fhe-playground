use std::env;
use std::fs::File;
use std::io::Write;
use std::time::Instant;
use tfhe::prelude::*;
use tfhe::{generate_keys, set_server_key, ConfigBuilder, FheUint32};

struct BenchmarkResult {
    operation: String,
    value1: u32,
    value2: u32,
    encrypted_result_val: u32,
    clear_result_val: u32,
    encrypted_time_ns: u128,
    clear_time_ns: u128,
    slowdown_factor: f64,
}

fn run_benchmarks() -> Result<(), Box<dyn std::error::Error>> {
    let operations = ["add", "sub", "mul", "div"];
    // Using a small set of values for demonstration.
    let value_pairs = [(10u32, 5u32), (20u32, 3u32), (100u32, 10u32), (25u32, 4u32), (7u32, 8u32), (50u32, 50u32)]; // Added one more pair for sub edge case

    let mut results = Vec::new();

    // Basic configuration to use homomorphic integers
    let config = ConfigBuilder::default().build();
    // Key generation (once for all benchmarks)
    let (client_key, server_keys) = generate_keys(config);
    // Set server key (once for all benchmarks)
    set_server_key(server_keys);

    for &operation_str in operations.iter() {
        for &(value1, value2) in value_pairs.iter() {
            // Skip division by zero for the clear operation, FHE might also panic or give unexpected results
            if operation_str == "div" && value2 == 0 {
                println!("Skipping division by zero for {} / {}", value1, value2);
                continue;
            }

            // Skip subtraction if value1 < value2 for u32 to prevent overflow
            if operation_str == "sub" && value1 < value2 {
                println!(
                    "Skipping subtraction for unsigned integers where value1 < value2: {} - {}",
                    value1,
                    value2
                );
                continue;
            }

            // Encrypting the input data
            let encrypted_a = FheUint32::try_encrypt(value1, &client_key)?;
            let encrypted_b = FheUint32::try_encrypt(value2, &client_key)?;

            // Perform encrypted operation
            let encrypted_start = Instant::now();
            let encrypted_op_result = match operation_str {
                "add" => &encrypted_a + &encrypted_b,
                "sub" => &encrypted_a - &encrypted_b,
                "mul" => &encrypted_a * &encrypted_b,
                "div" => {
                    if value2 == 0 { // FHE division by zero can panic or return large numbers
                        println!("Skipping FHE division by zero for {} / {}", value1, value2);
                        // To keep the structure, we could encrypt a placeholder or handle appropriately
                        // For now, we just skip to avoid panic, or one could encrypt a zero or one.
                        // However, decrypting it later would still require a special case.
                        // A robust solution might involve encrypting a flag or using a type that supports error states.
                        // Here, we'll just use a placeholder encryption if needed, but skipping is safer.
                        continue; // Skip this specific case for FHE as well.
                    }
                    &encrypted_a / &encrypted_b
                }
                _ => unreachable!(), // Should not happen due to operations array
            };
            let encrypted_elapsed = encrypted_start.elapsed();

            // Decrypting on the client side:
            let encrypted_clear_result: u32 = encrypted_op_result.decrypt(&client_key);

            // Perform clear operation (with iterations for better timing)
            const CLEAR_OP_ITERATIONS: u32 = 1_000_000;
            let clear_start = Instant::now();
            let mut clear_op_result = 0u32; // Initialize with a default
            for _ in 0..CLEAR_OP_ITERATIONS {
                clear_op_result = match operation_str {
                    "add" => value1.wrapping_add(value2), // Use wrapping_add for benchmark consistency
                    "sub" => value1.wrapping_sub(value2), // Use wrapping_sub, already checked v1 >= v2
                    "mul" => value1.wrapping_mul(value2), // Use wrapping_mul
                    "div" => value1 / value2, // Already checked for value2 == 0
                    _ => unreachable!(),
                };
            }
            let clear_elapsed_total = clear_start.elapsed();
            let clear_elapsed_per_op = if CLEAR_OP_ITERATIONS > 0 {
                clear_elapsed_total.as_nanos() / CLEAR_OP_ITERATIONS as u128
            } else {
                clear_elapsed_total.as_nanos()
            };

            let slowdown = if clear_elapsed_per_op > 0 {
                encrypted_elapsed.as_nanos() as f64 / clear_elapsed_per_op as f64
            } else {
                0.0 // Avoid division by zero if clear operation was instantaneous 
            };

            results.push(BenchmarkResult {
                operation: operation_str.to_string(),
                value1,
                value2,
                encrypted_result_val: encrypted_clear_result,
                clear_result_val: clear_op_result, // This is the result of the last iteration
                encrypted_time_ns: encrypted_elapsed.as_nanos(),
                clear_time_ns: clear_elapsed_per_op,
                slowdown_factor: slowdown,
            });

            println!(
                "Op: {}, V1: {}, V2: {}, EncRes: {}, ClearRes: {}, EncT: {:?}, ClearT_avg: {:?}ns, Slowdown: {:.2}x",
                operation_str, value1, value2, encrypted_clear_result, clear_op_result, encrypted_elapsed, clear_elapsed_per_op, slowdown
            );
        }
    }

    // Write results to CSV
    let mut file = File::create("benchmark_results.csv")?;
    writeln!(
        file,
        "Operation,Value1,Value2,EncryptedResult,ClearResult,EncryptedTime_ns,ClearTime_ns,SlowdownFactor"
    )?;
    for res in results {
        writeln!(
            file,
            "{},{},{},{},{},{},{},{:.2}",
            res.operation,
            res.value1,
            res.value2,
            res.encrypted_result_val,
            res.clear_result_val,
            res.encrypted_time_ns,
            res.clear_time_ns,
            res.slowdown_factor
        )?;
    }
    println!("Benchmark results saved to benchmark_results.csv");

    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // The original command-line argument processing is removed.
    // We now directly call the benchmark runner.
    run_benchmarks()
}
