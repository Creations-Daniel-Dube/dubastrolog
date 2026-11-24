use std::env;
use std::fs;
use std::path::PathBuf;

fn main() {
    println!("cargo:rerun-if-changed=images/dubastrolog.png");
    if env::var("CARGO_CFG_TARGET_OS").ok().as_deref() != Some("windows") {
        return;
    }

    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
    let ico_path = out_dir.join("dubastrolog.ico");
    let png = fs::read("images/dubastrolog.png").expect("images/dubastrolog.png");
    fs::write(&ico_path, png_to_ico(&png)).expect("écriture de l'icône Windows");

    let mut res = winresource::WindowsResource::new();
    let icon = ico_path.to_string_lossy().replace('\\', "/");
    res.set_icon(&icon)
        .compile()
        .expect("intégration de l'icône dans le .exe");
}

fn png_to_ico(png: &[u8]) -> Vec<u8> {
    let source = image::load_from_memory(png).expect("décodage de images/dubastrolog.png");
    let sizes = [16u32, 32, 48, 64, 128, 256];
    let images: Vec<(u32, Vec<u8>)> = sizes
        .into_iter()
        .map(|size| {
            let rgba = source
                .resize_exact(size, size, image::imageops::FilterType::Lanczos3)
                .into_rgba8();
            (size, dib_image(rgba.as_raw(), size, size))
        })
        .collect();

    let mut ico = Vec::new();
    ico.extend_from_slice(&0u16.to_le_bytes());
    ico.extend_from_slice(&1u16.to_le_bytes());
    ico.extend_from_slice(&(images.len() as u16).to_le_bytes());

    let mut offset = (6 + images.len() * 16) as u32;
    for (size, dib) in &images {
        let dim = if *size >= 256 { 0 } else { *size as u8 };
        ico.push(dim);
        ico.push(dim);
        ico.push(0);
        ico.push(0);
        ico.extend_from_slice(&1u16.to_le_bytes());
        ico.extend_from_slice(&32u16.to_le_bytes());
        ico.extend_from_slice(&(dib.len() as u32).to_le_bytes());
        ico.extend_from_slice(&offset.to_le_bytes());
        offset += dib.len() as u32;
    }
    for (_, dib) in &images {
        ico.extend_from_slice(dib);
    }
    ico
}

fn dib_image(rgba: &[u8], width: u32, height: u32) -> Vec<u8> {
    let mut dib = Vec::with_capacity(40 + rgba.len() + ((width + 31) / 32 * 4 * height) as usize);
    dib.extend_from_slice(&40u32.to_le_bytes());
    dib.extend_from_slice(&(width as i32).to_le_bytes());
    dib.extend_from_slice(&((height as i32) * 2).to_le_bytes());
    dib.extend_from_slice(&1u16.to_le_bytes());
    dib.extend_from_slice(&32u16.to_le_bytes());
    dib.extend_from_slice(&0u32.to_le_bytes());
    dib.extend_from_slice(&0u32.to_le_bytes());
    dib.extend_from_slice(&0i32.to_le_bytes());
    dib.extend_from_slice(&0i32.to_le_bytes());
    dib.extend_from_slice(&0u32.to_le_bytes());
    dib.extend_from_slice(&0u32.to_le_bytes());

    for y in (0..height).rev() {
        for x in 0..width {
            let i = ((y * width + x) * 4) as usize;
            dib.extend_from_slice(&[rgba[i + 2], rgba[i + 1], rgba[i], rgba[i + 3]]);
        }
    }

    let row_bytes = ((width + 31) / 32) * 4;
    let mut mask = vec![0u8; (row_bytes * height) as usize];
    for y in 0..height {
        let dest_y = height - 1 - y;
        for x in 0..width {
            if rgba[((y * width + x) * 4 + 3) as usize] < 128 {
                let byte = (dest_y * row_bytes + x / 8) as usize;
                mask[byte] |= 0x80 >> (x % 8);
            }
        }
    }
    dib.extend_from_slice(&mask);
    dib
}
