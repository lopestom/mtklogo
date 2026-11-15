mod container;
mod header;
mod logo;

pub use self::container::{ImageInfo, join_parts, logo_images};
pub use self::header::{MtkHeader, MtkType};
pub use self::logo::{LogoImage, LogoTable};

trait StartExt {
    /// checks whether one byte array "starts with" another byte array,
    /// assuming the byte array is ascii characters and case is ignored.
    fn starts_with_ascii_ignore_case(&self, with: &Self) -> bool;
}

impl StartExt for [u8] {
    fn starts_with_ascii_ignore_case(&self, with: &[u8]) -> bool {
        if self.len() < with.len() {
            return false;
        }
        self[..with.len()]
            .iter()
            .zip(with)
            .all(|(c, w)| *c == w.to_ascii_lowercase() || *c == w.to_ascii_uppercase())
    }
}
