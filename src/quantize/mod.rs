use image::{imageops, DynamicImage, ImageBuffer, RgbaImage};
use std::cmp::Ordering;

pub mod error;

pub fn fit_resize(width: u32, height: u32, image: &DynamicImage) -> DynamicImage {
    let image_width = image.width() as f64;
    let image_height = image.height() as f64;
    let image_aspect_ratio = image_width / image_height;
    let target_aspect_ratio = width as f64 / height as f64;
    let resized = image.resize(width, height, imageops::FilterType::Lanczos3);

    let (overlay_x, overlay_y) = match image_aspect_ratio.total_cmp(&target_aspect_ratio) {
        Ordering::Less => ((width - resized.width()) / 2, 0),
        Ordering::Equal => (0, 0),
        Ordering::Greater => (0, (height - resized.height()) / 2),
    };

    let mut new_image = RgbaImage::new(width, height);
    imageops::overlay(&mut new_image, &resized, overlay_x as i64, overlay_y as i64);

    return new_image.into();
}

/** Resize a [DynamicImage] into the given width and height without distortion. */
pub fn crop_resize(width: u32, height: u32, image: &DynamicImage) -> DynamicImage {
    let image_width = image.width() as f64;
    let image_height = image.height() as f64;
    let image_aspect_ratio = image_width / image_height;
    let target_aspect_ratio = width as f64 / height as f64;

    let (crop_width, crop_height) = match image_aspect_ratio.total_cmp(&target_aspect_ratio) {
        Ordering::Less => (image.width(), (image_width / target_aspect_ratio) as u32),
        Ordering::Equal => (image.width(), image.height()),
        Ordering::Greater => ((image_height * target_aspect_ratio) as u32, image.height()),
    };

    let crop_x = (image.width() - crop_width) / 2;
    let crop_y = (image.height() - crop_height) / 2;

    return image
        .crop_imm(crop_x, crop_y, crop_width, crop_height)
        .resize_exact(width, height, imageops::FilterType::Lanczos3);
}

/** Convert an RGBA [ImageBuffer] into a vector of [imagequant::RGBA] pixels. */
pub fn image_buffer_into_vec(
    image: ImageBuffer<image::Rgba<u8>, Vec<u8>>,
) -> Vec<imagequant::RGBA> {
    bytemuck::allocation::cast_vec(image.into_raw())
}

/** Quantize an image (as a boxed slice of pixels) according to a palette of max. 256 colors. */
pub fn quantize(
    palette: &[imagequant::RGBA],
    width: usize,
    height: usize,
    buffer: Box<[imagequant::RGBA]>,
) -> Result<Vec<u8>, imagequant::Error> {
    // Initialize the quantizer
    let mut quantizer = imagequant::new();
    quantizer.set_max_colors(palette.len() as u32)?;
    quantizer.set_speed(1)?;

    // Force the quantizer to only use palette colors
    let mut image = quantizer.new_image(buffer, width, height, 0.0)?;
    for color in palette {
        image.add_fixed_color(color.clone())?;
    }

    // Quantize
    let mut quantization = quantizer.quantize(&mut image)?;
    let (out_palette, mut outbuf) = quantization.remapped(&mut image)?;

    // The order of the palette is not necessarily preserved,
    // so we remap the output palette from the quantizer to the input palette
    let palette_remap: Vec<u8> = out_palette
        .iter()
        .map(|x| palette.iter().position(|y| x == y).unwrap() as u8)
        .collect();
    for x in outbuf.iter_mut() {
        *x = palette_remap[*x as usize];
    }

    return Ok(outbuf);
}
