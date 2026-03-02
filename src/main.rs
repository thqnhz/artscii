use clap::{ArgAction, Parser, ValueEnum};
use supports_color::{on, Stream};
use terminal_size::{Width, Height, terminal_size};

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

    /// Output art width
    #[arg(
        short,
        long,
        default_value_t = 0,
        hide_default_value = true,
        long_help = "The width of the output art\n\
                     \n\
                     By default the width will be scaled with the height based on the input aspect ratio.\n\
                     Might squish the output art if both width and height are specified."
    )]
    width: u32,

    /// Output art height
    #[arg(
        short,
        long,
        default_value_t = 0,
        hide_default_value = true,
        long_help = "The height of the output art\n\
                     \n\
                     By default the height will be scaled with the width based on the input aspect ratio.\n\
                     Might squish the output art if both width and height are specified."
    )]
    height: u32,
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

fn ansi256(r: u8, g: u8, b: u8) -> String {
    let r6 = (r as u16 * 5 / 255) as u16;
    let g6 = (g as u16 * 5 / 255) as u16;
    let b6 = (b as u16 * 5 / 255) as u16;
    let code = 16 + 36 * r6 + 6 * g6 + b6;
    format!("\x1b[38;5;{}m", code)
}

fn ansi_truecolor(r: u8, g: u8, b: u8) -> String {
    format!("\x1b[38;2;{};{};{}m", r, g, b)
}

fn choose_ansi(r: u8, g: u8, b: u8, color_mode: &ColorMode) -> String {
    match color_mode {
        ColorMode::Full => ansi_truecolor(r, g, b),
        ColorMode::Partial => ansi256(r, g, b),
        _ => "".to_string()
    }
}

fn process(rgb: image::RgbImage, out_w: u32, out_h: u32, color_mode: ColorMode, charset: &str) {
    let (w, h) = rgb.dimensions();
    let block_w = w / out_w;
    let block_h = h / out_h;
    let area = (block_w * block_h) as u32;

    let byte_per_char = match color_mode {
        ColorMode::Full => 32,
        ColorMode::Partial => 16,
        _ => 1,
    };
    let mut output = String::with_capacity((out_w * out_h * out_h * byte_per_char) as usize);

    let mut last_ansi = String::with_capacity(byte_per_char as usize);
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

            let avg_r = (sum_r / area) as u8;
            let avg_g = (sum_g / area) as u8;
            let avg_b = (sum_b / area) as u8;
            let glyph = choose_glyph(avg_r, avg_g, avg_b, charset);
            let ansi = choose_ansi(avg_r, avg_g, avg_b, &color_mode);
            if ansi != last_ansi {
                output.push_str(&ansi);
                last_ansi = ansi;
            }
            output.push(glyph);
        }
        output.push('\n');
    }
    let reset = "\x1b[0m";
    output.push_str(reset);

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
    let rgb = img.to_rgb8();
    
    let mut out_w = args.width;
    let mut out_h = args.height;
    let font_h_to_w_ratio = 2.5_f32;
    let resolution = rgb.width() as f32 * font_h_to_w_ratio / rgb.height() as f32;
    if out_w == 0 && out_h == 0 {
        let size = terminal_size();
        let (term_w, term_h): (u32, u32);
        if let Some((Width(w), Height(h))) = size {
            term_w = w as u32;
            term_h = h as u32;
        } else {
            term_h = 25_u32;
            term_w = 0_u32;
        }
        out_h = term_h;
        out_w = (out_h as f32 * resolution) as u32;
        if out_w > term_w {
            out_w = term_w;
            out_h = (out_w as f32 / resolution) as u32;
        }
    } else if out_w == 0 {
        out_w = (out_h as f32 * resolution) as u32;
    } else if out_h == 0 {
        out_h = (out_w as f32 / resolution) as u32;
    } else if out_w > rgb.width() || out_h > rgb.height() {
        out_w = rgb.width();
        out_h = rgb.height();
    }

    process(rgb, out_w, out_h, color_mode, charset);
    Ok(())
}

