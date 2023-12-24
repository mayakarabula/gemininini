use fleck::Font;
pub use graph::Graph;
pub use wrapped_text::WrappedText;

use crate::block::DrawBlock;
use crate::{Block, PIXEL_SIZE};

pub mod graph;
pub mod wrapped_text;

type UpdateFn<D> = fn(&mut ElementKind<D>, &D);

pub struct Element<D> {
    /// If `None`, the width is determined based on the [`Element`] content. If set with
    /// `Some(width)`, this width is taken to be this `Element`s width.
    width: Option<usize>,
    // TODO: Make alignment apply to other ElementKinds besides Text. Once that is done, update doc
    // comments for the with_alignment function and remove todo!() .
    /// The alignment of the contents within the [`Element`] bounds.
    ///
    /// For now, it only applies to [`ElementKind::Text`]. All other cases are drawn as
    /// [`Alignment::Left]`.
    alignment: Alignment,
    /// Function that will update `thing` according to some `data` with type `D`.
    inner_update: Option<UpdateFn<D>>, // TODO: Bikeshedded name.
    thing: ElementKind<D>,
}

impl<D> Element<D> {
    pub fn new(update: Option<UpdateFn<D>>, thing: ElementKind<D>) -> Self {
        Self {
            width: None,
            alignment: Alignment::default(),
            inner_update: update,
            thing,
        }
    }

    pub fn still(thing: ElementKind<D>) -> Self {
        Self::new(None, thing)
    }

    pub fn dynamic(update: UpdateFn<D>, thing: ElementKind<D>) -> Self {
        Self::new(Some(update), thing)
    }

    /// Explicitly set a fixed width for this [`Element`].
    pub fn with_fixed_width(mut self, width: usize) -> Self {
        self.width = Some(width);
        self
    }

    /// Let the width of this [`Element`] adapt based on its content.
    pub fn with_auto_width(mut self) -> Self {
        self.width = None;
        self
    }

    /// Set the width of this [`Element`] according to its content at the moment this function is
    /// called.
    pub fn with_baked_width(mut self, font: &Font) -> Self {
        self.width = Some(self.block_width(font));
        self
    }

    pub fn with_alignment(mut self, alignment: Alignment) -> Self {
        self.alignment = alignment;
        self
    }
}

impl<D> Element<D> {
    /// Update the inner [`ElementKind`] and subsequently let these children call their own update
    /// functions as well, in the case of a collection [`ElementKind`], such as
    /// [`ElementKind::Row`] or [`ElementKind::Stack`].
    pub(crate) fn update(&mut self, data: &D) {
        if let Some(update) = self.inner_update {
            update(&mut self.thing, data)
        }

        // TODO: (easy) Break out the 'is it a collection type' logic to a
        // `ElementKind::is_collection(&self) -> bool`.
        match &mut self.thing {
            // Update the chirren.
            ElementKind::Row(elements) | ElementKind::Stack(elements) => {
                elements.iter_mut().for_each(|element| element.update(data))
            }
            ElementKind::Scroll(element, _, _) => element.update(data),
            // These are not collection ElementKinds, and therefore do not require their children
            // to be updated.
            ElementKind::Space
            | ElementKind::Padding(_)
            | ElementKind::Text(_)
            | ElementKind::Paragraph(_, _, _)
            | ElementKind::Graph(_) => {}
        };
    }
}

impl<D> DrawBlock for Element<D> {
    // TODO: See, this with the font is where my design starts breaking down. I think the a
    // reference to the font should be part of the element.
    fn block_width(&self, font: &Font) -> usize {
        // If the width is set explicitly on the [`Element`], that has priority.
        if let Some(width) = self.width {
            return width;
        }

        // Otherwise, calculate the width from the current content.
        match &self.thing {
            ElementKind::Space => font.determine_width("  ") as usize,
            ElementKind::Padding(width) => *width,
            ElementKind::Text(t) => font.determine_width(&t),
            ElementKind::Paragraph(_, width, _) => *width,
            ElementKind::Graph(g) => g.len(),
            ElementKind::Scroll(element, _, _) => element.block_width(font),
            ElementKind::Row(row) => row.iter().map(|element| element.block_width(font)).sum(),
            ElementKind::Stack(stack) => stack
                .iter()
                .map(|element| element.block_width(font))
                .max()
                .unwrap_or_default(),
        }
    }

