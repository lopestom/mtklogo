use super::super::config::{Config, Format};
use super::split::image_dir_name;
use super::{cmd, data1, data2, data3, emphasize1, emphasize2, err, warn};
use crate::Profile;
use mtklogo::utils::{image::ImageIO, z_lib};
use mtklogo::{ColorMode, ContentType, FileInfo, ImageInfo, LogoImage, logo_images};
use std::fs::File;
use std::io::{BufWriter, Cursor, Error as IOError, ErrorKind, Result, Write};
use std::path::{Path, PathBuf};

/// Which image of a (possibly multi-image) container `unpack` reads.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ImageSel {
    /// Historical behaviour: the first image; anything after it stays opaque.
    First,
    /// Every logo image, one sub-folder each (a single image is extracted flat).
    All,
    /// 1-based position: `1` is `logo1`, `2` is `logo2`...
    Number(usize),
    /// Header label, e.g. `logo2`.
    Name(String),
}

impl ImageSel {
    pub fn parse(arg: &str) -> ImageSel {
        let a = arg.trim();
        if a.eq_ignore_ascii_case("all") {
            ImageSel::All
        } else if let Ok(n) = a.parse::<usize>() {
            ImageSel::Number(n)
        } else {
            ImageSel::Name(a.to_string())
        }
    }
}

/// All relevant inputs required by the `unpack` subcommand.
pub struct UnpackRequest {
    pub config: Config,
    pub slots: Option<Vec<usize>>,
    pub profile_name: String,
    pub mode: Option<String>,
    pub flip: bool,
    pub zip: bool,
    pub check: bool,
    pub image: ImageSel,
    pub path: PathBuf,
    pub output: PathBuf,
}

pub fn run_unpack(req: UnpackRequest) -> Result<()> {
    let UnpackRequest {
        config,
        slots,
        profile_name,
        mode,
        flip,
        zip,
        check,
        image,
        path,
        output,
    } = req;
    /// does profile have this name or this alias?
    fn match_name_or_alias(profile: &Profile, name: &str) -> bool {
        if profile.name.eq(name) {
            true
        } else {
            match profile.alias {
                None => false,
                Some(ref aliases) => aliases.contains(&name.to_string()),
            }
        }
    }
    // What is active profile?
    let maybe_profile = config
        .profiles
        .iter()
        .find(|profile| match_name_or_alias(profile, &profile_name));
    let mut profile = match maybe_profile {
        Some(p) => Ok(p.clone()),
        None => Err(IOError::new(
            ErrorKind::InvalidData,
            format!(
                "profile '{}' is not declared in configuration file",
                profile_name
            ),
        )),
    }?;
    // User may override color model.
    if let Some(model) = mode {
        profile = profile.with_color_model(model);
    };
    let mtk_color_model = ColorMode::by_name(&profile.color_model)?;
    println!(
        "{} file {} with profile {}, color mode {}, flip orientation: {} to directory {}.",
        cmd("unpack"),
        emphasize1(path.display()),
        data1(profile_name),
        data2(format!("{}", mtk_color_model)),
        emphasize1(format!("{}", flip)),
        emphasize1(output.display())
    );

    // Reads the whole file: images of a container are located by walking its headers.
    let data = std::fs::read(&path)?;

    let format_provider = |sz: u32| profile.guess_format(sz, flip);
    let mut had_error = false;

    if image == ImageSel::First {
        // Historical behaviour, untouched: the first image, the rest stays opaque `cert`.
        let img = LogoImage::read(&mut Cursor::new(&data))?;
        println!("logo image has {} slots", data1(img.blobs.len()));
        warn_hidden_images(&data);
        had_error |= unpack_image(
            &img,
            &slots,
            check,
            zip,
            mtk_color_model,
            &output,
            format_provider,
        )?;
    } else {
        let infos = logo_images(&data)?;
        let picked: Vec<usize> = match &image {
            ImageSel::All => (0..infos.len()).collect(),
            ImageSel::Number(0) => {
                return Err(IOError::new(
                    ErrorKind::InvalidInput,
                    "images are numbered from 1 (1 = logo1, 2 = logo2...)",
                ));
            }
            ImageSel::Number(n) if *n <= infos.len() => vec![*n - 1],
            ImageSel::Name(name) => {
                match infos.iter().position(|i| i.name.eq_ignore_ascii_case(name)) {
                    Some(i) => vec![i],
                    None => return Err(no_such_image(&image, &infos)),
                }
            }
            _ => return Err(no_such_image(&image, &infos)),
        };
        let multi_dirs = image == ImageSel::All && infos.len() > 1;
        for i in picked {
            let info = &infos[i];
            let img = LogoImage::read(&mut Cursor::new(&data[info.range.clone()]))?;
            let dir = if multi_dirs {
                let d = output.join(image_dir_name(i, &infos));
                std::fs::create_dir_all(&d)?;
                d
            } else {
                output.clone()
            };
            println!(
                "image {}/{} ({}) -> {}",
                data1(i + 1),
                infos.len(),
                data2(&info.name),
                emphasize1(dir.display())
            );
            println!("logo image has {} slots", data1(img.blobs.len()));
            had_error |= unpack_image(
                &img,
                &slots,
                check,
                zip,
                mtk_color_model,
                &dir,
                format_provider,
            )?;
        }
    }
    if had_error {
        return Err(IOError::new(
            ErrorKind::InvalidData,
            "one or more slots failed the format check",
        ));
    }
    Ok(())
}

