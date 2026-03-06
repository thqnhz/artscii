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
        long,
        hide_default_value = true,
        long_help = "The width of the output art\n\
                     \n\
                     By default the width will be scaled with the height based on the input aspect ratio.\n\
                     Might squish the output art if both width and height are specified."
    )]
    width: Option<u32>,

    /// Output art height
    #[arg(
        long,
        hide_default_value = true,
        long_help = "The height of the output art\n\
                     \n\
                     By default the height will be scaled with the width based on the input aspect ratio.\n\
                     Might squish the output art if both width and height are specified."
    )]
    height: Option<u32>,

    /// Dimension
    #[arg(
        long,
        conflicts_with_all = ["width", "height"],
        long_help = "The dimension of the output art\n\
                     \n\
                     Format: WxH
                     \n\
                     Mutually exclusive with width/height flag."
    )]
    dimension: Option<String>,

    /// Output file
    #[arg(
        short,
        long,
    )]
    output: Option<std::path::PathBuf>,
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

fn get_width_by_height(height: u32, aspect_ratio: f32) -> u32 {
    (height as f32 * aspect_ratio) as u32
}

fn get_height_by_width(width: u32, aspect_ratio: f32) -> u32 {
    (width as f32 / aspect_ratio) as u32
}

fn get_output_dimension(args: &Args, img_width: u32, img_height: u32) -> Option<(u32, u32)> {
    let font_h_to_w_ratio = 2.5_f32;
    let img_aspect_ratio = img_width as f32 * font_h_to_w_ratio / img_height as f32;

    if let Some(dimension) = &args.dimension {
        let (width, height) = dimension.split_once('x')?;
        return Some((width.parse().ok()?, height.parse().ok()?));
    } else if let Some(width) = args.width {
        if let Some(height) = args.height {
            return Some((width, height));
        }
        return Some((width, get_height_by_width(width, img_aspect_ratio)));
    } else if let Some(height) = args.height {
        return Some((get_width_by_height(height, img_aspect_ratio), height));
    }
    get_best_terminal_output_dimension(img_aspect_ratio)
}

fn get_best_terminal_output_dimension(img_aspect_ratio: f32) -> Option<(u32, u32)> {
    let term_size = terminal_size();
    let (term_w, term_h): (u32, u32);
    if let Some((Width(w), Height(h))) = term_size {
        term_w = w as u32;
        term_h = h as u32;
    } else {
        term_w = 25_u32;
        term_h = 0_u32;
    }
    let mut width = term_w;
    let mut height = get_height_by_width(width, img_aspect_ratio);
    if height > term_h {
        height = term_h;
        width = get_width_by_height(height, img_aspect_ratio);
    }
    Some((width, height))
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

fn process(rgb: image::RgbImage, out_w: u32, out_h: u32, color_mode: ColorMode, charset: &str) -> String {
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
    output
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    let color_mode = match args.color {
        None => ColorMode::None,
        Some(ColorMode::Auto) => detect_color_support(),
        Some(ref mode) => mode.clone(),
    };

    // The glyph sets are from https://inkmeascii.com/blog/best-ascii-characters/
    let charset = match args.detailed {
        0 => ".:-=+*#%@",
        1 => "_.,-=+:;cba!?0123456789$W#@",
        _ => ".'`^\",:;Il!i><~+_-?][}{1)(|\\/tfjrxnuvczXYUJCLQ0OZmwqpdbkhao*#MW&8%B@$",
    };

    let img = image::ImageReader::open(&args.image)?.decode()?;
    let rgb = img.to_rgb8();

    if let Some((out_w, out_h)) = get_output_dimension(&args, rgb.width(), rgb.height()) {
        let output = process(rgb, out_w, out_h, color_mode, charset);
        if let Some(out_file) = &args.output {
            std::fs::write(out_file, output)?;
            return Ok(());
        }
        println!("{}", output);
    }
    Ok(())
}

