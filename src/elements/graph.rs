use std::collections::VecDeque;

use crate::{elements::Style, Pixel};

// TODO: (easy) Isn't there a std lib type for this?! I'm pretty sure there is. Just moving on now.
#[derive(Default)]
struct Range {
    min: Option<f32>,
    max: Option<f32>,
}

impl Range {
    fn clamp(&self, value: f32) -> f32 {
        match (self.min, self.max) {
            (None, None) => value,
            (None, Some(max)) => value.min(max),
            (Some(min), None) => value.max(min),
            (Some(min), Some(max)) => value.clamp(min, max),
        }
    }
}

pub struct Graph(VecDeque<f32>, Range);

impl Graph {
    pub fn new(size: usize) -> Self {
        let mut inner = VecDeque::new();
        inner.resize(size, 0.0);
        Self(inner, Range::default())
    }

    pub fn with_min(mut self, min: f32) -> Self {
        self.1.min = Some(min);
        self
    }

    pub fn with_max(mut self, max: f32) -> Self {
        self.1.max = Some(max);
        self
    }

    pub fn with_range(mut self, min: f32, max: f32) -> Self {
        self.1.min = Some(min);
        self.1.max = Some(max);
        self
    }

    pub fn push(&mut self, value: f32) {
        let Self(inner, _) = self;
        let size = inner.len();
        // TODO: (easy) Check whether my assumption about VecDeque allocation behavior is accurate.
        inner.truncate(size.saturating_sub(1)); // Truncate to prepare to prevent allocation.
        inner.push_front(value)
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = f32> + '_ {
        self.0.iter().map(|v| self.1.clamp(*v))
    }

    pub fn inner_mut(&mut self) -> &mut VecDeque<f32> {
        &mut self.0
    }

    pub fn min(&self) -> f32 {
        if self.is_empty() {
            return Default::default();
        }
        if let Some(min) = self.1.min {
            return min;
        }
        self.iter().fold(f32::INFINITY, f32::min)
    }

    pub fn max(&self) -> f32 {
        if self.is_empty() {
            return Default::default();
        }
        if let Some(max) = self.1.max {
            return max;
        }
        self.iter().fold(f32::NEG_INFINITY, f32::max)
    }

    /// Paint the graph onto a pixel buffer with a specified height.
    ///
    /// All values represented within the graph are mapped the pixels, such that `min` point of the
    /// [`Graph`] is drawn on the lowest row of the pixel buffer, and the `max` point on the first
    /// row.
    ///
    /// The width of the pixel buffer is expected to be equal to its length divided by the provided
    /// height. The length of the [`Graph`] must be equal to that width.
    ///
    /// The foreground and background colors are provided through `style`.
    pub fn paint(&self, buf: &mut [Pixel], height: u32, style: &Style) {
        let width = self.len();
        assert_eq!(buf.len(), width * height as usize);

        let min = self.min();
        let max = self.max();
        let delta = max - min;
        let factor = height.saturating_sub(1) as f32 / delta;

        buf.fill(style.background);
        for (x, y) in self.iter().enumerate() {
            let y = (y - min) * factor;
            let y = y.round() as usize;
            let idx = y * width + x;
            buf[idx] = style.foreground;
        }
    }
}

impl From<VecDeque<f32>> for Graph {
    fn from(deque: VecDeque<f32>) -> Self {
        Self(deque, Range::default())
    }
}