/// Extracts (or checks) the slots of one image and writes its `.mtklogo-meta`.
fn unpack_image<F>(
    image: &LogoImage,
    slots: &Option<Vec<usize>>,
    check: bool,
    zip: bool,
    color_mode: &ColorMode,
    output: &Path,
    format_provider: F,
) -> Result<bool>
where
    F: Fn(u32) -> Result<Format> + Copy,
{
    let mut had_error = false;
    for (id, blob) in image.blobs.iter().enumerate() {
        let should_process = match slots {
            None => true,
            Some(s) => s.contains(&id),
        };
        if !should_process {
            continue;
        }
        if check {
            if check_logo(id, blob, zip, color_mode, output, format_provider).is_err() {
                had_error = true;
            }
        } else {
            extract_logo(id, blob, zip, color_mode, output, format_provider)?;
        }
    }
    // Persist the header and cert so repack can rebuild and re-append them.
    use super::meta;
    let _ = meta::write(output, image);
    Ok(had_error)
}

/// Tells the user when `LogoImage::read` left logo images unread.
fn warn_hidden_images(data: &[u8]) {
    if let Ok(infos) = logo_images(data)
        && infos.len() > 1
    {
        let names: Vec<&str> = infos.iter().skip(1).map(|i| i.name.as_str()).collect();
        println!(
            "{}: this file holds {} logo images; only the first was read ({} left as opaque data). \
Use --image all, --image <n> or the split command to reach them.",
            warn("warning"),
            infos.len(),
            names.join(", ")
        );
    }
}

fn no_such_image(sel: &ImageSel, infos: &[ImageInfo]) -> IOError {
    let names: Vec<&str> = infos.iter().map(|i| i.name.as_str()).collect();
    IOError::new(
        ErrorKind::InvalidInput,
        format!(
            "no such image '{}': the file holds {} logo image(s): {}",
            match sel {
                ImageSel::Number(n) => n.to_string(),
                ImageSel::Name(n) => n.clone(),
                ImageSel::All => "all".to_string(),
                ImageSel::First => "1".to_string(),
            },
            infos.len(),
            names.join(", ")
        ),
    )
}

fn extract_logo<F>(
    id: usize,
    blob: &[u8],
    zip: bool,
    color_mode: &ColorMode,
    outpath: &Path,
    format_provider: F,
) -> Result<()>
where
    F: Fn(u32) -> Result<Format>,
{
    let info = FileInfo::from_info(id, zip, color_mode);
    // computes the output name.
    let output_file = outpath.join(info.filename());
    match &info.content_type {
        ContentType::Z => export_raw(&info, &output_file, blob),
        ContentType::PNG(e) => {
            export_png(&info, &output_file, blob, color_mode, format_provider).or_else(|er| {
                println!(
                    "{} slot {} as {} because {}. Falling back to raw .z.",
                    warn("Could not export"),
                    data1(id),
                    emphasize1(e),
                    err(er.to_string()),
                );
                // invalidates names.
                let info = FileInfo::from_info(id, true, color_mode);
                // computes the output name.
                let output_file = outpath.join(info.filename());
                export_raw(&info, &output_file, blob)
            })
        }
    }
}

fn check_logo<F>(
    id: usize,
    blob: &[u8],
    zip: bool,
    color_mode: &ColorMode,
    outpath: &Path,
    format_provider: F,
) -> Result<()>
where
    F: Fn(u32) -> Result<Format>,
{
    let info = FileInfo::from_info(id, zip, color_mode);
    // computes the output name.
    let output_file = outpath.join(info.filename());
    match info.content_type {
        ContentType::Z => println!(
            "slot {} is {} bytes and will be exported as raw zip to {}",
            id,
            blob.len(),
            output_file.display()
        ),
        ContentType::PNG(_) => {
            let exported = z_lib::inflate(blob)
                // then resolves the couple (format, inflated).
                .and_then(|inflated| format_provider(inflated.len() as u32).map(|format| (format, inflated)))
                .map(|(format, inflated)| {
                    println!("slot {} is {} bytes ({} inflated) and will be exported as {}x{} image to {}",
                             id, blob.len(), inflated.len(), format.w, format.h, output_file.display());
                });
            if let Some(er) = exported.err() {
                println!(
                    "{} slot {} ({} bytes) as an image : {}",
                    warn("Cannot export"),
                    id,
                    blob.len(),
                    warn(er.to_string())
                );
                return Err(IOError::other(format!("slot {} check failed", id)));
            }
        }
    };
    Ok(())
}

fn export_raw(info: &FileInfo, output_file: &Path, blob: &[u8]) -> Result<()> {
    println!(
        "storing slot {} ({} bytes) to {} as raw zip .",
        data1(info.id),
        data2(blob.len()),
        emphasize1(output_file.display())
    );
    let mut f = File::create(output_file)?;
    f.write_all(blob)
}

fn export_png<F>(
    info: &FileInfo,
    output_file: &Path,
    blob: &[u8],
    color_mode: &ColorMode,
    format_provider: F,
) -> Result<()>
where
    F: Fn(u32) -> Result<Format>,
{
    // inflates the zip
    z_lib::inflate(blob)
        // then resolves the couple (format, inflated).
        .and_then(|inflated| {
            format_provider(inflated.len() as u32).map(|format| (format, inflated))
        })
        .and_then(|(format, inflated)| {
            let file = File::create(output_file)?;
            let file_writer = BufWriter::new(file);
            println!(
                "storing slot {} ({} bytes) to {} as {}x{} {} image.",
                data1(info.id),
                data2(blob.len()),
                emphasize1(output_file.display()),
                data3(format.w),
                data3(format.h),
                emphasize2(color_mode)
            );
            color_mode.write_png(file_writer, &inflated, format.w, format.h)
        })
}
