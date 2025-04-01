use std::collections::HashMap;
use std::fs;
use std::io::{self};
use image::{DynamicImage, GenericImageView};
use log::{info, error};
use prettytable::{Table, Row, Cell};
use webp::Encoder;

/// Reads an image from the specified file path, returning a Result to handle errors gracefully.
fn read_image(image_path: &str) -> Result<DynamicImage, String> {
    image::open(image_path).map_err(|_| {
        format!(
            "Error: Unable to open the image file '{}'. Please ensure it exists and is a valid JPG or PNG.",
            image_path
        )
    })
}

/// Calculates entropy of a given data array.
fn calculate_entropy(image_data: &[u8]) -> f64 {
    let mut histogram = HashMap::new();
    let total_pixels = image_data.len() as f64;

    for &value in image_data {
        *histogram.entry(value).or_insert(0) += 1;
    }

    histogram
        .iter()
        .map(|(_, &count)| {
            let probability = count as f64 / total_pixels;
            -probability * probability.log2()
        })
        .sum()
}

/// Splits the image into its red, green, and blue color channels.
fn split_rgb_channels(img: &DynamicImage) -> (Vec<u8>, Vec<u8>, Vec<u8>) {
    let (width, height) = img.dimensions();
    let mut red_channel = Vec::with_capacity((width * height) as usize);
    let mut green_channel = Vec::with_capacity((width * height) as usize);
    let mut blue_channel = Vec::with_capacity((width * height) as usize);

    for pixel in img.pixels() {
        let [r, g, b, _] = pixel.2 .0; // Access inner array using `.0`
        red_channel.push(r);
        green_channel.push(g);
        blue_channel.push(b);
    }

    (red_channel, green_channel, blue_channel)
}

/// Compresses the image using lossless WebP compression.
fn webp_compress(image: &DynamicImage) -> usize {
    let encoder = Encoder::from_image(image).expect("Failed to create WebP encoder");
    let webp_data = encoder.encode_lossless(); // Lossless WebP compression
    webp_data.len()
}

/// Displays results in a formatted table for better readability.
fn display_results(
    red_entropy: f64,
    green_entropy: f64,
    blue_entropy: f64,
    total_entropy: f64,
    original_size: usize,
    theoretical_size: f64,
    webp_compressed_size: usize,
    compression_percentage: f64,
    model_message: &str,
) {
    let mut table = Table::new();
    table.add_row(Row::new(vec![Cell::new("Color Channel"), Cell::new("Entropy (bits/pixel)")])); // Header
    table.add_row(Row::new(vec![Cell::new("Red"), Cell::new(&format!("{:.2}", red_entropy))]));
    table.add_row(Row::new(vec![Cell::new("Green"), Cell::new(&format!("{:.2}", green_entropy))]));
    table.add_row(Row::new(vec![Cell::new("Blue"), Cell::new(&format!("{:.2}", blue_entropy))]));
    table.add_row(Row::new(vec![Cell::new("Total"), Cell::new(&format!("{:.2}", total_entropy))]));
    table.printstd();

    println!("Original Size: {} bytes", original_size);
    println!("{}", model_message); // Display model message instead of invalid theoretical size
    println!(
        "Compression Percentage (Based on Entropy Limit): {:.2}%",
        compression_percentage
    );

    println!("WebP Compressed Size (bytes): {}", webp_compressed_size);

    // Suppress unused variable warning by prefixing with an underscore
    let _theoretical_size = theoretical_size;

    if compression_percentage <= 0.0 {
        println!("Note: This file is already highly compressed and may not benefit from additional compression.");
    }
}

fn main() {
    env_logger::init(); // Initialize logger
    info!("Program started...");

    println!("Enter the path to the image file:");

    let mut path = String::new();
    io::stdin()
        .read_line(&mut path)
        .expect("Failed to read input.");
    let path = path.trim(); // Trim newline or extra spaces

    // Validate file path and format
    let ext = path.split('.').last().unwrap_or("").to_lowercase();
    if !["jpg", "jpeg", "png"].contains(&ext.as_str()) {
        println!("Unsupported file format. Please use JPG or PNG files.");
        return;
    }

    // Get the actual file size from metadata
    let metadata = match fs::metadata(path) {
        Ok(meta) => meta,
        Err(err) => {
            error!("Error accessing file metadata: {}", err);
            println!("Failed to access file: {}", err); // User-friendly error message
            return;
        },
    };
    let file_size = metadata.len(); // File size in bytes

    let img = match read_image(path) {
        Ok(image) => image,
        Err(error_message) => {
            error!("{}", error_message);
            println!("{}", error_message); // User-friendly error message
            return;
        },
    };
    info!("Image successfully loaded.");

    let (width, height) = img.dimensions();
    let total_pixels = (width * height) as f64;

    // Separate data into color channels
    let (red_channel, green_channel, blue_channel) = split_rgb_channels(&img);

    // Calculate entropies in parallel
    let red_entropy = calculate_entropy(&red_channel);
    let green_entropy = calculate_entropy(&green_channel);
    let blue_entropy = calculate_entropy(&blue_channel);

    let total_entropy = red_entropy + green_entropy + blue_entropy;

    // Calculate theoretical lossless limit
    let mut theoretical_minimum_size = (total_entropy * total_pixels) / 8.0;
    let mut model_message = format!(
        "Theoretical Minimum Size (Lossless Limit): {:.2} bytes",
        theoretical_minimum_size
    );

    // Ensure theoretical size does not exceed original size
    if theoretical_minimum_size > file_size as f64 {
        theoretical_minimum_size = file_size as f64;
        model_message = String::from(
            "The model isn't effective enough to predict a better compression for this image.",
        );
    }

    // Compress the image using lossless WebP compression
    let webp_compressed_size = webp_compress(&img);

    // Calculate compression percentage
    let compression_percentage = if theoretical_minimum_size > file_size as f64 {
        0.0 // No further compression is achievable
    } else {
        (1.0 - theoretical_minimum_size / file_size as f64) * 100.0
    };

    // Display results
    display_results(
        red_entropy,
        green_entropy,
        blue_entropy,
        total_entropy,
        file_size as usize,
        theoretical_minimum_size,
        webp_compressed_size,
        compression_percentage,
        &model_message,
    );
    info!("Program completed.");
}
