use std::collections::HashMap;
use std::io::{self};
use image::{DynamicImage, GenericImageView};

fn read_image(image_path: &str) -> DynamicImage {
    image::open(image_path).unwrap_or_else(|_| {
        panic!("Error: Unable to open the image file '{}'. Please ensure it exists and is a valid JPG or PNG.", image_path);
    })
}

fn calculate_entropy(image_data: &[u8]) -> f64 {
    let mut histogram = HashMap::new();
    let total_pixels = image_data.len() as f64;

    for &value in image_data {
        *histogram.entry(value).or_insert(0) += 1;
    }

    histogram.iter()
        .map(|(_, &count)| {
            let probability = count as f64 / total_pixels;
            -probability * probability.log2()
        })
        .sum()
}

fn simulate_compression(image_data: &[u8]) -> usize {
    let mut compressed_data = Vec::new();
    let mut current_byte = image_data[0];
    let mut count = 0;

    for &byte in image_data {
        if byte == current_byte {
            count += 1;
        } else {
            compressed_data.push(current_byte);
            compressed_data.push(count);
            current_byte = byte;
            count = 1;
        }
    }

    compressed_data.push(current_byte);
    compressed_data.push(count);
    compressed_data.len()
}

fn split_rgb_channels(img: &DynamicImage) -> (Vec<u8>, Vec<u8>, Vec<u8>) {
    let (width, height) = img.dimensions();
    let mut red_channel = Vec::with_capacity((width * height) as usize);
    let mut green_channel = Vec::with_capacity((width * height) as usize);
    let mut blue_channel = Vec::with_capacity((width * height) as usize);

    for pixel in img.pixels() {
        let [r, g, b, _] = pixel.2.0; // Access inner array using `.0`
        red_channel.push(r);
        green_channel.push(g);
        blue_channel.push(b);
    }

    (red_channel, green_channel, blue_channel)
}

fn main() {
    // Prompt user for the image file path
    println!("Enter the path to the image file:");

    let mut path = String::new();
    io::stdin().read_line(&mut path).expect("Failed to read input.");
    let path = path.trim(); // Trim newline or extra spaces

    let img = read_image(path);

    // Separate data into color channels
    let (red_channel, green_channel, blue_channel) = split_rgb_channels(&img);

    // Calculate entropy for each channel
    let red_entropy = calculate_entropy(&red_channel);
    let green_entropy = calculate_entropy(&green_channel);
    let blue_entropy = calculate_entropy(&blue_channel);

    // Ensure total entropy calculation is valid
    let total_entropy = red_entropy + green_entropy + blue_entropy;

    // Present results in a cleaner format
    println!("-------------------------");
    println!("Image Analysis Results:");
    println!("-------------------------");
    println!("Entropy (Red): {:.2} bits/pixel", red_entropy);
    println!("Entropy (Green): {:.2} bits/pixel", green_entropy);
    println!("Entropy (Blue): {:.2} bits/pixel", blue_entropy);

    if total_entropy.is_finite() {
        println!("-------------------------");
        println!("Total Entropy: {:.2} bits/pixel", total_entropy);
    } else {
        println!("-------------------------");
        println!("Error: Entropy calculation resulted in an invalid value.");
    }

    // Simulate compression for the entire image
    let compressed_size = simulate_compression(&img.to_rgb8());
    println!("Simulated Compressed Size: {} bytes", compressed_size);
    println!("-------------------------");
}
