//! Kitty graphics protocol options.

use super::protocol::{DELETE_ALL, DIRECT, FILE, RGBA, TRANSMIT, ZLIB};

/// Kitty Graphics Protocol options.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Options {
    /// Action (`a=`).
    pub action: u8,
    /// Quiet mode (`q=`).
    pub quiet: u8,
    /// Image id (`i=`).
    pub id: i32,
    /// Placement id (`p=`).
    pub placement_id: i32,
    /// Image number (`I=`).
    pub number: i32,
    /// Pixel format (`f=`).
    pub format: i32,
    /// Transmitted image width (`s=`).
    pub image_width: i32,
    /// Transmitted image height (`v=`).
    pub image_height: i32,
    /// Compression (`o=`).
    pub compression: u8,
    /// Transmission type (`t=`).
    pub transmission: u8,
    /// File path when using file transmission.
    pub file: String,
    /// Size to read (`S=`).
    pub size: i32,
    /// Read offset (`O=`).
    pub offset: i32,
    /// Chunked transmission (`m=`).
    pub chunk: bool,
    /// Display X offset in pixels (`x=`).
    pub x: i32,
    /// Display Y offset in pixels (`y=`).
    pub y: i32,
    /// Z-index (`z=`).
    pub z: i32,
    /// Display width (`w=`).
    pub width: i32,
    /// Display height (`h=`).
    pub height: i32,
    /// Cell X offset (`X=`).
    pub offset_x: i32,
    /// Cell Y offset (`Y=`).
    pub offset_y: i32,
    /// Column span (`c=`).
    pub columns: i32,
    /// Row span (`r=`).
    pub rows: i32,
    /// Virtual Unicode placement (`U=1`).
    pub virtual_placement: bool,
    /// Do not move cursor after display (`C=1`).
    pub do_not_move_cursor: bool,
    /// Parent image id (`P=`).
    pub parent_id: i32,
    /// Parent placement id (`Q=`).
    pub parent_placement_id: i32,
    /// Delete action (`d=`).
    pub delete: u8,
    /// Delete associated resources (uppercase delete letter).
    pub delete_resources: bool,
}

impl Options {
    /// Serializes options as comma-separated key=value pairs.
    #[must_use]
    pub fn option_strings(&self) -> Vec<String> {
        let mut opts = Vec::new();
        let format = if self.format == 0 { RGBA } else { self.format };
        let action = if self.action == 0 {
            TRANSMIT
        } else {
            self.action
        };
        let mut transmission = self.transmission;
        if transmission == 0 {
            transmission = if self.file.is_empty() { DIRECT } else { FILE };
        }
        let delete = if self.delete == 0 {
            DELETE_ALL
        } else {
            self.delete
        };

        if format != RGBA {
            opts.push(format!("f={format}"));
        }
        if self.quiet > 0 {
            opts.push(format!("q={}", self.quiet));
        }
        if self.id > 0 {
            opts.push(format!("i={}", self.id));
        }
        if self.placement_id > 0 {
            opts.push(format!("p={}", self.placement_id));
        }
        if self.number > 0 {
            opts.push(format!("I={}", self.number));
        }
        if self.image_width > 0 {
            opts.push(format!("s={}", self.image_width));
        }
        if self.image_height > 0 {
            opts.push(format!("v={}", self.image_height));
        }
        if transmission != DIRECT {
            opts.push(format!("t={}", char::from(transmission)));
        }
        if self.size > 0 {
            opts.push(format!("S={}", self.size));
        }
        if self.offset > 0 {
            opts.push(format!("O={}", self.offset));
        }
        if self.compression == ZLIB {
            opts.push(format!("o={}", char::from(self.compression)));
        }
        if self.virtual_placement {
            opts.push("U=1".to_string());
        }
        if self.do_not_move_cursor {
            opts.push("C=1".to_string());
        }
        if self.parent_id > 0 {
            opts.push(format!("P={}", self.parent_id));
        }
        if self.parent_placement_id > 0 {
            opts.push(format!("Q={}", self.parent_placement_id));
        }
        if self.x > 0 {
            opts.push(format!("x={}", self.x));
        }
        if self.y > 0 {
            opts.push(format!("y={}", self.y));
        }
        if self.z > 0 {
            opts.push(format!("z={}", self.z));
        }
        if self.width > 0 {
            opts.push(format!("w={}", self.width));
        }
        if self.height > 0 {
            opts.push(format!("h={}", self.height));
        }
        if self.offset_x > 0 {
            opts.push(format!("X={}", self.offset_x));
        }
        if self.offset_y > 0 {
            opts.push(format!("Y={}", self.offset_y));
        }
        if self.columns > 0 {
            opts.push(format!("c={}", self.columns));
        }
        if self.rows > 0 {
            opts.push(format!("r={}", self.rows));
        }
        if delete != DELETE_ALL || self.delete_resources {
            let mut da = delete;
            if self.delete_resources && da.is_ascii_lowercase() {
                da = da.to_ascii_uppercase();
            }
            opts.push(format!("d={}", char::from(da)));
        }
        if action != TRANSMIT {
            opts.push(format!("a={}", char::from(action)));
        }
        opts
    }

