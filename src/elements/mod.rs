use std::rc::Rc;

use fleck::Font;
pub use graph::Graph;
pub use wrapped_text::WrappedText;

use crate::block::DrawBlock;
use crate::{Block, Pixel};

pub mod graph;
pub mod wrapped_text;

type UpdateFn<D> = fn(element: &mut Element<D>, data: &D);

#[derive(Debug, Default, Clone, Copy)]
pub enum SizingStrategy {
    #[default]
    /// The [`Element`] is sized according to the normal rules for the `min` and `max` dimensions
    /// as set by the [`Size`].
    Whatever,
    /// > "Pretty please fill out the room as much as you can!"
    ///
    /// If there's the opportunity to grow right up to the [`Size`]'s `max` dimensions, do it.
    /// When these dimensions are unset, the same rules apply as for [`SizingStrategy::Whatever`].
    Chonker,
    /// > "Hey, please use as little room as possible please."
    ///
    /// If there's the opportunity to shrink right down to the [`Size`]'s `min` dimensions, do it.
    /// When these dimensions are unset, the same rules apply as for [`SizingStrategy::Whatever`].
    Smollest,
}

#[derive(Debug, Default, Clone, Copy)]
/// Specifies the size of an Element.
///
/// Use [`Element::fill_size`] and [`Element::overall_size`] to get the actual sizes constrained by
/// the `min` and `max` dimensions.
pub struct Size {
    pub strategy: SizingStrategy,
    /// The unconstrained width dimension. It may fall outside of `maxwidth` and `minwidth`.
    baked_width: u32,
    /// The unconstrained height dimension. It may fall outside of `maxheight` and `minheight`.
    baked_height: u32,
    pub maxwidth: Option<u32>,
    pub maxheight: Option<u32>,
    pub minwidth: Option<u32>,
    pub minheight: Option<u32>,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Dimensions {
    pub width: u32,
    pub height: u32,
}

impl Dimensions {
    pub fn new(width: u32, height: u32) -> Self {
        Self { width, height }
    }
}

type Pad = u32;

#[derive(Debug, Default, Clone, Copy)]
pub struct Padding {
    pub top: Pad,
    pub bottom: Pad,
    pub left: Pad,
    pub right: Pad,
}

#[derive(Debug, Default, Clone, Copy)]
pub struct Flex {
    pub top: bool,
    pub bottom: bool,
    pub left: bool,
    pub right: bool,
}

impl Flex {
    /// Returns the number of vertical flexes of this [`Flex`].
    fn vertical_flexes(&self) -> usize {
        self.top as usize + self.bottom as usize
    }

    /// Returns the number of horizontal flexes of this [`Flex`].
    fn horizontal_flexes(&self) -> usize {
        self.left as usize + self.right as usize
    }
}

#[derive(Debug, Clone)]
pub struct Style {
    pub foreground: Pixel,
    pub background: Pixel,
    pub font: Rc<Font>,
}

impl Style {
    /// Creates a new [`Style`].
    pub fn new(foreground: Pixel, background: Pixel, font: Rc<Font>) -> Self {
        Self {
            foreground,
            background,
            font,
        }
    }

    /// Creates a new [`Style`] with a black `foreground`, white `background`, and the specified
    /// `font`.
    pub fn default_with_font(font: Rc<Font>) -> Self {
        Self {
            foreground: [0x00, 0x00, 0x00, 0xff],
            background: [0xff, 0xff, 0xff, 0xff],
            font,
        }
    }
}

pub struct Element<D> {
    pub size: Size,
    pub padding: Padding,
    pub flex: Flex,
    pub style: Style,
    update: Option<UpdateFn<D>>,
    pub content: Content<D>,
    pub scroll: Option<u32>,
}

pub enum Content<D> {
    Text(String, Alignment),
    Paragraph(WrappedText, Alignment),
    Custom { buf: Vec<Pixel>, height: u32 },
    Row(Vec<Element<D>>),
    Stack(Vec<Element<D>>),
}

pub mod builder {
    use super::*;

    pub trait ElementBuilder<D> {
        fn with_update(self, update: UpdateFn<D>) -> Self;
        fn build(self) -> Element<D>;
    }

    enum ContainerKind {
        Stack,
        Row,
    }

    pub struct ContainerBuilder<D> {
        font: Rc<Font>,
        update: Option<UpdateFn<D>>,
        kind: ContainerKind,
        children: Vec<Element<D>>,
    }

