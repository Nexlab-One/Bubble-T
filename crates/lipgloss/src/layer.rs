//! Compositing layers (Lip Gloss v2).

use crate::size::{height, width};

/// A positioned visual layer with optional child layers.
#[derive(Debug, Clone)]
pub struct Layer {
    id: String,
    content: String,
    width: i32,
    height: i32,
    x: i32,
    y: i32,
    z: i32,
    layers: Vec<Layer>,
}

/// Result of a compositor hit test.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LayerHit {
    id: String,
    bounds: (i32, i32, i32, i32),
}

impl LayerHit {
    /// Returns whether the hit is empty.
    pub fn is_empty(&self) -> bool {
        self.id.is_empty()
    }

    /// Returns the hit layer id.
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Returns `(x, y, width, height)` bounds of the hit layer.
    pub fn bounds(&self) -> (i32, i32, i32, i32) {
        self.bounds
    }
}

impl Layer {
    /// Creates a layer with `content` and optional children.
    pub fn new(content: impl Into<String>, layers: Vec<Layer>) -> Self {
        let mut layer = Self {
            id: String::new(),
            content: content.into(),
            width: 0,
            height: 0,
            x: 0,
            y: 0,
            z: 0,
            layers: Vec::new(),
        };
        layer.add_layers(layers);
        layer
    }

    /// Returns the layer content.
    pub fn content(&self) -> &str {
        &self.content
    }

    /// Sets the layer id used for hit testing.
    pub fn id(mut self, id: impl Into<String>) -> Self {
        self.id = id.into();
        self
    }

    /// Sets the x offset relative to the parent.
    pub fn x(mut self, x: i32) -> Self {
        self.x = x;
        self
    }

    /// Sets the y offset relative to the parent.
    pub fn y(mut self, y: i32) -> Self {
        self.y = y;
        self
    }

    /// Sets the z-index relative to siblings.
    pub fn z(mut self, z: i32) -> Self {
        self.z = z;
        self
    }

    /// Adds child layers and recomputes bounds.
    pub fn add_layers(&mut self, layers: Vec<Layer>) -> &mut Self {
        self.layers.extend(layers);
        let (w, h) = self.measure_bounds(0, 0);
        self.width = w;
        self.height = h;
        self
    }

    fn measure_bounds(&self, parent_x: i32, parent_y: i32) -> (i32, i32) {
        let abs_x = self.x + parent_x;
        let abs_y = self.y + parent_y;
        let mut max_x = abs_x + width(&self.content) as i32;
        let mut max_y = abs_y + height(&self.content) as i32;

        for child in &self.layers {
            let (cw, ch) = child.measure_bounds(abs_x, abs_y);
            max_x = max_x.max(cw);
            max_y = max_y.max(ch);
        }

        (max_x, max_y)
    }
}

/// Flattens a layer tree for rendering and hit testing.
#[derive(Debug, Clone)]
pub struct Compositor {
    root: Layer,
    flat: Vec<FlatLayer>,
    bounds: (i32, i32, i32, i32),
}

#[derive(Debug, Clone)]
pub(crate) struct FlatLayer {
    pub(crate) content: String,
    pub(crate) abs_x: i32,
    pub(crate) abs_y: i32,
    z: i32,
    width: i32,
    height: i32,
    id: String,
}

impl Compositor {
    /// Creates a compositor with optional root children.
    pub fn new(layers: Vec<Layer>) -> Self {
        let mut root = Layer::new(String::new(), Vec::new());
        root.add_layers(layers);
        let mut c = Self {
            root,
            flat: Vec::new(),
            bounds: (0, 0, 0, 0),
        };
        c.flatten();
        c
    }

    /// Adds layers to the compositor root and refreshes state.
    pub fn add_layers(&mut self, layers: Vec<Layer>) -> &mut Self {
        self.root.add_layers(layers);
        self.flatten();
        self
    }

    /// Re-flattens after manual layer tree edits.
    pub fn refresh(&mut self) {
        self.flatten();
    }

    fn flatten(&mut self) {
        self.flat.clear();
        let root = self.root.clone();
        self.flatten_recursive(&root, 0, 0);
        self.flat.sort_by_key(|l| l.z);

        if let Some(first) = self.flat.first() {
            let mut max_x = first.abs_x + first.width;
            let mut max_y = first.abs_y + first.height;
            for layer in &self.flat[1..] {
                max_x = max_x.max(layer.abs_x + layer.width);
                max_y = max_y.max(layer.abs_y + layer.height);
            }
            self.bounds = (0, 0, max_x, max_y);
        } else {
            self.bounds = (0, 0, 0, 0);
        }
    }

    fn flatten_recursive(&mut self, layer: &Layer, parent_x: i32, parent_y: i32) {
        let abs_x = layer.x + parent_x;
        let abs_y = layer.y + parent_y;
        let w = width(&layer.content) as i32;
        let h = height(&layer.content) as i32;

        self.flat.push(FlatLayer {
            id: layer.id.clone(),
            content: layer.content.clone(),
            abs_x,
            abs_y,
            z: layer.z,
            width: w,
            height: h,
        });

        for child in &layer.layers {
            self.flatten_recursive(child, abs_x, abs_y);
        }
    }

    /// Returns overall bounds `(x, y, width, height)`.
    pub fn bounds(&self) -> (i32, i32, i32, i32) {
        self.bounds
    }

    /// Hit-tests `(x, y)` returning the top-most layer with a non-empty id.
    pub fn hit(&self, x: i32, y: i32) -> LayerHit {
        for layer in self.flat.iter().rev() {
            if layer.id.is_empty() {
                continue;
            }
            if x >= layer.abs_x
                && y >= layer.abs_y
                && x < layer.abs_x + layer.width
                && y < layer.abs_y + layer.height
            {
                return LayerHit {
                    id: layer.id.clone(),
                    bounds: (layer.abs_x, layer.abs_y, layer.width, layer.height),
                };
            }
        }
        LayerHit {
            id: String::new(),
            bounds: (0, 0, 0, 0),
        }
    }

    /// Renders the compositor to a styled string via a temporary canvas.
    pub fn render(&self) -> String {
        let (_x, _y, w, h) = self.bounds;
        let mut canvas = crate::canvas::Canvas::new(w.max(1) as u16, h.max(1) as u16);
        canvas.compose(self);
        canvas.render()
    }
}

impl Compositor {
    pub(crate) fn layers(&self) -> &[FlatLayer] {
        &self.flat
    }
}
