use super::meta;
use super::{cmd, data1, data2, emphasize1, emphasize2, warn};
use mtklogo::utils::{image, image::ImageIO, load_raw, z_lib};
use mtklogo::{ContentType, FileInfo, LogoImage, MtkHeader};
use std::fs::File;
use std::io::{BufReader, BufWriter, Cursor, Error as IOError, ErrorKind, Result};
use std::path::PathBuf;

pub fn run_repack(
    outpath: PathBuf,
    files: Vec<PathBuf>,
    strip_alpha: bool,
    reference: Option<PathBuf>,
    force: bool,
) -> Result<()> {
    println!(
        "{} {} files into {} stripping alpha: {}.",
        cmd("repack"),
        data1(files.len()),
        emphasize1(outpath.display()),
        data2(strip_alpha)
    );

    // Resolve the input directory before `files` is moved, so we can locate
    // the `.mtklogo-meta` sidecar written by `unpack`.
    let input_dir = files
        .iter()
        .filter_map(|f| f.parent())
        .next()
        .map(PathBuf::from);
    let packable_files = reorder(files)?;
    // extracts blob data.
    let mut blobs = Vec::with_capacity(packable_files.len());
    for file in packable_files.iter() {
        blobs.push(import_logo(file, strip_alpha)?);
    }
    let count = blobs.len();
    if let Some(reference) = reference.as_ref() {
        check_against_reference(reference, &blobs, force)?;
    }
    let mut image = LogoImage::new_blobs(blobs)?;
    // Restore the device header and re-append the cert from the sidecar.
    if let Some(dir) = input_dir
        && let Some(m) = meta::load(&dir)
    {
        if let Ok(h) = MtkHeader::read(&mut Cursor::new(&m.header)) {
            image.table.header = h;
        }
        image.cert = m.cert;
        image.cert_inner_len = m.cert_inner_len;
    }
    let mut writer = BufWriter::new(File::create(&outpath)?);
    image.write(&mut writer)?;
    drop(writer);
    println!(
        "successfully repacked {} logos to {}",
        data1(count),
        emphasize1(outpath.display())
    );
    if let Some(reference) = reference.as_ref()
        && let (Ok(new), Ok(old)) = (std::fs::metadata(&outpath), std::fs::metadata(reference))
    {
        let (new, old) = (new.len(), old.len());
        let delta = new as i64 - old as i64;
        println!(
            "size: {} bytes (original {} bytes, {:+} bytes)",
            data1(new),
            data2(old),
            delta
        );
        // The real limit is the size of the logo partition, which we can't know;
        // a noticeable growth is worth a warning though.
        if new > old + old / 20 {
            println!(
                "{}: the new image is more than 5% bigger than the original; check that it fits in your logo partition.",
                warn("warning")
            );
        }
    }
    Ok(())
}

/// Compares the new blobs with the ones of the original `logo.bin`.
/// A slot whose decompressed size changed means the picture has other
/// dimensions than the device expects.
fn check_against_reference(reference: &PathBuf, blobs: &[Vec<u8>], force: bool) -> Result<()> {
    let mut reader = BufReader::new(File::open(reference)?);
    let original = LogoImage::read(&mut reader)?;
    let mut problems = 0usize;
    if original.blobs.len() != blobs.len() {
        println!(
            "{}: the original has {} slots but {} files were given",
            warn("mismatch"),
            original.blobs.len(),
            blobs.len()
        );
        problems += 1;
    }
    for (id, (old, new)) in original.blobs.iter().zip(blobs.iter()).enumerate() {
        let old_len = z_lib::inflate(old).map(|v| v.len());
        let new_len = z_lib::inflate(new).map(|v| v.len());
        if let (Ok(o), Ok(n)) = (old_len, new_len)
            && o != n
        {
            println!(
                "{}: slot {} decompressed size is {} bytes, original is {} bytes (wrong image dimensions?)",
                warn("mismatch"),
                data1(id),
                n,
                o
            );
            problems += 1;
        }
    }
    if problems == 0 {
        println!(
            "reference check: all {} slots match the original sizes",
            data1(blobs.len())
        );
        Ok(())
    } else if force {
        println!(
            "{}: {} mismatch(es) ignored because of --force",
            warn("warning"),
            problems
        );
        Ok(())
    } else {
        Err(IOError::new(
            ErrorKind::InvalidData,
            format!(
                "{} slot(s) differ from the original; use --force to repack anyway",
                problems
            ),
        ))
    }
}

struct PackableFile {
    path: PathBuf,
    info: FileInfo,
}

fn import_logo(logo: &PackableFile, strip_alpha: bool) -> Result<Vec<u8>> {
    let file = File::open(&logo.path)?;
    match logo.info.content_type {
        ContentType::Z => load_raw(file),
        ContentType::PNG(color_mode) => {
            // loads png as rgba
            let (mut rgba, w, h) = image::png_to_rgba(BufReader::new(file))?;
            // do we want to strip alpha?
            if strip_alpha {
                image::strip_alpha(&mut rgba)
            };
            // converts to device format.
            let device = color_mode.rgba_to_device(&rgba, w, h)?;
            // zipped data.
            z_lib::deflate(&device)
        }
    }
}

fn reorder(files: Vec<PathBuf>) -> Result<Vec<PackableFile>> {
    // Analyses each file.
    let mut analyzed = Vec::with_capacity(files.len());
    for file in files.iter() {
        let name = file.file_name().and_then(|n| n.to_str()).ok_or_else(|| {
            IOError::other(format!("file '{}' has a non-UTF8 name.", file.display()))
        })?;
        let info = FileInfo::from_name(name)?;
        match &info.content_type {
            ContentType::Z => println!(
                "file {} is slot {} in raw z format.",
                emphasize1(file.display()),
                data1(info.id)
            ),
            ContentType::PNG(p) => println!(
                "file {} is slot {} in {} format.",
                emphasize1(file.display()),
                data1(info.id),
                emphasize2(p)
            ),
        }
        analyzed.push(PackableFile {
            path: file.clone(),
            info,
        });
    }
    // returns the ordered list of files;
    analyzed.sort_by_key(|a| a.info.id);
    // The output is a dense 0..n sequence, so reject duplicate or gap ids.
    for (index, file) in analyzed.iter().enumerate() {
        if file.info.id != index {
            return Err(IOError::new(
                ErrorKind::InvalidInput,
                format!(
                    "slot ids must be a contiguous 0..{} sequence, but slot {} maps to id {}",
                    analyzed.len(),
                    index,
                    file.info.id
                ),
            ));
        }
    }
    Ok(analyzed)
}
