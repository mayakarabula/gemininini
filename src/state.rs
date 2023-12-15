use pixels::Pixels;
use std::collections::VecDeque;

use crate::config::{Pixel, PIXEL_SIZE};
use crate::font::{Font, WrappedFont};

pub const NORMAL_MODE: &str = "normal";
pub const INSERT_MODE: &str = "insert";
pub const LINK_MODE: &str = "link";

pub struct State {
    pub font: WrappedFont,
    pub foreground: Pixel,
    pub background: Pixel,
    pub page_address: String,
    pub page_content: String,
    pub window_width: u32,
    pub window_height: u32,
    pub content_lines: Vec<String>,
    pub starting_line: usize,
    pub mode: String,
}

struct Block {
    height: usize,
    pixels: Vec<Pixel>,
}

impl Block {
    fn width(&self) -> usize {
        self.pixels.len() / self.height
    }

    fn rows(&self) -> Option<std::slice::ChunksExact<'_, Pixel>> {
        if self.width() == 0 {
            return None;
        }

        Some(self.pixels.chunks_exact(self.width()))
    }

    fn draw_onto_pixels(self, pixels: &mut Pixels, start_y: usize) {
        let size = pixels.texture().size();
        if let Some(rows) = self.rows() {
            for (y, row) in rows.enumerate() {
                let idx: usize = ((start_y + y) * size.width as usize) * PIXEL_SIZE;
                let row_bytes = row.flatten();
                pixels.frame_mut()[idx..idx + row_bytes.len()].copy_from_slice(row_bytes);
            }
        }
    }
}

trait Draw {
    fn draw(&self, state: &State, foreground: Pixel, background: Pixel) -> Block;
    fn draw_default(&self, state: &State) -> Block;
    fn width(&self, state: &State) -> usize;
}

impl Draw for &str {
    fn draw_default(&self, state: &State) -> Block {
        self.draw(state, state.foreground, state.background)
    }

    fn width(&self, state: &State) -> usize {
        let glyphs = self.chars().flat_map(|ch| state.font.glyph(ch));
        glyphs.clone().map(|g| g.width()).sum()
    }

    fn draw(&self, state: &State, foreground: Pixel, background: Pixel) -> Block {
        let height = state.font.height();
        let glyphs = self.chars().flat_map(|ch| state.font.glyph(ch));
        let width: usize = glyphs.clone().map(|g| g.width()).sum();
        let mut pixels = vec![state.background; height * width];
        let mut x0 = 0;

        for g in glyphs {
            for (y, row) in g.rows().enumerate() {
                for (xg, &cell) in row.iter().enumerate() {
                    let x = x0 + xg;
                    let idx = y * width + x;
                    pixels[idx] = if cell { foreground } else { background };
                }
            }
            x0 += g.width();
        }

        Block { height, pixels }
    }
}

impl State {
    pub fn new(
        font: WrappedFont,
        foreground: Pixel,
        background: Pixel,
        page_address: String,
        page_content: String,
        window_width: u32,
        window_height: u32,
    ) -> Self {
        Self {
            font,
            foreground,
            background,
            page_address,
            page_content,
            window_width,
            window_height,
            content_lines: Vec::new(),
            starting_line: 0,
            mode: String::from("normal"),
        }
    }

    pub fn prepare_lines(&mut self) {
        let lines: Vec<String> = self.page_content.lines().map(|s| s.to_string()).collect();
        let mut deque: VecDeque<String> = VecDeque::from(lines);
        self.content_lines.clear();

        while let Some(line) = deque.pop_front() {
            let mut index = 0;
            let mut s = String::new();
            let words: Vec<&str> = line.split_whitespace().collect();

            while index < words.len() && s.as_str().width(&self) < self.window_width as usize {
                s.push_str(words[index]);
                s.push(' ');

                index += 1;
            }

            if words.len() > index {
                s = words.get(0..index - 1).unwrap().join(" ");
                deque.push_front(words.get(index - 1..).unwrap().join(" "));
            } else {
                s = words.get(0..index).unwrap().join(" ");
            }

            self.content_lines.push(s);
        }
    }

    pub fn resize(&mut self, window_width: u32, window_height: u32) {
        self.window_width = window_width;
        self.window_height = window_height;
    }

    pub fn update(&mut self, page_address: String, page_content: String) {
        self.page_address = page_address;
        self.page_content = page_content;
    }

    pub fn set_starting_line(&mut self, starting_line: usize) {
        self.starting_line = starting_line;
    }

    pub fn set_mode(&mut self, mode: String) {
        self.mode = mode;
    }

    pub fn draw(&self, pixels: &mut Pixels) {
        let mut start_y = 0;
        let font_height: usize = self.font.height();

        let block =
            self.page_address
                .to_string()
                .as_str()
                .draw(&self, self.background, self.foreground);
        block.draw_onto_pixels(pixels, start_y);
        start_y += font_height; // Move to the next vertical position

        let mut index = 0;
        let lines = self.content_lines.get(self.starting_line..).unwrap();
        let mut link_index = 0;

        while let Some(line) = lines.get(index) {
            index += 1;

            if line.len() == 0 {
                start_y += font_height;
                continue;
            }

            let s = line.clone();

            let block = s.to_string().as_str().draw_default(&self);
            block.draw_onto_pixels(pixels, start_y);

            if line.starts_with("=>") {
                if self.mode == LINK_MODE {
                    let block = link_index.to_string().as_str().draw(&self, self.background, self.foreground);
                    block.draw_onto_pixels(pixels, start_y);
                }
                link_index += 1;
            }

            start_y += font_height;

            if start_y + 3 * self.font.height() > self.window_height as usize {
                break;
            }
        }

        let block =
            self.mode
                .to_string()
                .as_str()
                .draw(&self, self.background, self.foreground);
        block.draw_onto_pixels(pixels, self.window_height as usize - font_height);
    }

    pub(crate) fn set_address(&mut self, address: String) {
        self.page_address = address
    }
}

