use clap::{ArgAction, Parser, ValueEnum};
use supports_color::{on, Stream};

#[derive(Parser, Debug)]
#[command(author, version, about)]
struct Args {
    /// Path to the image
    image: std::path::PathBuf,

    /// Use detailed ASCII charset. Can be stacked twice.
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

fn process(img: image::DynamicImage, mode: ColorMode, charset: &str) {
    let rgb = img.to_rgb8();
    let (w, h) = rgb.dimensions();

    let font_h_to_w_ratio = 2.5_f32;
    let out_h = 25;
    let out_w = (w as f32 * out_h as f32 * font_h_to_w_ratio / h as f32) as u32;
    let block_w = w / out_w;
    let block_h = h / out_h;
    let area = (block_w * block_h) as u32;

    let mut output = String::with_capacity((out_w * out_h) as usize);
    for block_y in 0..out_h {
        for block_x in 0..out_w {
            let mut sum_r = 0_u32;
            let mut sum_g = 0_u32;
            let mut sum_b = 0_u32;

            for per_y in 0..block_h {
                for per_x in 0..block_w {
                    let x = block_x * block_w + per_x;
                    let y = block_y * block_h + per_y;

                    let pixel = rgb.get_pixel(x, y);
                    let [r, g, b] = pixel.0;
                    sum_r += r as u32;
                    sum_g += g as u32;
                    sum_b += b as u32;
                }
            }

            let avg_r = sum_r / area;
            let avg_g = sum_g / area;
            let avg_b = sum_b / area;
            let glyph = choose_glyph(avg_r as u8, avg_g as u8, avg_b as u8, charset);
            output.push(glyph);
        }
        output.push('\n');
    }
    print!("{}", output);
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
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

    let img = image::ImageReader::open(&args.image)?.decode()?;
    process(img, color_mode, charset);
    Ok(())
}

