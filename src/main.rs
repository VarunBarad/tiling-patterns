use clap::{arg, ArgMatches, Command};
use image::{ImageBuffer, Rgba, RgbaImage};
use imageproc::drawing::{draw_filled_rect_mut, draw_polygon_mut};
use imageproc::rect::Rect;
use palette::rgb::Rgb;
use palette::{FromColor, Hsl, Srgb};
use rand::Rng;
use std::collections::VecDeque;
use std::f64::consts::PI;
use std::path::Path;
use std::str::FromStr;
use std::{panic, thread};

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
                .arg(
                    arg!(--"base-color" <VALUE>)
                        .required(true)
                        .default_value("#1b4332"),
                ),
        )
        .subcommand(
            Command::new("hexagon")
                .about("Create a pattern of hexagons")
                .arg(arg!(--output <VALUE>).required(true))
                .arg(arg!(--width <VALUE>).required(true))
                .arg(arg!(--height <VALUE>).required(true))
                .arg(arg!(--size <VALUE>).required(true))
                .arg(
                    arg!(--"base-color" <VALUE>)
                        .required(true)
                        .default_value("#1b4332"),
                ),
        )
        .subcommand(
            Command::new("voronoi-random")
                .about("Create a random voronoi pattern")
                .arg(arg!(--output <VALUE>).required(true))
                .arg(arg!(--width <VALUE>).required(true))
                .arg(arg!(--height <VALUE>).required(true))
                .arg(arg!(--size <VALUE>).required(true))
                .arg(
                    arg!(--"base-color" <VALUE>)
                        .required(true)
                        .default_value("#1b4332"),
                ),
        )
        .get_matches();

    match program_arguments.subcommand() {
        Some(("square", arguments)) => handle_subcommand_square(arguments),
        Some(("hexagon", arguments)) => handle_subcommand_hexagon(arguments),
        Some(("voronoi-random", arguments)) => handle_subcommand_voronoi_random(arguments),
        _ => {
            eprintln!("No known pattern found")
        }
    }
}

#[derive(Clone)]
struct Point {
    x: f64,
    y: f64,
}

#[derive(Clone)]
struct Anchor {
    point: Point,
    color: Rgba<u8>,
}

struct Bounds {
    width: u64,
    height: u64,
}

struct Distance {
    minimum: u32,
    maximum: u32,
}

impl Point {
    fn squared_distance_from(&self, other_point: &Point) -> f64 {
        let horizontal_distance = (self.x - other_point.x).powf(2f64);
        let vertical_distance = (self.y - other_point.y).powf(2f64);

        horizontal_distance + vertical_distance
    }

    fn closest_anchor(
        &self,
        anchors: &Vec<Anchor>,
        minimum_distance_between_anchors: u32,
    ) -> Option<Anchor> {
        let x = ((minimum_distance_between_anchors as f64) / 2f64).powf(2f64);

        let mut closest_anchor: Option<(Anchor, f64)> = None;
        for anchor in anchors {
            let distance = self.squared_distance_from(&anchor.point);
            if distance < x {
                closest_anchor = Some((anchor.clone(), distance));
            } else {
                match closest_anchor {
                    None => {
                        closest_anchor = Some((anchor.clone(), distance));
                    }
                    Some((_, min_distance)) => {
                        if min_distance > distance {
                            closest_anchor = Some((anchor.clone(), distance));
                        }
                    }
                }
            }
        }

        closest_anchor.map(|(anchor, _)| anchor)
    }

    fn random_point_at_certain_distance(&self, distance: &Distance, bounds: &Bounds) -> Point {
        let mut rng = rand::thread_rng();

        let angle = rng.gen::<f64>() * (2f64 * PI);
        let actual_distance = (distance.minimum as f64)
            + (rng.gen::<f64>() * ((distance.maximum - distance.minimum) as f64));

        let point = Point {
            x: (actual_distance * angle.cos()) + self.x,
            y: (actual_distance * angle.sin()) + self.y,
        };

        let is_point_in_horizontal_bounds = (point.x > 0f64) && (point.x < (bounds.width as f64));
        let is_point_in_vertical_bounds = (point.y > 0f64) && (point.y < (bounds.height as f64));

        if is_point_in_horizontal_bounds && is_point_in_vertical_bounds {
            point
        } else {
            self.random_point_at_certain_distance(distance, bounds)
        }
    }