    impl<D> ElementBuilder<D> for ContainerBuilder<D> {
        fn with_update(mut self, update: UpdateFn<D>) -> Self {
            self.update = Some(update);
            self
        }

        fn build(self) -> Element<D> {
            let content = match self.kind {
                ContainerKind::Stack => Content::Stack(self.children),
                ContainerKind::Row => Content::Row(self.children),
            };
            Element::new(self.update, content, self.font)
        }
    }

    impl<D> ContainerBuilder<D> {
        fn row(font: &Rc<Font>) -> Self {
            Self {
                font: Rc::clone(font),
                update: None,
                kind: ContainerKind::Row,
                children: Vec::new(),
            }
        }

        fn stack(font: &Rc<Font>) -> Self {
            Self {
                font: Rc::clone(font),
                update: None,
                kind: ContainerKind::Stack,
                children: Vec::new(),
            }
        }

        pub fn add_child(mut self, child: Element<D>) -> Self {
            self.children.push(child);
            self
        }

        pub fn add_children(mut self, children: impl Iterator<Item = Element<D>>) -> Self {
            self.children.extend(children);
            self
        }
    }

    pub struct TextBuilder<D> {
        font: Rc<Font>,
        update: Option<UpdateFn<D>>,
        alignment: Alignment,
        string: String,
    }

    impl<D> ElementBuilder<D> for TextBuilder<D> {
        fn with_update(mut self, update: UpdateFn<D>) -> Self {
            self.update = Some(update);
            self
        }

        fn build(self) -> Element<D> {
            let content = Content::Text(self.string, self.alignment);
            Element::new(self.update, content, self.font)
        }
    }

    impl<D> TextBuilder<D> {
        fn text(font: &Rc<Font>) -> Self {
            Self {
                font: Rc::clone(font),
                update: None,
                alignment: Default::default(),
                string: Default::default(),
            }
        }

        // TODO: I don't feel like thinking about what the proper thing to do here would be. So
        // this is fine for now. Pretty ergonomic in the sense that we don't have to call
        // .to_string() every time. See also the caunterpart for ParagraphBuilder.
        pub fn with_string(mut self, s: &str) -> Self {
            self.string.clear();
            self.string.push_str(s);
            self
        }

        pub fn with_alignment(mut self, alignment: Alignment) -> Self {
            self.alignment = alignment;
            self
        }
    }

    pub struct ParagraphBuilder<D> {
        font: Rc<Font>,
        update: Option<UpdateFn<D>>,
        alignment: Alignment,
        width: Option<u32>,
        string: String,
    }

    impl<D> ElementBuilder<D> for ParagraphBuilder<D> {
        fn with_update(mut self, update: UpdateFn<D>) -> Self {
            self.update = Some(update);
            self
        }

        fn build(self) -> Element<D> {
            let wrapped = WrappedText::new_without_width(self.string, self.width, &self.font);
            let content = Content::Paragraph(wrapped, self.alignment);
            Element::new(self.update, content, self.font)
        }
    }

    impl<D> ParagraphBuilder<D> {
        fn paragraph(font: &Rc<Font>) -> Self {
            Self {
                font: Rc::clone(font),
                update: None,
                alignment: Default::default(),
                width: None,
                string: Default::default(),
            }
        }

        pub fn with_string(mut self, s: &str) -> Self {
            self.string.clear();
            self.string.push_str(s);
            self
        }

        pub fn with_width(mut self, width: u32) -> Self {
            self.width = Some(width);
            self
        }

        pub fn with_alignment(mut self, alignment: Alignment) -> Self {
            self.alignment = alignment;
            self
        }
    }

    impl<D> Element<D> {
        pub fn row_builder(font: &Rc<Font>) -> ContainerBuilder<D> {
            ContainerBuilder::row(font)
        }

        pub fn stack_builder(font: &Rc<Font>) -> ContainerBuilder<D> {
            ContainerBuilder::stack(font)
        }

        pub fn text(s: &str, font: &Rc<Font>) -> TextBuilder<D> {
            TextBuilder::text(font).with_string(s)
        }

        pub fn paragraph(s: &str, font: &Rc<Font>) -> ParagraphBuilder<D> {
            ParagraphBuilder::paragraph(font).with_string(s)
        }

        pub fn empty_text(font: &Rc<Font>) -> TextBuilder<D> {
            TextBuilder::text(font)
        }

