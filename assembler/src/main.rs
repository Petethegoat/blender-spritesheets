use image::RgbaImage;
use std::{cmp::max, path::PathBuf};

mod errors;
use errors::{ImageFormatError, InconsistentSizeError, NoImagesError};

#[derive(Debug, Copy, Clone)]
struct Dims {
    x: usize,
    y: usize,
}

type BoxResult<T> = Result<T, Box<dyn std::error::Error>>;

fn main() -> BoxResult<()> {
    let matches = clap::App::new("assembler")
        .about("Combined PNGs into a spritesheet")
        .arg(
            clap::Arg::with_name("root")
                .short("r")
                .long("root")
                .value_name("DIR")
                .help("Where to search for spritesheet tiles")
                .takes_value(true)
                .required(true),
        )
        .arg(
            clap::Arg::with_name("output")
                .short("o")
                .long("out")
                .value_name("PNG_FILENAME")
                .help("Spritesheet output filename")
                .takes_value(true),
        )
        .get_matches();

    let root = matches.value_of("root").unwrap();
    let images = collect_images(root);
    let dims = dims(&images)?;
    let tiles = optimal_stacking(images.len(), dims);
    let width = (tiles.x * dims.x) as u32;
    let height = (tiles.y * dims.y) as u32;

    let mut out: RgbaImage = image::ImageBuffer::new(width, height);
    for (i, img) in images.iter().enumerate() {
        let x = (i % tiles.x) * dims.x;
        let y = (i / tiles.x) * dims.y;
        image::imageops::replace(&mut out, img, x as u32, y as u32);
    }

    let output = matches.value_of("output").unwrap_or("out.png");
    let out_path: PathBuf = [root, output].iter().collect();
    out.save(out_path)?;

    Ok(())
}

fn dims(images: &[RgbaImage]) -> BoxResult<Dims> {
    let mut iter = images.iter();
    let first = iter.next().ok_or_else(|| NoImagesError)?;
    let dims = first.dimensions();
    if images.iter().all(|next| next.dimensions() == dims) {
        Ok(Dims {
            x: dims.0 as usize,
            y: dims.1 as usize,
        })
    } else {
        Err(InconsistentSizeError.into())
    }
}

fn optimal_stacking(count: usize, dims: Dims) -> Dims {
    struct Min {
        dim: usize,
        x: usize,
    }
    let Min { x, .. } = (1..=count).fold(
        Min {
            dim: std::usize::MAX,
            x: 0,
        },
        |min, x| {
            let y = y_from_x(x, count);
            let dim = max(y * dims.y, x * dims.x);
            if dim < min.dim {
                Min { x, dim }
            } else {
                min
            }
        },
    );
    Dims {
        x: count,
        y: 1,
    }
}

fn y_from_x(x: usize, count: usize) -> usize {
    (count as f32 / x as f32).ceil() as usize
}

fn collect_images(root: &str) -> Vec<RgbaImage> {
    let temporary: PathBuf = [root, "temp"].iter().collect();
    walkdir::WalkDir::new(temporary)
        .sort_by(|a, b| a.file_name().cmp(b.file_name()))
        .into_iter()
        .filter_map(|e| match image_filter(e) {
            Ok(img) => Some(img),
            Err(_) => None,
        })
        .collect::<Vec<_>>()
}

fn image_filter(entry: Result<walkdir::DirEntry, walkdir::Error>) -> BoxResult<RgbaImage> {
    match image::open(entry?.path())? {
        image::ImageRgba8(img) => Ok(img),
        _ => Err(ImageFormatError.into()),
    }
}