    fn generate_anchor_candidates(&self, distance: &Distance, bounds: &Bounds) -> Vec<Point> {
        let mut candidates = Vec::with_capacity(25);

        for _ in 0..25 {
            candidates.push(self.random_point_at_certain_distance(distance, bounds));
        }

        candidates
    }
}

fn generate_anchor_points(bounds: &Bounds, minimum_distance: u32) -> Vec<Point> {
    let mut rng = rand::thread_rng();

    let squared_minimum_distance = minimum_distance * minimum_distance;

    let mut final_anchors: Vec<Point> = Vec::new();
    let mut anchor_candidates: VecDeque<Point> = VecDeque::new();

    let first_anchor = Point {
        x: rng.gen::<f64>() * (bounds.width as f64),
        y: rng.gen::<f64>() * (bounds.height as f64),
    };

    final_anchors.push(first_anchor.clone());

    let distance = Distance {
        minimum: minimum_distance,
        maximum: minimum_distance * 2,
    };
    anchor_candidates.extend(first_anchor.generate_anchor_candidates(&distance, bounds));

    loop {
        match anchor_candidates.pop_front() {
            None => {
                break;
            }
            Some(candidate) => {
                let mut is_valid_anchor = true;
                for anchor in &final_anchors {
                    if anchor.squared_distance_from(&candidate) < (squared_minimum_distance as f64)
                    {
                        is_valid_anchor = false;
                        break;
                    }
                }

                if is_valid_anchor {
                    final_anchors.push(candidate.clone());

                    match final_anchors.last() {
                        None => {}
                        Some(source) => {
                            anchor_candidates
                                .extend(source.generate_anchor_candidates(&distance, bounds));
                        }
                    }
                }
            }
        }
    }

    final_anchors
}

fn pixel_calculator(
    x: u32,
    image_height: u32,
    anchors: Vec<Anchor>,
    minimum_distance_between_anchors: u32,
) -> Vec<(Point, Rgba<u8>)> {
    let mut pixels: Vec<(Point, Rgba<u8>)> = Vec::with_capacity(image_height as usize);

    let mut filtered_anchors: Vec<Anchor> = Vec::with_capacity(anchors.len());

    for anchor in anchors {
        if (anchor.point.x > (((x as i64) - (minimum_distance_between_anchors as i64)) as f64))
            && (anchor.point.x < (((x as i64) + (minimum_distance_between_anchors as i64)) as f64))
        {
            filtered_anchors.push(anchor);
        }
    }

    for y in 0..image_height {
        let point = Point {
            x: x as f64,
            y: y as f64,
        };
        let closest_anchor =
            point.closest_anchor(&filtered_anchors, minimum_distance_between_anchors);
        match closest_anchor {
            None => {}
            Some(anchor) => {
                pixels.push((point, anchor.color));
            }
        }
    }

    pixels
}

fn generate_voronoi_random_pattern(
    image_width: u32,
    image_height: u32,
    pattern_size: u32,
    base_color: Rgb,
) -> ImageBuffer<Rgba<u8>, Vec<u8>> {
    let minimum_distance = pattern_size / 2;
    let bounds = Bounds {
        width: image_width as u64,
        height: image_height as u64,
    };

    let mut img = RgbaImage::new(image_width, image_height);

    let anchors = generate_anchor_points(&bounds, minimum_distance)
        .into_iter()
        .map(|point| Anchor {
            point: point,
            color: Rgba::from_srgb(generate_color_like(base_color)),
        })
        .collect::<Vec<Anchor>>();

    for step in (0..image_width).step_by(10) {
        let mut thread_pool = Vec::with_capacity(10);
        for x in 0..10 {
            if (x + step) >= image_width {
                break;
            } else {
                let loop_anchors = anchors.clone();
                let handle = thread::spawn(move || {
                    pixel_calculator(x + step, image_height, loop_anchors, minimum_distance)
                });

                thread_pool.push(handle);
            }
        }

        for thread in thread_pool {
            match thread.join() {
                Ok(pixels) => {
                    for (coordinates, color) in pixels {
                        img.put_pixel(coordinates.x as u32, coordinates.y as u32, color);
                    }
                }
                Err(message) => {
                    panic::resume_unwind(message);
                }
            }
        }
    }

    img
}

