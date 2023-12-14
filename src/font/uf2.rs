pub const FILE_SIZE: usize = 0x2100;
const TILE_ROWS: usize = 8;
const TILES: usize = 4;
use std::iter::FromIterator;
use std::convert::TryInto;

/// A [`Tile`] represents 8x8 pixels as 8 rows of 8 bits.
type Tile = [u8; TILE_ROWS];
/// A [`GlyphTiles`] is a square of 4 tiles that form the underlying data for a [`Glyph`].
type GlyphTiles<'t> = [&'t Tile; TILES];

/// A [`Glyph`] represents one character.
#[derive(Debug, Clone, Copy)]
pub struct Glyph<'t> {
    pub width: usize,
    tiles: GlyphTiles<'t>,
}

#[derive(Debug, Clone, Copy)]
pub struct Rows {
    y: usize,
    width: usize,
    rows: [[bool; 16]; 16],
}

impl From<Glyph<'_>> for Rows {
    fn from(glyph: Glyph<'_>) -> Self {
        Self {
            y: 0,
            width: glyph.width,
            rows: glyph.full_rows(),
        }
    }
}

impl Iterator for Rows {
    type Item = Box<[bool]>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.y >= 16 {
            return None;
        }

        let row = Box::from_iter(self.rows[self.y][..self.width].iter().copied());
        self.y += 1;
        Some(row)
    }
}

impl<'t> Glyph<'t> {
    fn from_tiles(width: usize, tiles: GlyphTiles<'t>) -> Self {
        Self { width, tiles }
    }

    /// Returns the full (16 wide) row at the specified y coordinate.
    ///
    /// # Panics
    ///
    /// Panics if `y` is not in the range 0..16.
    fn full_row(&self, y: usize) -> [bool; 16] {
        assert!(y < 16, "y out of 0..16 range");

        let mut ret = [false; 16];
        for (x, px) in ret[..self.width].iter_mut().enumerate() {
            *px = self.get_px_uncheched(x, y)
        }
        ret
    }

    /// Returns the full (16 wide) rows of this [`Glyph`].
    fn full_rows(&self) -> [[bool; 16]; 16] {
        (0..16)
            .map(|y| self.full_row(y))
            .collect::<Vec<_>>()
            .try_into()
            .unwrap()
    }

    pub fn rows(self) -> Rows {
        Rows::from(self)
    }

    /// # Panics
    ///
    /// If `x` or `y` falls outside of the 0..16 range, this function will panic.
    #[inline]
    fn get_px_uncheched(&self, x: usize, y: usize) -> bool {
        // TODO: Should I keep this assert? Measure to find out I guess. May actually be better
        // since the compiler can know that this invariant holds beyond this assert.
        assert!(x < 16 && y < 16, "x or y out of 0..16 range");

        let glyph_idx = (x / 8) * 2 + y / 8;
        let bit_idx = x % 8;
        let bit = self.tiles[glyph_idx][y % 8] >> (7 - bit_idx) & 1;
        bit != 0
    }
}

type Widths = [u8; 256];
type Glyphs = [u8; 8192];

/// A uf2 font.
///
/// Details can be found on
/// [XXIIVV -- ufx font format](https://wiki.xxiivv.com/site/ufx_format.html)
/// and related repositories.
#[derive(Debug, Clone)]
#[repr(C)]
pub struct Font {
    widths: Widths,
    glyphs: Glyphs,
}

impl Font {
    fn glyph_tiles(&self, n: usize) -> Option<GlyphTiles> {
        self.glyphs.array_chunks().array_chunks().nth(n)
    }

    pub fn glyph(&self, ch: char) -> Option<Glyph> {
        if !ch.is_ascii() {
            return None;
        }

        let n = ch as u8 as usize;
        let width = self.widths[n] as usize;
        let tiles = self.glyph_tiles(n).unwrap();
        Some(Glyph::from_tiles(width, tiles))
    }

    pub(crate) const fn height(&self) -> usize {
        TILES / 2 * TILE_ROWS
    }

    pub(crate) fn determine_width(&self, s: &str) -> usize {
        s.chars()
            .flat_map(|ch| self.glyph(ch))
            .map(|g| g.width)
            .sum()
    }
}

impl Font {
    pub fn from_uf2(bytes: &[u8; FILE_SIZE]) -> Self {
        let (widths, glyphs) = bytes.split_at(std::mem::size_of::<Widths>());
        Self {
            widths: widths.try_into().unwrap(),
            glyphs: glyphs.try_into().unwrap(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn correct_font_size() {
        assert_eq!(std::mem::size_of::<Font>(), FILE_SIZE);
    }
}