    // TODO: Consider not relying on font here. May actually be more correct, since we always have
    // the same Font::GLYPH_HEIGHT?
    fn block_height(&self, _font: &Font) -> usize {
        match &self.thing {
            ElementKind::Space
            | ElementKind::Padding(_)
            | ElementKind::Text(_)
            | ElementKind::Graph(_) => Font::GLYPH_HEIGHT,
            ElementKind::Paragraph(_, _, height) => *height,
            ElementKind::Scroll(_, height, _) => *height,
            ElementKind::Row(row) => row
                .iter()
                .map(|element| element.block_height(_font))
                .max()
                .unwrap_or_default(),
            ElementKind::Stack(stack) => stack
                .iter()
                .map(|element| element.block_height(_font))
                .sum(),
        }
    }

    fn block(&self, font: &Font) -> Block {
        // TODO: Let this point to the Element's style, once we have that.
        let background = [0xff; PIXEL_SIZE];
        let foreground = [0x77, 0x33, 0x22, 0xff];

        let width = self.block_width(font);
        let mut block = Block::new(width, self.block_height(font), background);

        #[inline(always)]
        fn draw_text(
            block: &mut Block,
            text: &str,
            alignment: Alignment,
            font: &Font,
            foreground: [u8; 4],
            background: [u8; 4],
        ) {
            // TODO: This implementation is not ideal. Since a Block is created to fit the entire
            // line of text to then be cut off in a manner that depends on the alignment, we do
            // more allocations than is necessary, and we may render more characters into that
            // Block than is necessary (since they are eventually thrown away due to alignment).
            // A better way of implementing this would be to do the proper computer maths with
            // indices, so that it can all be done through iterators (which evaluate lazily).
            // As a note for such an implementation in the future, see the state at or around
            // commit e945006.

            let scrap_width = font.determine_width(text);
            // TODO: Perhaps this case can be handled a little more gracefull, but that will
            // require a more holistic view of the whole layout "engine" in a later stage.
            // Postponing ;)
            if block.width == 0 || scrap_width == 0 {
                eprintln!("scrap_width == 0, watch out here");
                return; // Nothing to even draw, here. Why expend the energy?
            }
            let mut scrap = Block::new(scrap_width, block.height, background);
            let glyphs = text.chars().flat_map(|ch| font.glyph(ch));
            let mut x0 = 0;
            for glyph in glyphs {
                let glyph_width = glyph.width as usize;
                for (y, row) in glyph.enumerate() {
                    for (xg, cell) in row.enumerate() {
                        let x = x0 + xg;
                        // TODO: This may be more efficient than what I did in Graph. May be
                        // worth investigating which is better.
                        scrap.buf[y * scrap.width + x] = if cell { foreground } else { background };
                    }
                }
                x0 += glyph_width;
            }

            // TODO: Not loving how this match turned out. Seems kind of messy and with repetition
            // that is confusing, since it obscures a more elegant insight.
            match alignment {
                Alignment::Left => {
                    let end = usize::min(block.width, scrap.width);
                    block
                        .rows_mut()
                        .zip(scrap.rows())
                        .for_each(|(row, scrap_row)| {
                            row[..end].copy_from_slice(&scrap_row[..end]);
                        })
                }
                // TODO: The first two branches do the exact same thing. Figure out how to do this
                // nicely.
                // TODO: Decide on the actually correct behavior for Alignment::Center in this case.
                Alignment::Center if scrap.width >= block.width => {
                    let end = usize::min(block.width, scrap.width);
                    block
                        .rows_mut()
                        .zip(scrap.rows())
                        .for_each(|(row, scrap_row)| {
                            row[..end].copy_from_slice(&scrap_row[..end]);
                        })
                }
                Alignment::Center => {
                    let remainder = block.width - scrap.width;
                    let start = remainder / 2;
                    block
                        .rows_mut()
                        .zip(scrap.rows())
                        .for_each(|(row, scrap_row)| {
                            row[start..start + scrap.width].copy_from_slice(&scrap_row);
                        })
                }
                Alignment::Right => {
                    let rstart = block.width.saturating_sub(scrap.width);
                    let sstart = scrap.width.saturating_sub(block.width);
                    block
                        .rows_mut()
                        .zip(scrap.rows())
                        .for_each(|(row, scrap_row)| {
                            row[rstart..].copy_from_slice(&scrap_row[sstart..]);
                        })
                }
            }
        }

        match &self.thing {
            ElementKind::Space | ElementKind::Padding(_) => {}
            ElementKind::Text(text) => draw_text(
                &mut block,
                text,
                self.alignment,
                font,
                foreground,
                background,
            ),
            ElementKind::Paragraph(text, width, height) => {
                let mut y = 0;
                for line in text.as_ref().lines() {
                    let mut line_block = Block::new(*width, font.height(), background);
                    draw_text(
                        &mut line_block,
                        line,
                        self.alignment,
                        font,
                        foreground,
                        background,
                    );
                    let line_block_height = line_block.height;
                    block.paint(line_block, 0, y);
                    y += line_block_height; // FIXME
                    if y > *height {
                        break;
                    }
                }
            }
            ElementKind::Graph(graph) => {
                let min = graph.min();
                let max = graph.max();
                let height = block.height - 1;
                let points = graph
                    .iter()
                    .map(|v| height - ((v - min) / (max - min) * height as f32).round() as usize);
                let mut rows: Vec<_> = block.rows_mut().collect();
                // TODO: (easy) Add at least the option to draw the old-style (like original tid)
                // long bars that extend from the bottom to the (x, y) point.
                for (x, y) in points.enumerate() {
                    rows[y][x] = [0x66, 0x33, 0x99, 0xff]
                }
            }
            ElementKind::Scroll(element, height, pos) => {
                // We can just skip drawing anything if the content is scrolled out of view.
                if height > pos {
                    let mut scrollable = element.block(font);

                    // We cut off the top of this block by `pos` rows, but not beyond the height of the
                    // block.
                    let start = usize::min(scrollable.height, *pos);
                    // TODO: I don't love this .to_vec() here, and we could whip out some fun
                    // unsafe but there's probably a better way regardless. Moving on.
                    scrollable.buf = scrollable.buf[start * scrollable.width..].to_vec();
                    scrollable.height = usize::min(scrollable.height - start, *height);

                    block.paint(scrollable, 0, 0)
                }
            }
            ElementKind::Row(row) => {
                let row_blocks = row.iter().map(|element| element.block(font));
                let mut x = 0;
                for row_block in row_blocks {
                    let width = row_block.width;
                    block.paint(row_block, x, 0);
                    x += width;
                }
            }
            ElementKind::Stack(stack) => {
                let stack_blocks = stack.iter().map(|element| element.block(font));
                let mut y = 0;
                for stack_block in stack_blocks {
                    let height = stack_block.height;
                    block.paint(stack_block, 0, y);
                    y += height;
                }
            }
        }

        block
    }
}

pub enum ElementKind<D> {
    /// Font-dependent spacer.
    Space,
    /// Fixed-sized spacer with a width in pixels.
    Padding(usize),

    /// Simple single-line text. Very useful for labels.
    Text(String),
    // TODO: (easy) May want to convert this to a named struct (Paragraph { text: .., width: .. ..}
    /// Paragraph with wrapped text, a width, and a height.
    Paragraph(WrappedText, usize, usize),
    /// Simple graph.
    Graph(Graph),

    /// Vertically scrolling container.
    Scroll(Box<Element<D>>, usize, usize),
    /// Horizontal container.
    Row(Vec<Element<D>>),
    /// Vertical container.
    Stack(Vec<Element<D>>),
}

#[derive(Default, Clone, Copy)]
pub enum Alignment {
    #[default]
    Left,
    Center,
    Right,
}
