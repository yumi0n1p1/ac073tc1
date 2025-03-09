use std::{fmt::Display, io, process};

#[derive(derive_more::From)]
pub enum QuantizeError {
    Io(io::Error),
    Image(image::ImageError),
    Quantize(imagequant::Error),
}

impl Display for QuantizeError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            QuantizeError::Io(error) => write!(f, "File error: {error}"),
            QuantizeError::Image(error) => write!(f, "File error: {error}"),
            QuantizeError::Quantize(error) => write!(f, "Quantization error: {error}"),
        }
    }
}

pub fn handle_error<T, E>(error: E) -> T
where
    E: Into<QuantizeError>,
{
    println!("{}", error.into());
    process::exit(1);
}
