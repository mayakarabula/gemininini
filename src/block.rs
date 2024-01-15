use crate::{Pixel, Rows, RowsMut, PIXEL_SIZE};

// TODO: The methods for this trait rely on a Font argument. I think I want Font to be specified by
// whatever Self it is implemented for.
pub(crate) trait DrawBlock {
    fn block(&self) -> Block;
}

pub(crate) struct Block {
    /// Width in pixels.
    pub(crate) width: u32,
    /// Height in pixels.
    pub(crate) height: u32,
    /// Row-major pixel buffer.
    pub(crate) buf: Vec<Pixel>, // TODO: Invariant: buf.len() == width * height
}

impl Block {
    /// Creates a new [`Block`].
    pub(crate) fn new(width: u32, height: u32, background: Pixel) -> Self {
        Self {
            width,
            height,
            buf: vec![background; width as usize * height as usize],
        }
    }

    /// Returns an iterator over the rows in this [`Block`].
    ///
    /// # Panics
    ///
    /// If `self.width` is 0, the function will panic.
    pub(crate) fn rows(&self) -> Rows {
        self.buf.chunks_exact(self.width as usize)
    }

    /// Returns an iterator over mutable rows in this [`Block`].
    ///
    /// # Panics
    ///
    /// If `self.width` is 0, the function will panic.
    pub(crate) fn rows_mut(&mut self) -> RowsMut {
        self.buf.chunks_exact_mut(self.width as usize)
    }

    // TODO: Improve this weirdly worded doc comment.
    // TODO: Doc comment out of date because we now also do height.
    /// Paint another [`Block`] onto this one.
    ///
    /// If the remaining space in the `Block` after `start_x` is smaller than the width of the
    /// `other`, the first pixels of `other` are drawn up to the border of `self`.
    ///
    /// # Panics
    ///
    /// In case `start_x` is greater than the width of the `Block` that is painted onto
    /// (`self.width`), this function will panic.
    pub(crate) fn paint(&mut self, other: &Self, start_x: u32, start_y: u32) {
        if other.width == 0 || self.width == 0 {
            return;
        }
        assert!(start_x <= self.width);
        assert!(start_y <= self.height);
        let delta_x = u32::min(other.width, self.width - start_x);
        let delta_y = u32::min(other.height, self.height - start_y);
        let end_x = start_x + delta_x;
        for (row, other_row) in self
            .rows_mut()
            .skip(start_y as usize)
            .take(delta_y as usize)
            .zip(other.rows())
        {
            row[start_x as usize..end_x as usize].copy_from_slice(&other_row[..delta_x as usize])
        }
    }

    /// Draws this [`Block`]s contents onto the provided pixel buffer.
    ///
    /// The pixel buffer is provided as a mutable slice of bytes. It is assumed that this buffer
    /// uses the same pixel representation as [`Block`], which is 32-bit rgba pixels.
    ///
    /// See also: [`Pixel`].
    pub(crate) fn draw_onto_pixels(&self, pixels: &mut [u8]) {
        assert!(
            pixels.len() >= self.buf.len(),
            "pixel buffer is not large enough"
        );
        for (y, row) in self.rows().enumerate() {
            let idx = y * self.width as usize * PIXEL_SIZE;
            // TODO: See if we can get rid of this iter(). Perhaps through feature(slice_flatten)?
            // TODO: Where should the .copied() go, ideally?
            let row_bytes: Vec<_> = row.iter().copied().flatten().collect();
            pixels[idx..idx + row_bytes.len()].copy_from_slice(&row_bytes);
        }
    }
}
