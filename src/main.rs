use std::path::Path;
use std::str::FromStr;
use clap::{arg, Command};
use image::{Rgba, RgbaImage};
use imageproc::drawing::draw_filled_rect_mut;
use imageproc::rect::Rect;
use palette::rgb::Rgb;
use palette::{FromColor, Hsl, Srgb};
use rand::Rng;

fn generate_color_like(base: Srgb<f32>) -> Srgb<f32> {
    let hsl: Hsl = Hsl::from_color(base);
    let mut rng = rand::thread_rng();

    let min_lightness = hsl.lightness * 3.0 / 4.0;
    let max_lightness = hsl.lightness * 4.0 / 3.0;

    let new_lightness = rng.gen_range(min_lightness..max_lightness);
    let new_hsl = Hsl::new(hsl.hue, hsl.saturation, new_lightness);

    Rgb::from_color(new_hsl)
}

pub trait RgbaExtensions {
    fn from_srgb(input: Srgb<f32>) -> Rgba<u8>;
}

impl RgbaExtensions for Rgba<u8> {
    fn from_srgb(input: Srgb<f32>) -> Rgba<u8> {
        Rgba::from([
            (input.red * 255.0) as u8,
            (input.green * 255.0) as u8,
            (input.blue * 255.0) as u8,
            255,
        ])
    }
}

fn main() {
    let program_arguments = Command::new("tiling-patterns")
        .version("0.1.0")
        .author("Varun Barad <varun@varunbarad.com>")
        .about("CLI tool to generate images of tiling patterns")
        .args_override_self(true)
        .subcommand_required(true)
        .subcommand(
            Command::new("square")
                .about("Create a pattern of squares")
                .arg(arg!(--output <VALUE>).required(true))
                .arg(arg!(--width <VALUE>).required(true))
                .arg(arg!(--height <VALUE>).required(true))
                .arg(arg!(--size <VALUE>).required(true))
                .arg(arg!(--"base-color" <VALUE>).required(true).default_value("#1b4332")),
        )
        .get_matches();

    match program_arguments.subcommand() {
        Some(("square", arguments)) => {
            let output_path = Path::new(arguments.get_one("output").unwrap() as &String);

            let image_width = u32::from_str_radix(arguments.get_one("width").unwrap() as &String, 10u32).unwrap();
            let image_height = u32::from_str_radix(arguments.get_one("height").unwrap() as &String, 10u32).unwrap();
            let pattern_size = u32::from_str_radix(arguments.get_one("size").unwrap() as &String, 10u32).unwrap();

            let base_color = Srgb::from_str(arguments.get_one("base-color").unwrap() as &String).unwrap().into_format();

            let mut img = RgbaImage::new(image_width, image_height);

            for i in 0..(f32::ceil((image_width as f32) / (pattern_size as f32)) as u32) {
                for j in 0..(f32::ceil((image_height as f32) / (pattern_size as f32)) as u32) {
                    let tile_color = Rgba::from_srgb(generate_color_like(base_color));
                    let x: i32 = (i * pattern_size) as i32;
                    let y: i32 = (j * pattern_size) as i32;
                    let pattern = Rect::at(x, y).of_size(pattern_size, pattern_size);
                    draw_filled_rect_mut(&mut img, pattern, tile_color);
                }
            }

            img.save(output_path).unwrap();
        }
        _ => {
            eprintln!("No known pattern found")
        }
    }
}
