use fleck::Font;

/// A wrapper for a [`String`] where its contents are guaranteed to be wrapped at time of use.
///
/// To iterate over the wrapped lines, use [`WrappedText::lines`]. A [`String`] with baked newlines
/// is returned by [`WrappedText::wrapped`].
///
/// # Note
///
/// No actual wrapping of the internal string takes place at time of [creation](WrappedText::new)
/// or when [rewrapped](WrappedText::rewrap). In fact, the internal string is not mutate over the
/// lifetime of [`WrappedText`].
#[derive(Debug, Default, Clone)]
pub struct WrappedText(String, Vec<usize>);

impl WrappedText {
    /// Creates a new [`WrappedText`] that will be wrapped to the specified `width` and according
    /// to the glyphs in the provided [`Font`].
    pub fn new(text: String, width: u32, font: &Font) -> Self {
        Self::new_without_width(text, Some(width), font)
    }

    // TODO: Consider whether it is worth it to expose this function as `pub`. Will a user ever
    // actually need this, especially with a good builder API for the Element tree?
    // The function does not really do harm but also, it is quite an implementation detail. It will
    // make it more expensive to mess with it at a later time.
    /// Set up a new [`WrappedText`] without wrapping any lines.
    ///
    /// In order to wrap the text to the desired width at a later stage, call
    /// [`WrappedText::rewrap`].
    pub(crate) fn new_without_width(text: String, width: Option<u32>, font: &Font) -> Self {
        let mut ret = Self(text, Vec::new());
        ret.rewrap(width, font);
        ret
    }

    /// Rewrap the [`WrappedText`] to the desired width.
    ///
    /// If `None` is passed as the `maxwidth`, the lines are not wrapped.
    pub fn rewrap(&mut self, maxwidth: Option<u32>, font: &Font) {
        // TODO: Do this optimization that I had this note for:
        // > TODO: I don't know whether this makes any sense. Never measured it. I like it because
        // > it may prevent two allocations but also, who cares.

        // TODO: Equal starts optimization.

        let Self(text, breaklist) = self;
        breaklist.clear();
        let mut scrapwidth = 0u32;
        let mut wordwidth = 0u32;
        // FIXME: There may be a bug with a very long unbroken first line because we set it to 0
        // here. Maybe consider a None here.
        let mut last_whitespace = None;
        for (idx, ch) in text.char_indices() {
            match ch {
                '\n' => {
                    scrapwidth = 0;
                    wordwidth = 0;
                    last_whitespace = None; // FIXME: Or None?
                    breaklist.push(idx)
                }
                ch if maxwidth.is_some() => {
                    if ch.is_whitespace() {
                        last_whitespace = Some(idx);
                        wordwidth = 0;
                    }
                    let glyphwidth = font.glyph(ch).map_or(0, |ch| ch.width) as u32;
                    // TODO: Think about this unwrap().
                    if scrapwidth + glyphwidth > maxwidth.unwrap() {
                        let br = match last_whitespace {
                            Some(br) => br,
                            None => {
                                wordwidth = 0;
                                idx
                            }
                        };
                        breaklist.push(br);
                        wordwidth += glyphwidth;
                        scrapwidth = wordwidth;
                    } else {
                        wordwidth += glyphwidth;
                        scrapwidth += glyphwidth;
                    }
                }
                _ => {}
            }
        }

        breaklist.push(text.len());
    }

    /// Returns an iterator over the lines of this [`WrappedText`].
    pub fn lines(&self) -> impl Iterator<Item = &str> {
        let mut runner = 0;
        self.1.iter().map(move |&breakpoint| {
            let a = &self.0[runner..breakpoint];
            runner = breakpoint;
            if a.chars().next().is_some_and(|ch| ch.is_whitespace()) {
                &a[1..]
            } else {
                a
            }
        })
    }

    /// Returns the number of wrapped lines in this [`WrappedText`].
    pub fn lines_count(&self) -> usize {
        self.1.len()
    }

    /// Return a wrapped [`String`].
    ///
    /// It may be more efficient to use the [`WrappedText::lines`] directly, if that is actually what you need.
    pub fn wrapped(&self) -> String {
        self.lines().intersperse("\n").collect()
    }
}
