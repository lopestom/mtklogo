//! Multi-image MTK containers.
//!
//! Some devices (e.g. Android 16 images) ship a `logo.bin` that chains several
//! images, each followed by its own certificates:
//!
//! ```text
//! [logo1][cert1][cert2] [logo2][cert1][cert2]   (every image 512-byte header + `dsize`)
//! ```
//!
//! [`LogoImage::read`] only understands the first one and keeps everything that
//! follows as an opaque `cert`. The helpers below walk the header chain, the way
//! the loader does, so every `logo*` image can be reached.

use super::StartExt;
use super::header::MtkHeader;
use std::io::{Error as IOError, ErrorKind, Result};
use std::ops::Range;

/// `0x88168858`, big endian: first word of every MTK image header.
const MAGIC: [u8; 4] = [0x88, 0x16, 0x88, 0x58];
/// `0x89168958`, big endian: marks the extended header block at 0x30.
const MAGIC_EXT: [u8; 4] = [0x89, 0x16, 0x89, 0x58];
/// Alignment used when a header does not declare one.
const DEFAULT_ALIGN: usize = 16;

/// One `logo*` image of a container.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImageInfo {
    /// Header label (`logo`, `logo1`, `logo2`, ...).
    pub name: String,
    /// Bytes of the image: header, payload, certificates and padding, up to the
    /// next logo image (or the end of the file). It can be read with
    /// [`LogoImage::read`](super::LogoImage::read) as if it were a file of its own.
    pub range: Range<usize>,
}

struct RawHeader {
    name: String,
    size: usize,
    align: usize,
    is_logo: bool,
}

/// Minimal header reader. `MtkHeader::read` refuses labels it does not know
/// (such as `cert1`/`cert2`), but those still have to be skipped over.
fn raw_header(data: &[u8], pos: usize) -> Option<RawHeader> {
    let h = data.get(pos..pos.checked_add(MtkHeader::SIZE)?)?;
    if h[..4] != MAGIC {
        return None;
    }
    let label = &h[8..40];
    let len = label.iter().position(|b| *b == 0).unwrap_or(label.len());
    let size = u32::from_le_bytes(h[4..8].try_into().ok()?) as usize;
    let align = if h[0x30..0x34] == MAGIC_EXT {
        u32::from_le_bytes(h[0x44..0x48].try_into().ok()?) as usize
    } else {
        0
    };
    Some(RawHeader {
        name: String::from_utf8_lossy(&label[..len]).into_owned(),
        size,
        align: if align > 0 { align } else { DEFAULT_ALIGN },
        is_logo: label.starts_with_ascii_ignore_case(b"logo"),
    })
}

/// Lists every `logo*` image of `data`, in file order.
///
/// The walk starts at offset 0 and follows `header + dsize`, rounded up to the
/// header's `align_size`; certificates and other non-logo images are skipped.
/// It stops quietly at the first thing that is not an MTK header, so a classic
/// single-image `logo.bin` yields exactly one entry spanning the whole file.
pub fn logo_images(data: &[u8]) -> Result<Vec<ImageInfo>> {
    let mut found: Vec<(String, usize)> = Vec::new();
    let mut pos = 0usize;
    while let Some(h) = raw_header(data, pos) {
        if h.is_logo {
            found.push((h.name.clone(), pos));
        } else if found.is_empty() {
            break; // the file must start with a logo image
        }
        let Some(end) = pos
            .checked_add(MtkHeader::SIZE)
            .and_then(|p| p.checked_add(h.size))
        else {
            break;
        };
        pos = end.div_ceil(h.align) * h.align;
        while data.get(pos) == Some(&0) {
            pos += 1;
        }
    }
    if found.is_empty() {
        return Err(IOError::new(
            ErrorKind::InvalidData,
            "MTK image is not a logo",
        ));
    }
    let ends: Vec<usize> = found
        .iter()
        .skip(1)
        .map(|(_, start)| *start)
        .chain(std::iter::once(data.len()))
        .collect();
    Ok(found
        .into_iter()
        .zip(ends)
        .map(|((name, start), end)| ImageInfo {
            name,
            range: start..end,
        })
        .collect())
}

