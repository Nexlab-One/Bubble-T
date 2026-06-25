//! Median-cut palette quantization for Sixel encoding.

use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap};

use image::RgbaImage;

use crate::sixel::color::SixelColor;

/// Maximum palette size for Sixel images.
pub const MAX_COLORS: usize = 256;

/// Quantized Sixel palette with O(1) color lookup.
#[derive(Debug, Clone)]
pub struct SixelPalette {
    pub(crate) palette_colors: Vec<SixelColor>,
    palette_indexes: HashMap<SixelColor, usize>,
}

impl SixelPalette {
    /// Returns the palette index for an image color.
    #[must_use]
    pub(crate) fn color_index(&self, c: SixelColor) -> usize {
        self.palette_indexes.get(&c).copied().unwrap_or(0)
    }

    /// Palette entries in index order.
    #[must_use]
    pub(crate) fn colors(&self) -> &[SixelColor] {
        &self.palette_colors
    }
}

/// Builds a median-cut palette from `img` with at most `max_colors` entries.
#[must_use]
pub fn new_palette(img: &RgbaImage, max_colors: usize) -> SixelPalette {
    let mut pixel_counts: HashMap<SixelColor, u64> = HashMap::new();
    for (_, _, px) in img.enumerate_pixels() {
        let c = SixelColor::from_rgba8(px[0], px[1], px[2], px[3]);
        *pixel_counts.entry(c).or_insert(0) += 1;
    }

    let mut unique_colors: Vec<SixelColor> = pixel_counts.keys().copied().collect();
    let max_colors = max_colors.min(MAX_COLORS);

    let mut palette = SixelPalette {
        palette_colors: Vec::new(),
        palette_indexes: HashMap::new(),
    };

    if unique_colors.len() <= max_colors {
        palette.palette_colors = unique_colors.clone();
    } else {
        palette.quantize(&mut unique_colors, &pixel_counts, max_colors);
    }

    for c in unique_colors {
        let (best_index, _) = palette
            .palette_colors
            .iter()
            .enumerate()
            .map(|(i, p)| (i, color_distance(c, *p)))
            .min_by_key(|(_, d)| *d)
            .unwrap_or((0, 0));
        palette.palette_indexes.insert(c, best_index);
    }

    palette
}

#[derive(Debug, Clone)]
struct QuantCube {
    start: usize,
    len: usize,
    slice_channel: Channel,
    score: u64,
    pixel_count: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Channel {
    Red,
    Green,
    Blue,
    Alpha,
}

impl SixelPalette {
    fn quantize(
        &mut self,
        unique_colors: &mut [SixelColor],
        pixel_counts: &HashMap<SixelColor, u64>,
        max_colors: usize,
    ) {
        let mut heap = BinaryHeap::new();
        heap.push(CubeOrder(self.create_cube(
            unique_colors,
            pixel_counts,
            0,
            unique_colors.len(),
        )));

        while heap.len() < max_colors {
            let Some(CubeOrder(cube)) = heap.pop() else {
                break;
            };
            if cube.len <= 1 {
                heap.push(CubeOrder(cube));
                break;
            }

            unique_colors[cube.start..cube.start + cube.len]
                .sort_by(|a, b| cmp_channel(*a, *b, cube.slice_channel));

            let mut count_so_far = pixel_counts[&unique_colors[cube.start]];
            let target = cube.pixel_count / 2;
            let mut left_len = 1usize;
            for i in cube.start + 1..cube.start + cube.len {
                let weight = pixel_counts[&unique_colors[i]];
                if count_so_far + weight > target {
                    break;
                }
                left_len += 1;
                count_so_far += weight;
            }
            let right_len = cube.len - left_len;
            let right_index = cube.start + left_len;

            heap.push(CubeOrder(self.create_cube(
                unique_colors,
                pixel_counts,
                cube.start,
                left_len,
            )));
            heap.push(CubeOrder(self.create_cube(
                unique_colors,
                pixel_counts,
                right_index,
                right_len,
            )));
        }

        while let Some(CubeOrder(cube)) = heap.pop() {
            self.load_color(unique_colors, pixel_counts, cube.start, cube.len);
        }
    }

    fn create_cube(
        &self,
        unique_colors: &[SixelColor],
        pixel_counts: &HashMap<SixelColor, u64>,
        start: usize,
        len: usize,
    ) -> QuantCube {
        let mut min = [u32::MAX; 4];
        let mut max = [0u32; 4];
        let mut total_weight = 0u64;

        for c in &unique_colors[start..start + len] {
            let px = [c.red, c.green, c.blue, c.alpha];
            total_weight += pixel_counts[c];
            for (i, v) in px.into_iter().enumerate() {
                min[i] = min[i].min(v);
                max[i] = max[i].max(v);
            }
        }

        let d = [
            max[0] - min[0],
            max[1] - min[1],
            max[2] - min[2],
            max[3] - min[3],
        ];
        let slice_channel = if d[0] >= d[1] && d[0] >= d[2] && d[0] >= d[3] {
            Channel::Red
        } else if d[1] >= d[2] && d[1] >= d[3] {
            Channel::Green
        } else if d[2] >= d[3] {
            Channel::Blue
        } else {
            Channel::Alpha
        };
        let score = u64::from(match slice_channel {
            Channel::Red => d[0],
            Channel::Green => d[1],
            Channel::Blue => d[2],
            Channel::Alpha => d[3],
        }) * total_weight;

        QuantCube {
            start,
            len,
            slice_channel,
            score,
            pixel_count: total_weight,
        }
    }

    fn load_color(
        &mut self,
        unique_colors: &[SixelColor],
        pixel_counts: &HashMap<SixelColor, u64>,
        start: usize,
        len: usize,
    ) {
        let mut totals = [0u64; 4];
        let mut count = 0u64;
        for c in &unique_colors[start..start + len] {
            let w = pixel_counts[c];
            totals[0] += u64::from(c.red) * w;
            totals[1] += u64::from(c.green) * w;
            totals[2] += u64::from(c.blue) * w;
            totals[3] += u64::from(c.alpha) * w;
            count += w;
        }
        if count == 0 {
            return;
        }
        self.palette_colors.push(SixelColor {
            red: (totals[0] / count) as u32,
            green: (totals[1] / count) as u32,
            blue: (totals[2] / count) as u32,
            alpha: (totals[3] / count) as u32,
        });
    }
}

struct CubeOrder(QuantCube);

impl PartialEq for CubeOrder {
    fn eq(&self, other: &Self) -> bool {
        self.0.score == other.0.score
    }
}

impl Eq for CubeOrder {}

impl PartialOrd for CubeOrder {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for CubeOrder {
    fn cmp(&self, other: &Self) -> Ordering {
        self.0.score.cmp(&other.0.score)
    }
}

fn cmp_channel(a: SixelColor, b: SixelColor, ch: Channel) -> Ordering {
    match ch {
        Channel::Red => a.red.cmp(&b.red),
        Channel::Green => a.green.cmp(&b.green),
        Channel::Blue => a.blue.cmp(&b.blue),
        Channel::Alpha => a.alpha.cmp(&b.alpha),
    }
}

fn color_distance(a: SixelColor, b: SixelColor) -> u32 {
    let dr = a.red as i64 - b.red as i64;
    let dg = a.green as i64 - b.green as i64;
    let db = a.blue as i64 - b.blue as i64;
    let da = a.alpha as i64 - b.alpha as i64;
    (dr * dr + dg * dg + db * db + da * da) as u32
}