        pub fn empty_paragraph(font: &Rc<Font>) -> ParagraphBuilder<D> {
            ParagraphBuilder::paragraph(font)
        }
    }
}

impl<D> Element<D> {
    /// Creates a new [`Element<D>`].
    pub fn new(update: Option<UpdateFn<D>>, content: Content<D>, font: Rc<Font>) -> Self {
        Self {
            size: Default::default(),
            padding: Default::default(),
            flex: Default::default(),
            style: Style::default_with_font(font),
            update,
            content,
            scroll: Default::default(),
        }
    }

    /// Creates a new [`Element<D>`] without an `update` function.
    ///
    /// This [`Element`] will itself remain unchanged. Its children can of course have `update`
    /// functions of their own, but these cannot affect this `Element`s internals beyond
    /// themselves.
    pub fn still(font: Rc<Font>, content: Content<D>) -> Self {
        Self::new(None, content, font)
    }

    /// Creates a new [`Element<D>`] with an `update` function.
    ///
    /// The `update` function allows this [`Element`] to mutate its properties such as `size`,
    /// `padding`, and `content` based on `data` (`&D`) when
    /// [`Panel::update`](crate::Panel::update) is called.
    pub fn dynamic(update: UpdateFn<D>, font: Rc<Font>, content: Content<D>) -> Self {
        Self::new(Some(update), content, font)
    }

    /* min size */
    pub fn with_minwidth(mut self, minwidth: u32) -> Self {
        self.size.minwidth = Some(minwidth);
        self
    }

    pub fn with_minheight(mut self, minheight: u32) -> Self {
        self.size.minheight = Some(minheight);
        self
    }

    /* max size */
    pub fn with_maxwidth(mut self, maxwidth: u32) -> Self {
        self.size.maxwidth = Some(maxwidth);
        self
    }

    pub fn with_maxheight(mut self, maxheight: u32) -> Self {
        self.size.maxheight = Some(maxheight);
        self
    }

    /* fixed size */
    pub fn with_fixedwidth(mut self, width: u32) -> Self {
        self.size.minwidth = Some(width);
        self.size.maxwidth = Some(width);
        self
    }

    pub fn with_fixedheight(mut self, height: u32) -> Self {
        self.size.minheight = Some(height);
        self.size.maxheight = Some(height);
        self
    }

    /* strategy */
    pub fn with_strategy(mut self, strategy: SizingStrategy) -> Self {
        self.size.strategy = strategy;
        self
    }

    /* padding */
    pub fn with_padding_top(mut self, padding: Pad) -> Self {
        self.padding.top = padding;
        self
    }

    pub fn with_padding_bottom(mut self, padding: Pad) -> Self {
        self.padding.bottom = padding;
        self
    }

    pub fn with_padding_left(mut self, padding: Pad) -> Self {
        self.padding.left = padding;
        self
    }

    pub fn with_padding_right(mut self, padding: Pad) -> Self {
        self.padding.right = padding;
        self
    }

    /* flex */
    pub fn with_flex_top(mut self, flex: bool) -> Self {
        self.flex.top = flex;
        self
    }

    pub fn with_flex_bottom(mut self, flex: bool) -> Self {
        self.flex.bottom = flex;
        self
    }

    pub fn with_flex_left(mut self, flex: bool) -> Self {
        self.flex.left = flex;
        self
    }

    pub fn with_flex_right(mut self, flex: bool) -> Self {
        self.flex.right = flex;
        self
    }

    /* style */
    pub fn with_style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }

    pub fn with_foreground(mut self, foreground: Pixel) -> Self {
        self.style.foreground = foreground;
        self
    }

    pub fn with_background(mut self, background: Pixel) -> Self {
        self.style.background = background;
        self
    }

    pub fn with_scroll(mut self, scroll: u32) -> Self {
        self.scroll = Some(scroll);
        self
    }
}

