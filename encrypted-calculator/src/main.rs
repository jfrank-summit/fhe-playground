use std::env;
use std::time::Instant;
use tfhe::prelude::*;
use tfhe::{generate_keys, set_server_key, ConfigBuilder, FheUint32};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 4 {
        eprintln!("Usage: {} <operation> <value1> <value2>", args[0]);
        eprintln!("Operations: add, sub, mul, div");
        std::process::exit(1);
    }

    let operation = &args[1];
    let value1: u32 = args[2].parse()?;
    let value2: u32 = args[3].parse()?;

    // Basic configuration to use homomorphic integers
    let config = ConfigBuilder::default().build();

    // Key generation
    let (client_key, server_keys) = generate_keys(config);

    // Encrypting the input data
    let encrypted_a = FheUint32::try_encrypt(value1, &client_key)?;
    let encrypted_b = FheUint32::try_encrypt(value2, &client_key)?;

    // On the server side:
    set_server_key(server_keys);

    // Perform encrypted operation
    let encrypted_start = Instant::now();
    let encrypted_result = match operation.as_str() {
        "add" => &encrypted_a + &encrypted_b,
        "sub" => &encrypted_a - &encrypted_b,
        "mul" => &encrypted_a * &encrypted_b,
        "div" => &encrypted_a / &encrypted_b,
        _ => {
            eprintln!("Invalid operation. Use add, sub, mul, or div.");
            std::process::exit(1);
        }
    };
    let encrypted_elapsed = encrypted_start.elapsed();

    // Decrypting on the client side:
    let encrypted_clear_result: u32 = encrypted_result.decrypt(&client_key);

    // Perform clear operation
    let clear_start = Instant::now();
    let clear_result = match operation.as_str() {
        "add" => value1 + value2,
        "sub" => value1 - value2,
        "mul" => value1 * value2,
        "div" => value1 / value2,
        _ => unreachable!(),
    };
    let clear_elapsed = clear_start.elapsed();

    println!("Encrypted Result: {}", encrypted_clear_result);
    println!("Clear Result: {}", clear_result);
    println!("Time taken (encrypted): {:?}", encrypted_elapsed);
    println!("Time taken (clear): {:?}", clear_elapsed);
    println!(
        "Slowdown factor: {:.2}x",
        encrypted_elapsed.as_secs_f64() / clear_elapsed.as_secs_f64()
    );

    Ok(())
}