    /// Parses options from comma-separated text (without APC wrapper).
    pub fn parse(text: &str) -> Self {
        let mut o = Self::default();
        for part in text.split(',') {
            let Some((key, value)) = part.split_once('=') else {
                continue;
            };
            if value.is_empty() {
                continue;
            }
            match key {
                "a" => o.action = value.as_bytes()[0],
                "o" => o.compression = value.as_bytes()[0],
                "t" => o.transmission = value.as_bytes()[0],
                "d" => {
                    let mut d = value.as_bytes()[0];
                    if d.is_ascii_uppercase() {
                        o.delete_resources = true;
                        d = d.to_ascii_lowercase();
                    }
                    o.delete = d;
                }
                "i" | "q" | "p" | "I" | "f" | "s" | "v" | "S" | "O" | "m" | "x" | "y" | "z"
                | "w" | "h" | "X" | "Y" | "c" | "r" | "U" | "P" | "Q" => {
                    let Ok(v) = value.parse::<i32>() else {
                        continue;
                    };
                    match key {
                        "i" => o.id = v,
                        "q" => o.quiet = v.clamp(0, 255) as u8,
                        "p" => o.placement_id = v,
                        "I" => o.number = v,
                        "f" => o.format = v,
                        "s" => o.image_width = v,
                        "v" => o.image_height = v,
                        "S" => o.size = v,
                        "O" => o.offset = v,
                        "m" => o.chunk = v == 0 || v == 1,
                        "x" => o.x = v,
                        "y" => o.y = v,
                        "z" => o.z = v,
                        "w" => o.width = v,
                        "h" => o.height = v,
                        "X" => o.offset_x = v,
                        "Y" => o.offset_y = v,
                        "c" => o.columns = v,
                        "r" => o.rows = v,
                        "U" => o.virtual_placement = v == 1,
                        "P" => o.parent_id = v,
                        "Q" => o.parent_placement_id = v,
                        _ => {}
                    }
                }
                _ => {}
            }
        }
        o
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_options() {
        let o = Options {
            id: 31,
            format: super::super::protocol::RGB,
            quiet: 2,
            columns: 10,
            rows: 5,
            virtual_placement: true,
            action: super::super::protocol::TRANSMIT_AND_PUT,
            ..Default::default()
        };
        let s = o.option_strings().join(",");
        let parsed = Options::parse(&s);
        assert_eq!(parsed.id, 31);
        assert_eq!(parsed.format, super::super::protocol::RGB);
        assert!(parsed.virtual_placement);
    }

    #[test]
    fn shared_memory_transmission_option() {
        let o = Options {
            transmission: super::super::protocol::SHARED_MEMORY,
            size: 128,
            ..Default::default()
        };
        let s = o.option_strings().join(",");
        assert!(s.contains("t=s"));
        assert!(s.contains("S=128"));
        let parsed = Options::parse(&s);
        assert_eq!(parsed.transmission, super::super::protocol::SHARED_MEMORY);
        assert_eq!(parsed.size, 128);
    }
}