impl<D> Element<D> {
    /// Configure the internal size information based on the [`Element`]'s content and constraints.
    ///
    /// For container [`Content`]s, this method is called recursively.
    ///
    /// After calling this method, the `Element`'s width and height as accessed through
    /// [`Element::fill_size`] or [`Element::overall_size`] are reliably defined and are valid
    /// until the content of it or any of its children are mutated.
    ///
    /// In case of an `Element` with content [`Content::Paragraph`], the inner text is wrapped to
    /// the `maxwidth`, and the `width` and `height` are subsequently calculated based on these
    /// wrapped lines.
    pub(crate) fn bake_size(&mut self, maxwidth_hint: Option<u32>) {
        {
            let Size {
                minwidth,
                maxwidth,
                minheight,
                maxheight,
                ..
            } = self.size();
            if maxwidth.is_some() && minwidth.is_some() {
                assert!(
                    maxwidth >= minwidth,
                    "minwidth ({minwidth:?}) cannot be greater than maxwidth ({maxwidth:?})"
                );
            }
            if maxheight.is_some() && minheight.is_some() {
                assert!(
                    maxheight >= minheight,
                    "minheight ({minheight:?}) cannot be greater than maxheight ({maxheight:?})"
                );
            }
        }
        let width;
        let height;
        match &mut self.content {
            Content::Text(text, _) => {
                width = self.style.font.determine_width(text) as u32;
                height = self.style.font.height() as u32;
            }
            Content::Paragraph(wrapped, _) => {
                wrapped.rewrap(self.size.maxwidth.or(maxwidth_hint), &self.style.font);
                width = wrapped
                    .lines()
                    .map(|line| self.style.font.determine_width(line) as u32)
                    .max()
                    .unwrap_or_default();
                height = (self.style.font.height() * wrapped.lines_count()) as u32;
            }
            Content::Custom { buf, height: h } => {
                width = buf.len() as u32 / *h;
                height = *h;
            }
            Content::Row(children) | Content::Stack(children) => {
                // TODO: See whether this collect alloc can be eliminated. Perhaps unzip?
                let sizes: Vec<_> = children
                    .iter_mut()
                    .map(|child| {
                        child.bake_size(self.size.maxwidth);
                        child.overall_size()
                    })
                    .collect();
                let widths = sizes.iter().map(|size| size.width);
                let heights = sizes.iter().map(|size| size.height);
                match self.content {
                    Content::Row(_) => {
                        width = widths.sum();
                        height = heights.max().unwrap_or_default();
                    }
                    Content::Stack(_) => {
                        height = heights.sum();
                        width = widths.max().unwrap_or_default();
                    }
                    _ => unreachable!(),
                }
            }
        }

        match self.size.strategy {
            SizingStrategy::Whatever => {
                self.size.baked_width = width;
                self.size.baked_height = height;
            }
            SizingStrategy::Chonker => {
                self.size.baked_width = self.size.maxwidth.unwrap_or_default().max(width);
                self.size.baked_height = self.size.maxheight.unwrap_or_default().max(height);
            }
            SizingStrategy::Smollest => {
                // Ah okay wow just realized what the problem is while I was doing something else.
                // Wait no I was wrong.
                self.size.baked_width = self.size.minwidth.unwrap_or(width).min(width);
                self.size.baked_height = self.size.minheight.unwrap_or(height).min(height);
            }
        }
    }

    pub(crate) fn update(&mut self, data: &D) {
        if let Some(update) = self.update {
            update(self, data)
        }

        // TODO: (easy) Break out the 'is it a collection type' logic to a
        // `Content::is_collection(&self) -> bool`.
        match &mut self.content {
            // Update the chirren.
            Content::Row(elements) | Content::Stack(elements) => {
                elements.iter_mut().for_each(|element| element.update(data))
            }
            _ => {}
        };
    }
}

impl<D> Element<D> {
    /// Returns the size of this [`Element<D>`].
    pub fn size(&self) -> Size {
        self.size
    }

    /// Returns the fill size of this [`Element<D>`].
    ///
    /// These [`Dimensions`] _exclude_ the padding and only report the inner size of the `Element`.
    pub fn fill_size(&self) -> Dimensions {
        if let Content::Custom { buf, height } = &self.content {
            let width = buf.len() as u32 / height;
            return Dimensions {
                width,
                height: *height,
            };
        }
        let size = self.size();
        let width = match (size.minwidth, size.maxwidth) {
            (None, None) => size.baked_width,
            (None, Some(maxwidth)) => size.baked_width.min(maxwidth),
            (Some(minwidth), None) => size.baked_width.max(minwidth),
            (Some(minwidth), Some(maxwidth)) => size.baked_width.clamp(minwidth, maxwidth),
        };
        let height = match (size.minheight, size.maxheight) {
            (None, None) => size.baked_height,
            (None, Some(maxheight)) => size.baked_height.min(maxheight),
            (Some(minheight), None) => size.baked_height.max(minheight),
            (Some(minheight), Some(maxheight)) => size.baked_height.clamp(minheight, maxheight),
        };
        Dimensions { width, height }
    }

