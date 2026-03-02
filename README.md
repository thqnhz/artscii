# artscii

Image to ascii art

Transform your image to ascii art, customizable width & height, detail level, color output.

## How to use

Download the binary from the `Release` tab or [Build from source](#build-from-source)

```sh
./path/to/artscii --help
```

## Features

- Multiple charset: 3 set of ascii characters to increase output details.
- Support colored output: 256 color and true color support.
- Auto detect color support: Detect terminal color support with `support-color` crate.
- Customizable output width and height:

|Specified values | Width                | Height             |
|-----------------|----------------------|--------------------|
| Neither         | Scaled with height   | Terminal height    |
| Only width      | You alredy specified | Scaled with width  |
| Only height     | Scaled with height   | You chose it       |
| Both            | This is gonna be so  | much fun           |

> Note: If you specify weird width and height, the output might be cursed.

## Build from source

> Make sure you have `rustup` installed.

1. Clone the repo

```sh
git clone https://github.com/thqnhz/artscii.git
cd artscii
```

2. Build the project

```sh
cargo build --release
```

3. Be patient

4. Run the executable

```sh
./target/release/artscii --help
```

