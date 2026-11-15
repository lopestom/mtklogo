use super::{cmd, data1, data2, emphasize1, warn};
use mtklogo::{ImageInfo, join_parts, logo_images};
use std::fs;
use std::io::{Error as IOError, ErrorKind, Result};
use std::path::PathBuf;

/// File/folder name for the image `idx` (0-based): its header label reduced to
/// `[A-Za-z0-9_-]`, made unique when two images share the same label.
pub(crate) fn image_dir_name(idx: usize, infos: &[ImageInfo]) -> String {
    let clean = |n: &str| -> String {
        n.chars()
            .filter(|c| c.is_ascii_alphanumeric() || *c == '_' || *c == '-')
            .collect()
    };
    let base = clean(&infos[idx].name);
    let base = if base.is_empty() {
        format!("image{}", idx + 1)
    } else {
        base
    };
    let clashes = infos
        .iter()
        .filter(|i| clean(&i.name).eq_ignore_ascii_case(&base))
        .count();
    if clashes > 1 {
        format!("{}_{}", base, idx + 1)
    } else {
        base
    }
}

/// `split`: writes every `logo*` image of a container as a file of its own.
pub fn run_split(path: PathBuf, output: PathBuf) -> Result<()> {
    println!(
        "{} file {} into directory {}.",
        cmd("split"),
        emphasize1(path.display()),
        emphasize1(output.display())
    );
    let data = fs::read(&path)?;
    let infos = logo_images(&data)?;
    let mut parts = Vec::with_capacity(infos.len());
    for (i, info) in infos.iter().enumerate() {
        let file = output.join(format!("{}.bin", image_dir_name(i, &infos)));
        let bytes = &data[info.range.clone()];
        fs::write(&file, bytes)?;
        println!(
            "image {} ({}): {} bytes -> {}",
            data1(i + 1),
            data2(&info.name),
            bytes.len(),
            emphasize1(file.display())
        );
        parts.push(bytes.to_vec());
    }
    if infos.len() == 1 {
        println!("{}: this file holds a single logo image.", warn("note"));
    }
    println!(
        "parts joined back match the original: {}",
        data1(join_parts(&parts) == data)
    );
    Ok(())
}

/// `join`: concatenates image files, aligning each one.
pub fn run_join(output: PathBuf, files: Vec<PathBuf>) -> Result<()> {
    println!(
        "{} {} files into {}.",
        cmd("join"),
        data1(files.len()),
        emphasize1(output.display())
    );
    if files.is_empty() {
        return Err(IOError::new(ErrorKind::InvalidInput, "no files to join"));
    }
    let parts = files
        .iter()
        .map(fs::read)
        .collect::<Result<Vec<Vec<u8>>>>()?;
    let joined = join_parts(&parts);
    fs::write(&output, &joined)?;
    println!(
        "successfully joined {} bytes to {}",
        data1(joined.len()),
        emphasize1(output.display())
    );
    Ok(())
}
