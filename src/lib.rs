#![feature(iter_intersperse)]

use block::{Block, DrawBlock};
use elements::{Dimensions, Element};
pub use fleck::Font;

mod block;
pub mod elements;

/// The number of bytes per [`Pixel`].
pub const PIXEL_SIZE: usize = 4;
/// A pixel in the form `[r, g, b, a]`.
pub type Pixel = [u8; PIXEL_SIZE];

/// An iterator over rows of [`Pixel`]s.
type Rows<'b> = std::slice::ChunksExact<'b, Pixel>;
/// An iterator over mutable rows of [`Pixel`]s.
type RowsMut<'b> = std::slice::ChunksExactMut<'b, Pixel>;

/// Representation of the window and associated data of type `D`.
pub struct Panel<D> {
    pub width: u32,
    pub height: u32,
    // TODO: We can probably do without foreground and background since we can just take the values
    // from the root Element. That Element always has a Style. Consider. In fact, I don't think
    // that we actually use this `foreground` meaningfully anywhere.
    pub foreground: Pixel,
    pub background: Pixel,

    data: D,
    pub elements: Element<D>,
}

impl<D> Panel<D> {
    /// Creates a new [`Panel<D>`].
    pub fn new(mut elements: Element<D>, foreground: Pixel, background: Pixel, data: D) -> Self {
        elements.bake_size(None); // We calculate the sizes in order to give the first estimate.
        let Dimensions { width, height } = elements.overall_size();
        Self {
            width,
            height,
            foreground,
            background,
            data,
            elements,
        }
    }

    /// Returns a mutable reference to the data of this [`Panel<D>`].
    pub fn data_mut(&mut self) -> &mut D {
        &mut self.data
    }

    /// Update all elements in this [`Panel<D>`] with the internal `data`.
    pub fn update(&mut self) {
        self.elements.update(&self.data);
        self.elements.bake_size(Some(self.width));
    }

    /// Draw the [`Panel<D>`] onto a pixel buffer.
    ///
    /// The pixel buffer is provided as a mutable slice of bytes. It is assumed that this buffer
    /// uses the same pixel representation as [`Block`], which is 32-bit RGBA pixels.
    ///
    /// See also: [`Pixel`].
    pub fn draw(&self, pixels: &mut [u8]) {
        let mut block = Block::new(self.width, self.height, self.background);

        // Draw onto our block.
        block.paint(&self.elements.block(), 0, 0);

        // Draw the block onto the pixels.
        block.draw_onto_pixels(pixels);
    }

    /// Resize the [`Panel<D>`].
    pub fn resize(&mut self, width: u32, height: u32) {
        self.width = width;
        self.height = height;
    }
}
