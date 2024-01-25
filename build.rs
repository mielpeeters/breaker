use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::{env, fs};

const REVERB_SIZE: usize = 50;

fn main() {
    let out_dir = env::var("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).join("data/reverb.rs");
    if let Some(parent) = dest_path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    let mut f = File::create(&dest_path).unwrap();

    let input_data = include_bytes!("./data/impulse.wav");

    let mut rev = hound::WavReader::new(input_data.as_slice()).unwrap();

    let rev_int: Vec<i32> = rev.samples::<i32>().filter_map(Result::ok).collect();

    let max = rev_int.iter().fold(0, |a: i32, b| a.abs().max(b.abs()));

    let rev_float: Vec<f32> = rev_int.iter().map(|&x| x as f32 / max as f32).collect();

    let rev_float: Vec<_> = rev_float
        .into_iter()
        .skip_while(|x| x.abs() < 0.0001)
        .collect();

    writeln!(f, "pub const REVERB_SIZE: usize = {};", REVERB_SIZE).unwrap();
    writeln!(f, "pub const REVERB_MASK: &[f32; {}] = &[", REVERB_SIZE).unwrap();
    rev_float.iter().take(REVERB_SIZE).for_each(|x| {
        writeln!(f, "{:.15},", x).unwrap();
    });
    writeln!(f, "];").unwrap();
}