/// Concatenates image files (as produced by `split` or `repack`), starting each
/// one on its declared alignment boundary so that the loader can find it.
pub fn join_parts(parts: &[Vec<u8>]) -> Vec<u8> {
    let mut out = Vec::with_capacity(parts.iter().map(Vec::len).sum());
    for part in parts {
        let align = raw_header(part, 0).map_or(DEFAULT_ALIGN, |h| h.align);
        let rem = out.len() % align;
        if rem != 0 {
            out.resize(out.len() + align - rem, 0);
        }
        out.extend_from_slice(part);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn header(name: &str, size: u32) -> Vec<u8> {
        let mut h = vec![0xFFu8; MtkHeader::SIZE];
        h[..4].copy_from_slice(&MAGIC);
        h[4..8].copy_from_slice(&size.to_le_bytes());
        h[8..40].fill(0);
        h[8..8 + name.len()].copy_from_slice(name.as_bytes());
        h[0x30..0x34].copy_from_slice(&MAGIC_EXT);
        h[0x44..0x48].copy_from_slice(&16u32.to_le_bytes());
        h
    }

    fn image(name: &str, payload: &[u8]) -> Vec<u8> {
        let mut v = header(name, payload.len() as u32);
        v.extend_from_slice(payload);
        v
    }

    fn container() -> Vec<u8> {
        let mut v = image("logo1", &[1u8; 21]);
        v.resize(v.len().div_ceil(16) * 16, 0);
        v.extend(image("cert1", &[2u8; 30]));
        v.resize(v.len().div_ceil(16) * 16, 0);
        v.extend(image("logo2", &[3u8; 40]));
        v.resize(v.len().div_ceil(16) * 16, 0);
        v.extend(image("cert2", &[4u8; 10]));
        v
    }

    #[test]
    fn walks_the_header_chain() {
        let data = container();
        let imgs = logo_images(&data).unwrap();
        assert_eq!(imgs.len(), 2);
        assert_eq!(imgs[0].name, "logo1");
        assert_eq!(imgs[1].name, "logo2");
        assert_eq!(imgs[0].range.end, imgs[1].range.start);
        assert_eq!(imgs[1].range.end, data.len());
    }

    #[test]
    fn single_image_spans_the_whole_file() {
        let mut data = image("logo", &[1u8; 33]);
        data.extend_from_slice(&[0u8; 50]); // trailing padding that is not a header
        let imgs = logo_images(&data).unwrap();
        assert_eq!(imgs.len(), 1);
        assert_eq!(imgs[0].range, 0..data.len());
    }

    #[test]
    fn rejects_non_logo_files() {
        assert!(logo_images(&image("kernel", &[0u8; 16])).is_err());
        assert!(logo_images(b"not an mtk file").is_err());
    }

    #[test]
    fn join_restores_the_container() {
        let data = container();
        let parts: Vec<Vec<u8>> = logo_images(&data)
            .unwrap()
            .iter()
            .map(|i| data[i.range.clone()].to_vec())
            .collect();
        assert_eq!(join_parts(&parts), data);
    }

    #[test]
    fn join_realigns_a_grown_part() {
        let data = container();
        let imgs = logo_images(&data).unwrap();
        let mut a = data[imgs[0].range.clone()].to_vec();
        a.extend_from_slice(&[0u8; 5]); // part grew by 5 bytes
        let b = data[imgs[1].range.clone()].to_vec();
        let joined = join_parts(&[a, b]);
        let again = logo_images(&joined).unwrap();
        assert_eq!(again.len(), 2);
        assert_eq!(again[1].range.start % 16, 0);
    }
}