    /* without padding */
    /// Returns the smallest possible fill size of this [`Element<D>`].
    ///
    /// These [`Dimensions`] _exclude_ the padding and only report the inner size of the `Element`.
    pub fn min_fill_size(&self) -> Dimensions {
        let size = self.size();
        let width = match (size.minwidth, size.maxwidth) {
            (None, None) => size.baked_width,
            (None, Some(maxwidth)) => size.baked_width.min(maxwidth),
            (Some(minwidth), _) => minwidth,
        };
        let height = match (size.minheight, size.maxheight) {
            (None, None) => size.baked_height,
            (None, Some(maxheight)) => size.baked_height.min(maxheight),
            (Some(minheight), _) => minheight,
        };
        Dimensions { width, height }
    }

    /// Returns the largest possible fill size of this [`Element<D>`].
    ///
    /// These [`Dimensions`] _exclude_ the padding and only report the inner size of the `Element`.
    pub fn max_fill_size(&self) -> Dimensions {
        let size = self.size();
        let width = match (size.minwidth, size.maxwidth) {
            (None, None) => size.baked_width,
            (Some(minwidth), None) => size.baked_width.max(minwidth),
            (_, Some(maxwidth)) => maxwidth,
        };
        let height = match (size.minheight, size.maxheight) {
            (None, None) => size.baked_height,
            (Some(minheight), None) => size.baked_height.max(minheight),
            (_, Some(maxheight)) => maxheight,
        };
        Dimensions { width, height }
    }

    /* with padding */
    fn include_padding(mut fillsize: Dimensions, padding: Padding) -> Dimensions {
        fillsize.width += padding.left + padding.right;
        fillsize.height += padding.top + padding.bottom;
        fillsize
    }

    /// Returns the overall size of this [`Element<D>`].
    ///
    /// These [`Dimensions`] _include_ the padding.
    pub fn overall_size(&self) -> Dimensions {
        Self::include_padding(self.fill_size(), self.padding)
    }

    /// Returns a tuple with the horizontal then vertical room per flex for `children`.
    ///
    /// # Panics
    ///
    /// Can only be called on an element with a [`Content::Row`] or [`Content::Stack`]. If it is
    /// called on an [`Element`] with a different content kind, the function will panic.
    fn room_per_flex(&self, children: &[Element<D>]) -> (u32, u32) {
        let children_sizes = children.iter().map(|child| child.overall_size());
        let (children_width, children_height) = match self.content {
            Content::Row(_) => (
                children_sizes.clone().map(|size| size.width).sum(),
                children_sizes
                    .map(|size| size.height)
                    .max()
                    .unwrap_or_default(),
            ),
            Content::Stack(_) => (
                children_sizes
                    .clone()
                    .map(|size| size.width)
                    .max()
                    .unwrap_or_default(),
                children_sizes.map(|size| size.height).sum(),
            ),
            _ => unimplemented!(
                "this function can only produce meaningful results for Row and Stack Content"
            ),
        };
        let flex_room_hor = self.overall_size().width.saturating_sub(children_width);
        let flex_room_ver = self.overall_size().height.saturating_sub(children_height);
        let flexes = children.iter().map(|child| child.flex);
        let flexes_hor: u32 = flexes
            .clone()
            .map(|flex| flex.horizontal_flexes() as u32)
            .sum();
        let flexes_ver: u32 = flexes.map(|flex| flex.vertical_flexes() as u32).sum();
        (
            flex_room_hor.checked_div(flexes_hor).unwrap_or_default(),
            flex_room_ver.checked_div(flexes_ver).unwrap_or_default(),
        )
    }
}

