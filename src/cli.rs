use clap::Parser;

#[derive(Parser)]
#[command(version, author, about)]
pub struct Cli {
    /// Directory from which to randomly choose a file to display
    pub dir: String,
    #[arg(long, default_value_t = 0.5)]
    pub saturation: f64,
    #[arg(long)]
    pub no_crop: bool,
}
