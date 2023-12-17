use std::io::Read;
use std::path::Path;

pub fn load_font(path: &Path) -> Result<Font, std::io::Error> {
    let font = match path.extension().and_then(|s| s.to_str()) {
        Some("uf2") => {
            let mut file = std::fs::File::open(path)?;
            let mut buf = [0; fleck::FILE_SIZE];
            file.read_exact(&mut buf)?;
            Font::Uf2(Box::new(fleck::Font::new(&buf)))
        }
        Some(_) | None => {
            // Try whether it's psf2.
            let mut file = std::fs::File::open(path)?;
            let mut buf = Vec::new();
            file.read_to_end(&mut buf)?;
            Font::Psf2(psf2::Font::new(buf).map_err(std::io::Error::other)?)
        }
    };
    Ok(font)
}

#[derive(Clone)]
pub enum Font {
    Psf2(psf2::Font<Vec<u8>>),
    Uf2(Box<fleck::Font>),
}

impl Font {
    pub fn height(&self) -> usize {
        match self {
            Font::Psf2(font) => font.height() as usize,
            Font::Uf2(font) => font.height(),
        }
    }

    pub fn determine_width(&self, s: &str) -> usize {
        match self {
            // psf2 fonts are fixed-width, so the width determination is trivial.
            Font::Psf2(font) => s.len() * font.width() as usize,
            Font::Uf2(font) => font.determine_width(s),
        }
    }

    pub fn glyph(&self, ch: char) -> Option<Glyph> {
        match self {
            Font::Psf2(font) => font
                .get_unicode(ch)
                .map(|glyph| Glyph::Psf2(glyph, font.width())),
            Font::Uf2(font) => font.glyph(ch).map(Glyph::Uf2),
        }
    }
}

#[derive(Clone)]
pub enum Glyph<'g> {
    Psf2(psf2::Glyph<'g>, u32),
    Uf2(fleck::Glyph<'g>),
}

impl<'g> Iterator for Glyph<'g> {
    type Item = Row<'g>;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            Glyph::Psf2(rows, _width) => rows.next().map(|r| r.into()),
            Glyph::Uf2(rows) => rows.next().map(|r| r.into()),
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let size = match self {
            Glyph::Psf2(rows, _width) => rows.len(),
            Glyph::Uf2(rows) => rows.len(),
        };
        (size, Some(size))
    }
}

impl ExactSizeIterator for Glyph<'_> {}

impl Glyph<'_> {
    pub fn width(&self) -> usize {
        match self {
            Glyph::Psf2(_gl, width) => *width as usize,
            Glyph::Uf2(gl) => gl.width as usize,
        }
    }
}

pub enum Row<'g> {
    Psf2(psf2::GlyphRow<'g>),
    Uf2(fleck::Row),
}

impl<'g> From<psf2::GlyphRow<'g>> for Row<'g> {
    fn from(row: psf2::GlyphRow<'g>) -> Self {
        Self::Psf2(row)
    }
}

impl From<fleck::Row> for Row<'_> {
    fn from(row: fleck::Row) -> Self {
        Self::Uf2(row)
    }
}

impl Iterator for Row<'_> {
    type Item = bool;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            Row::Psf2(row) => row.next(),
            Row::Uf2(row) => row.next(),
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let size = match self {
            Row::Psf2(row) => row.len(),
            Row::Uf2(row) => row.len(),
        };
        (size, Some(size))
    }
}

impl ExactSizeIterator for Row<'_> {}