impl<D> DrawBlock for Element<D> {
    fn block(&self) -> Block {
        let Dimensions { width, height } = self.fill_size();
        let mut inner_block = Block::new(width, height, self.style.background);
        match &self.content {
            Content::Text(text, alignment) => draw_text(
                &mut inner_block,
                text,
                *alignment,
                &self.style.font,
                self.style.foreground,
                self.style.background,
            ),
            Content::Paragraph(wrapped, alignment) => {
                let mut y = 0;
                for line in wrapped.lines() {
                    let mut line_block = Block::new(
                        width,
                        self.style.font.height() as u32,
                        self.style.background,
                    );
                    draw_text(
                        &mut line_block,
                        line,
                        *alignment,
                        &self.style.font,
                        self.style.foreground,
                        self.style.background,
                    );
                    inner_block.paint(&line_block, 0, y);
                    y += line_block.height;

                    if let Some(maxheight) = self.size.maxheight {
                        if y > maxheight {
                            break;
                        }
                    }
                }
            }
            Content::Custom { buf, height: h } => {
                assert_eq!(buf.len() as u32 / h, width); // FIXME: Is this redundant?
                assert_eq!(*h, height);
                inner_block.buf.copy_from_slice(buf);
            }
            Content::Row(children) => {
                let (room_per_flex_hor, room_per_flex_ver) = self.room_per_flex(children);
                let mut x = 0;
                for child in children {
                    if child.flex.left {
                        x += room_per_flex_hor
                    }
                    inner_block.paint(&child.block(), x, child.flex.top as u32 * room_per_flex_ver);
                    if child.flex.right {
                        x += room_per_flex_hor
                    }
                    x += child.overall_size().width;
                }
            }
            Content::Stack(children) => {
                let (room_per_flex_hor, room_per_flex_ver) = self.room_per_flex(children);

                let children_height: u32 = children
                    .iter()
                    .map(|child| child.overall_size().height)
                    .sum::<u32>();
                let scroll_block_height = u32::max(children_height, inner_block.height);
                let mut scroll_block = Block::new(
                    self.overall_size().width,
                    scroll_block_height,
                    self.style.background,
                );

                let mut y = 0;
                for child in children {
                    if child.flex.top {
                        y += room_per_flex_ver
                    }

                    scroll_block.paint(
                        &child.block(),
                        child.flex.left as u32 * room_per_flex_hor,
                        y,
                    );
                    if child.flex.bottom {
                        y += room_per_flex_ver
                    }
                    y += child.overall_size().height;
                }

                let scroll_index: usize = self.scroll.unwrap_or(0) as usize;
                let start_index: usize = scroll_index * inner_block.width as usize;

                if let Some(scrolled_block) = scroll_block.buf.get(start_index..) {
                    // TODO: This may not be a very efficient way to set the inner_block buf.
                    inner_block.buf = scrolled_block.to_vec();
                }
            }
        }

        let Dimensions { width, height } = self.overall_size();
        let mut padded_block = Block::new(width, height, self.style.background);
        padded_block.paint(&inner_block, self.padding.left, self.padding.top);
        padded_block
    }
}

#[derive(Default, Clone, Copy)]
pub enum Alignment {
    #[default]
    Left,
    Center,
    Right,
}

#[inline(always)]
fn draw_text(
    block: &mut Block,
    text: &str,
    alignment: Alignment,
    font: &Font,
    foreground: Pixel,
    background: Pixel,
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
    // TODO: Perhaps this case can be handled a little more gracefully, but that will
    // require a more holistic view of the whole layout "engine" in a later stage.
    // Postponing ;)
    if block.width == 0 || scrap_width == 0 {
        return; // Nothing to even draw, here. Why expend the energy?
    }
    let mut scrap = Block::new(scrap_width as u32, font.height() as u32, background);
    let glyphs = text.chars().flat_map(|ch| font.glyph(ch));
    let mut x0 = 0;
    for glyph in glyphs {
        let glyph_width = glyph.width as usize;
        for (y, row) in glyph.enumerate() {
            for (xg, cell) in row.enumerate() {
                let x = x0 + xg;
                // TODO: This may be more efficient than what I did in Graph. May be
                // worth investigating which is better.
                scrap.buf[y * scrap.width as usize + x] =
                    if cell { foreground } else { background };
            }
        }
        x0 += glyph_width;
    }

    match alignment {
        Alignment::Left => {
            let end = block.width.min(scrap.width) as usize;
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
            let end = block.width.min(scrap.width) as usize;
            block
                .rows_mut()
                .zip(scrap.rows())
                .for_each(|(row, scrap_row)| {
                    row[..end].copy_from_slice(&scrap_row[..end]);
                })
        }
        Alignment::Center => {
            let remainder = block.width - scrap.width;
            let start = remainder as usize / 2;
            block
                .rows_mut()
                .zip(scrap.rows())
                .for_each(|(row, scrap_row)| {
                    row[start..start + scrap.width as usize].copy_from_slice(scrap_row);
                })
        }
        Alignment::Right => {
            let rstart = block.width.saturating_sub(scrap.width) as usize;
            let sstart = scrap.width.saturating_sub(block.width) as usize;
            block
                .rows_mut()
                .zip(scrap.rows())
                .for_each(|(row, scrap_row)| {
                    row[rstart..].copy_from_slice(&scrap_row[sstart..]);
                })
        }
    }
}
