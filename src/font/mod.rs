use std::{io::Read, path::Path, slice::ChunksExact};

mod uf2;

#[derive(Debug, Clone)]
pub struct GenericGlyph {
    buf: Vec<bool>,
    width: usize,
}

impl From<uf2::Glyph<'_>> for GenericGlyph {
    fn from(value: uf2::Glyph<'_>) -> Self {
        // TODO: Oh god. This will all go when I completely redo the uf2 stuff and make a crate
        // for it. In the future, I guess. Onwards, forwards.
        let mut buf = Vec::new();
        for row in value.rows() {
            for &cell in row.iter() {
                buf.push(cell)
            }
        }
        Self {
            buf,
            width: value.width,
        }
    }
}

impl From<psf2::Glyph<'_>> for GenericGlyph {
    fn from(value: psf2::Glyph<'_>) -> Self {
        // TODO: This is not that nice either. Investigate what we can do here instead.
        let height = value.clone().count();
        let mut buf = Vec::new();
        for row in value {
            for cell in row {
                buf.push(cell)
            }
        }
        let width = buf.len() / height;
        Self { buf, width }
    }
}

type Rows<'c> = ChunksExact<'c, bool>;

impl GenericGlyph {
    pub fn width(&self) -> usize {
        self.width
    }

    pub fn rows(&self) -> Rows {
        self.buf.chunks_exact(self.width())
    }
}

pub trait Font {
    fn height(&self) -> usize;
    fn determine_width(&self, s: &str) -> usize;
    fn glyph(&self, ch: char) -> Option<GenericGlyph>;
}

pub fn load_font(path: &Path) -> Result<WrappedFont, std::io::Error> {
    let font = match path.extension().and_then(|s| s.to_str()) {
        Some("uf2") => {
            let mut file = std::fs::File::open(path)?;
            let mut buf = [0; uf2::FILE_SIZE];
            file.read_exact(&mut buf)?;
            WrappedFont::Uf2(Box::new(uf2::Font::from_uf2(&buf)))
        }
        Some(_) | None => {
            // Try whether it's psf2.
            let mut file = std::fs::File::open(path)?;
            let mut buf = Vec::new();
            file.read_to_end(&mut buf)?;
            WrappedFont::Psf2(psf2::Font::new(buf).map_err(|err| std::io::Error::other(err))?)
        }
    };
    Ok(font)
}

#[derive(Clone)]
pub enum WrappedFont {
    Psf2(psf2::Font<Vec<u8>>),
    Uf2(Box<uf2::Font>),
}

impl Font for WrappedFont {
    fn height(&self) -> usize {
        match self {
            WrappedFont::Psf2(font) => font.height() as usize,
            WrappedFont::Uf2(font) => font.height(),
        }
    }

    fn determine_width(&self, s: &str) -> usize {
        match self {
            // psf2 fonts are fixed-width, so the width determination is trivial.
            WrappedFont::Psf2(font) => s.len() * font.width() as usize,
            WrappedFont::Uf2(font) => font.determine_width(s),
        }
    }

    fn glyph(&self, ch: char) -> Option<GenericGlyph> {
        match self {
            WrappedFont::Psf2(font) => font.get_unicode(ch).map(|g| g.into()),
            WrappedFont::Uf2(font) => font.glyph(ch).map(|g| g.into()),
        }
    }
}
