use std::collections::HashMap;
use std::fs;
use std::io::{self};
use image::{DynamicImage, GenericImageView};
use log::{info, error};
use prettytable::{Table, Row, Cell};

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

/// Compresses entropy results using Prediction by Partial Matching (PPM).
fn ppm_compress(image_data: &[u8]) -> usize {
    let mut context_map: HashMap<Vec<u8>, HashMap<u8, usize>> = HashMap::new();
    let mut compressed_size = 0;

    for (i, &value) in image_data.iter().enumerate() {
        let context = image_data[i.saturating_sub(3)..i].to_vec(); // Use last 3 bytes as context
        let context_freq = context_map.entry(context).or_insert_with(HashMap::new);
        *context_freq.entry(value).or_insert(0) += 1;

        let total_freq: usize = context_freq.values().sum();
        let prob = context_freq[&value] as f64 / total_freq as f64;

        compressed_size += prob.log2().abs().ceil() as usize; // Calculate compressed size
    }

    compressed_size
}

/// Displays results in a formatted table for better readability.
fn display_results(
    red_entropy: f64,
    green_entropy: f64,
    blue_entropy: f64,
    total_entropy: f64,
    original_size: usize,
    theoretical_size: f64,
    red_compressed_size: usize,
    green_compressed_size: usize,
    blue_compressed_size: usize,
    compression_percentage: f64,
    model_message: &str,
) {
    let mut table = Table::new();
    table.add_row(Row::new(vec![Cell::new("Color Channel"), Cell::new("Entropy (bits/pixel)")])); // Header
    table.add_row(Row::new(vec![Cell::new("Red"), Cell::new(&format!("{:.2}", red_entropy))]));
    table.add_row(Row::new(vec![Cell::new("Green"), Cell::new(&format!("{:.2}", green_entropy))]));
    table.add_row(Row::new(vec![Cell::new("Blue"), Cell::new(&format!("{:.2}", blue_entropy))]));
    table.add_row(Row::new(vec![Cell::new("Total"), Cell::new(&format!("{:.2}", total_entropy))]));
    table.add_row(Row::new(vec![Cell::new("Compressed Size (bytes)"), Cell::new(&format!(
        "Red: {}, Green: {}, Blue: {}",
        red_compressed_size, green_compressed_size, blue_compressed_size
    ))]));
    table.printstd();

    println!("Original Size: {} bytes", original_size);
    println!("{}", model_message); // Display model message instead of invalid theoretical size
    println!(
        "Compression Percentage (Based on Entropy Limit): {:.2}%",
        compression_percentage
    );

    let compressed_total_size = red_compressed_size + green_compressed_size + blue_compressed_size;
    println!("Total Compressed Size (bytes): {}", compressed_total_size);

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

    // Compress entropy results using PPM
    let red_compressed_size = ppm_compress(&red_channel);
    let green_compressed_size = ppm_compress(&green_channel);
    let blue_compressed_size = ppm_compress(&blue_channel);

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
        red_compressed_size,
        green_compressed_size,
        blue_compressed_size,
        compression_percentage,
        &model_message,
    );
    info!("Program completed.");
}
