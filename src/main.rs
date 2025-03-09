use std::{fs, path::Path};

use clap::Parser as _;
use cli::Cli;
use epd::inky::Inky;
use image::ImageReader;
use quantize::{crop_resize, error::QuantizeError, fit_resize, image_buffer_into_vec, quantize};

mod cli; // Cli options
mod epd; // Driver for the e-paper display
mod quantize; // Image quantization

const DESATURATED_PALETTE: &[[u8; 4]] = &[
    [0, 0, 0, 255],       // Black
    [255, 255, 255, 255], // White
    [0, 255, 0, 255],     // Green
    [0, 0, 255, 255],     // Blue
    [255, 0, 0, 255],     // Red
    [255, 255, 0, 255],   // Yellow
    [255, 140, 0, 255],   // Orange
    [0, 0, 0, 0],         // Transparent
];

const SATURATED_PALETTE: &[[u8; 4]] = &[
    [0x32, 0x25, 0x36, 0xFF], // Black
    [0xC1, 0xC6, 0xC0, 0xFF], // White
    [0x33, 0x5D, 0x56, 0xFF], // Green
    [0x3F, 0x39, 0x64, 0xFF], // Blue
    [0x9F, 0x51, 0x44, 0xFF], // Red
    [0xB0, 0xA4, 0x4E, 0xFF], // Yellow
    [0xA0, 0x72, 0x4C, 0xFF], // Orange
    [0x00, 0x00, 0x00, 0x00], // Transparent
];

fn lerp(x: u8, y: u8, i: f64) -> u8 {
    (x as f64 * (1.0 - i) + y as f64 * i) as u8
}

fn get_palette(saturation: f64) -> Vec<imagequant::RGBA> {
    DESATURATED_PALETTE
        .iter()
        .zip(SATURATED_PALETTE)
        .map(|(&[rd, gd, bd, ald], &[rs, gs, bs, als])| {
            rgb::Rgba::new(
                lerp(rd, rs, saturation),
                lerp(gd, gs, saturation),
                lerp(bd, bs, saturation),
                lerp(ald, als, saturation),
            )
        })
        .collect()
}

fn palettize_file(
    palette: &[imagequant::RGBA],
    no_crop: bool,
    width: u32,
    height: u32,
    path: &Path,
) -> Result<Vec<u8>, QuantizeError> {
    let original_image = ImageReader::open(path)?.decode()?;
    let image = if no_crop {
        fit_resize(width, height, &original_image)
    } else {
        crop_resize(width, height, &original_image)
    };
    let width = image.width();
    let height = image.height();
    let in_buffer = image_buffer_into_vec(image.into_rgba8());
    let out_buffer = quantize(&palette, width as usize, height as usize, in_buffer.into())?;

    return Ok(out_buffer);
}

fn main() {
    env_logger::init();

    let cli = Cli::parse();
    let palette = get_palette(cli.saturation);

    let mut inky = Inky::new().unwrap();
    let width = inky.eeprom.width as usize;
    let height = inky.eeprom.height as usize;

    let dir: Vec<fs::DirEntry> = fs::read_dir(Path::new(&cli.dir))
        .unwrap()
        .map(|e| e.unwrap())
        .collect();

    let infile = &dir[rand::random_range(0..dir.len())];

    let buffer = palettize_file(
        &palette,
        cli.no_crop,
        width as u32,
        height as u32,
        infile.path().as_path(),
    )
    .unwrap_or_else(quantize::error::handle_error);

    for (ix, px) in buffer.iter().enumerate() {
        inky.set_pixel(ix % width, ix / width, *px);
    }

    inky.show().unwrap();
}