fn handle_subcommand_voronoi_random(arguments: &ArgMatches) {
    let output_path = Path::new(arguments.get_one("output").unwrap() as &String);

    let image_width = (arguments.get_one("width").unwrap() as &String)
        .parse::<u32>()
        .unwrap();
    let image_height = (arguments.get_one("height").unwrap() as &String)
        .parse::<u32>()
        .unwrap();
    let pattern_size = (arguments.get_one("size").unwrap() as &String)
        .parse::<u32>()
        .unwrap();

    let base_color = Srgb::from_str(arguments.get_one("base-color").unwrap() as &String)
        .unwrap()
        .into_format();

    generate_voronoi_random_pattern(image_width, image_height, pattern_size, base_color)
        .save(output_path)
        .unwrap();
}

fn handle_subcommand_square(arguments: &ArgMatches) {
    let output_path = Path::new(arguments.get_one("output").unwrap() as &String);

    let image_width = (arguments.get_one("width").unwrap() as &String)
        .parse::<u32>()
        .unwrap();
    let image_height = (arguments.get_one("height").unwrap() as &String)
        .parse::<u32>()
        .unwrap();
    let pattern_size = (arguments.get_one("size").unwrap() as &String)
        .parse::<u32>()
        .unwrap();

    let base_color = Srgb::from_str(arguments.get_one("base-color").unwrap() as &String)
        .unwrap()
        .into_format();

    generate_square_pattern(image_width, image_height, pattern_size, base_color)
        .save(output_path)
        .unwrap();
}

fn handle_subcommand_hexagon(arguments: &ArgMatches) {
    let output_path = Path::new(arguments.get_one("output").unwrap() as &String);

    let image_width = (arguments.get_one("width").unwrap() as &String)
        .parse::<u32>()
        .unwrap();
    let image_height = (arguments.get_one("height").unwrap() as &String)
        .parse::<u32>()
        .unwrap();
    let pattern_size = (arguments.get_one("size").unwrap() as &String)
        .parse::<u32>()
        .unwrap();

    let base_color = Srgb::from_str(arguments.get_one("base-color").unwrap() as &String)
        .unwrap()
        .into_format();

    generate_hexagon_pattern(image_width, image_height, pattern_size, base_color)
        .save(output_path)
        .unwrap();
}

fn generate_square_pattern(
    image_width: u32,
    image_height: u32,
    pattern_size: u32,
    base_color: Rgb,
) -> ImageBuffer<Rgba<u8>, Vec<u8>> {
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

    img
}

fn generate_hexagon_anchors(bounds: &Bounds, pattern_size: u32) -> Vec<[Point; 6]> {
    let mut anchors: Vec<[Point; 6]> = Vec::new();

    let half_height = (PI / 3f64).sin() * (pattern_size as f64);

    for i in (-(pattern_size as i64)..(bounds.width as i64)).step_by((3 * pattern_size) as usize) {
        for j in
            -(pattern_size as i64)..((((bounds.height as f64) / half_height).ceil() as i64) + 1)
        {
            let anchor_x: f64 = match j % 2 == 0 {
                true => i as f64,
                false => (i as f64) + ((pattern_size as f64) * 1.5f64),
            };

            let point_1 = Point {
                x: anchor_x,
                y: (j as f64) * half_height,
            };
            let point_2 = Point {
                x: anchor_x + (pattern_size as f64),
                y: (j as f64) * half_height,
            };
            let point_3 = Point {
                x: anchor_x + ((pattern_size as f64) * 1.5f64),
                y: ((j + 1) as f64) * half_height,
            };
            let point_4 = Point {
                x: anchor_x + (pattern_size as f64),
                y: ((j + 2) as f64) * half_height,
            };
            let point_5 = Point {
                x: anchor_x,
                y: ((j + 2) as f64) * half_height,
            };
            let point_6 = Point {
                x: anchor_x - ((pattern_size as f64) * 0.5f64),
                y: ((j + 1) as f64) * half_height,
            };
            anchors.push([point_1, point_2, point_3, point_4, point_5, point_6]);
        }
    }

    anchors
}

fn generate_hexagon_pattern(
    image_width: u32,
    image_height: u32,
    pattern_size: u32,
    base_color: Rgb,
) -> ImageBuffer<Rgba<u8>, Vec<u8>> {
    let bounds = Bounds {
        width: image_width as u64,
        height: image_height as u64,
    };

    let mut img = RgbaImage::new(image_width, image_height);

    generate_hexagon_anchors(&bounds, pattern_size)
        .into_iter()
        .for_each(|points| {
            let color = Rgba::from_srgb(generate_color_like(base_color));
            let coordinates = points.map(|point| imageproc::point::Point {
                x: point.x as i32,
                y: point.y as i32,
            });

            draw_polygon_mut(&mut img, &coordinates, color);
        });

    img
}
