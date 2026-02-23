use clap::{ArgAction, Parser, ValueEnum};
use supports_color::{on, Stream};

#[derive(Parser, Debug)]
#[command(author, version, about)]
struct Args {
    /// Use detailed ASCII charset. Can be stacked twice
    #[arg(
        short,
        long,
        action = ArgAction::Count,
    )]
    detailed: u8,

    /// Color mode [auto, full, partial, none]
    #[arg(
        short,
        long,
        value_enum,
        num_args = 0..=1,
        default_missing_value = "auto",
        hide_possible_values = true,
        long_help = "Color mode\n\
                     \n\
                     none       - No color\n\
                     full       - 24-bit truecolor\n\
                     partial    - ANSI 256 color\n\
                     auto       - Detect automatically\n\
                     \n\
                     (default to auto when flag is passed without specifying value)."
    )]
    color: Option<ColorMode>,
}

#[derive(ValueEnum, Clone, Debug)]
enum ColorMode {
    None,
    Full,
    Partial,
    Auto,
}

fn detect_color_support() -> ColorMode {
    match on(Stream::Stdout) {
        Some(level) if level.has_16m => ColorMode::Full,
        Some(_) => ColorMode::Partial,
        None => ColorMode::None,
    }
}

fn luminance(r: u8, g: u8, b: u8) -> f32 {
    // Luminance fomula is found at https://en.wikipedia.org/wiki/Relative_luminance
    0.2126_f32 * f32::from(r) + 0.7152_f32 * f32::from(g) + 0.0722_f32 * f32::from(b)
}

fn choose_glyph(r: u8, g: u8, b: u8, charset: &str) -> char {
    let lum = luminance(r, g, b) / 255_f32;
    let index = (lum * (charset.len() - 1) as f32).round() as usize;
    charset.chars().nth(index).unwrap()
}

fn main() {
    let args = Args::parse();

    let color_mode = match args.color {
        None => ColorMode::None,
        Some(ColorMode::Auto) => detect_color_support(),
        Some(mode) => mode,
    };

    // The glyph sets are from https://inkmeascii.com/blog/best-ascii-characters/
    let charset = match args.detailed {
        0 => " .:-=+*#%@",
        1 => " _.,-=+:;cba!?0123456789$W#@",
        _ => " .'`^\",:;Il!i><~+_-?][}{1)(|\\/tfjrxnuvczXYUJCLQ0OZmwqpdbkhao*#MW&8%B@$",
    };

    println!("Color mode: {:?}", color_mode);
    println!("Charset used: {}", charset);
    println!("Glyph used: {}", choose_glyph(132, 215, 50, charset));
}

