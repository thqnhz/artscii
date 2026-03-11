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

    /// Output width
    #[arg(
        long,
        long_help = "Output width\n\
                     \n\
                     If not specified, output width will be calculated based on image aspect ratio."
    )]
    width: Option<u32>,

    /// Output height
    #[arg(
        long,
        long_help = "Output height\n\
                     \n\
                     If not specified, output height will be calculated based on image aspect ratio."
    )]
    height: Option<u32>,

    /// Output dimension
    #[arg(
        long,
        conflicts_with_all = ["width", "height"],
        value_name = "WxH",
        long_help = "Output dimension\n\
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

#[derive(ValueEnum, Clone, Debug, Default)]
enum ColorMode {
    None,
    Full,
    Partial,
    #[default] Auto,
}

#[derive(Default)]
struct Artscii {
    color_mode: ColorMode,
    charset: String,
    image: image::RgbImage,
    output_width: u32,
    output_height: u32,
    output: String,
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

fn split_dimension_arg(dimension: Option<&str>) -> Result<(u32, u32), ()> {
    let dimension = dimension.ok_or(())?;
    let (w, h) = dimension.split_once('x').ok_or(())?;
    Ok((
        w.parse().map_err(|_| ())?,
        h.parse().map_err(|_| ())?
    ))
}

fn get_output_dimension(artscii: &Artscii, arg_width: Option<u32>, arg_height: Option<u32>) -> Option<(u32, u32)> {
    let font_h_to_w_ratio = 2.5_f32;
    let img_aspect_ratio = artscii.image.width() as f32 * font_h_to_w_ratio / artscii.image.height() as f32;

    if let Some(width) = arg_width {
        if let Some(height) = arg_height {
            return Some((width, height));
        }
        return Some((width, get_height_by_width(width, img_aspect_ratio)));
    } else if let Some(height) = arg_height {
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

fn process(artscii: &mut Artscii) {
    let (w, h) = artscii.image.dimensions();
    let block_w = w / artscii.output_width;
    let block_h = h / artscii.output_height;
    let area = (block_w * block_h) as u32;

    let byte_per_char = match artscii.color_mode {
        ColorMode::Full => 32,
        ColorMode::Partial => 16,
        _ => 1,
    };
    artscii.output = String::with_capacity((artscii.output_width * artscii.output_height * artscii.output_height * byte_per_char) as usize);

    let mut last_ansi = String::with_capacity(byte_per_char as usize);
    for block_y in 0..artscii.output_height {
        for block_x in 0..artscii.output_width {
            let mut sum_r = 0_u32;
            let mut sum_g = 0_u32;
            let mut sum_b = 0_u32;

            for per_y in 0..block_h {
                for per_x in 0..block_w {
                    let x = block_x * block_w + per_x;
                    let y = block_y * block_h + per_y;

                    let pixel = artscii.image.get_pixel(x, y);
                    let [r, g, b] = pixel.0;
                    sum_r += r as u32;
                    sum_g += g as u32;
                    sum_b += b as u32;
                }
            }

            let avg_r = (sum_r / area) as u8;
            let avg_g = (sum_g / area) as u8;
            let avg_b = (sum_b / area) as u8;
            let glyph = choose_glyph(avg_r, avg_g, avg_b, &artscii.charset);
            let ansi = choose_ansi(avg_r, avg_g, avg_b, &artscii.color_mode);
            if ansi != last_ansi {
                artscii.output.push_str(&ansi);
                last_ansi = ansi;
            }
            artscii.output.push(glyph);
        }
        artscii.output.push('\n');
    }
    let reset = "\x1b[0m";
    artscii.output.push_str(reset);
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    let mut artscii = Artscii::default();

    artscii.color_mode = match args.color {
        None => ColorMode::None,
        Some(ColorMode::Auto) => detect_color_support(),
        Some(mode) => mode,
    };

    // The glyph sets are from https://inkmeascii.com/blog/best-ascii-characters/
    artscii.charset = match args.detailed {
        0 => ".:-=+*#%@".to_string(),
        1 => "_.,-=+:;cba!?0123456789$W#@".to_string(),
        _ => ".'`^\",:;Il!i><~+_-?][}{1)(|\\/tfjrxnuvczXYUJCLQ0OZmwqpdbkhao*#MW&8%B@$".to_string(),
    };

    artscii.image = image::ImageReader::open(&args.image)?.decode()?.to_rgb8();

    if let Ok((out_w, out_h)) = split_dimension_arg(args.dimension.as_deref()) {
        artscii.output_width = out_w;
        artscii.output_height = out_h;
    } else if let Some((out_w, out_h)) = get_output_dimension(&artscii, args.width, args.height) {
        if out_w > artscii.image.width() || out_h > artscii.image.height() {
            return Err("Height/Width value is bigger than the image resolution.".into());
        }
        artscii.output_width = out_w;
        artscii.output_height = out_h;
    }
    process(&mut artscii);
    if let Some(out_file) = &args.output {
        std::fs::write(out_file, artscii.output)?;
        return Ok(());
    }
    println!("{}", artscii.output);
    Ok(())
}

