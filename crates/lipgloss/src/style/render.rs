//! Text rendering with ANSI escape sequence generation.
//!
//! This module provides the core rendering functionality that converts styled text
//! into terminal-ready output with appropriate ANSI escape sequences for colors,
//! attributes, borders, spacing, and layout.

use crate::color::Color;
use crate::color::parse_hex_rgba;
use crate::output::{ColorProfileKind, color_profile};
use crate::security::{safe_repeat, safe_str_repeat};
use crate::style::{Style, properties::*};
use crate::width_visible;

impl Style {
    /// Renders text with all configured style properties applied.
    ///
    /// This method applies the complete style configuration to the provided text,
    /// generating ANSI escape sequences for terminal display. It handles:
    ///
    /// - Text attributes (bold, italic, underline, etc.)
    /// - Foreground and background colors
    /// - Borders and border colors
    /// - Padding and margins
    /// - Text alignment and positioning
    /// - Size constraints and content wrapping
    ///
    /// # Arguments
    ///
    /// * `s` - The text content to render with this style
    ///
    /// # Returns
    ///
    /// A string containing the input text wrapped with appropriate ANSI escape
    /// sequences and formatting to display the styled content in the terminal.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lipgloss::Style;
    ///
    /// // Create a style and render text with it
    /// let style = Style::default();
    /// let output = style.render("Hello, World!");
    /// // Returns "Hello, World!" with any configured style properties applied
    /// ```
    ///
    /// # Performance
    ///
    /// This method efficiently builds ANSI sequences by only including codes for
    /// properties that have been explicitly set on the style, minimizing output size.
    pub fn render(&self, s: &str) -> String {
        if self.is_input_passthrough(s) {
            return s.to_string();
        }

        // Layout pass: borders, padding, wrapping, and size constraints applied
        // to plain text before any colorization happens.
        let rendered = self.prepare_block(s);

        let target_width = self.get_width();
        let target_height = self.get_height();

        // Determine if we need to render any borders. If so, we must not early-return.
        let has_borders = (self.get_border_top()
            || self.get_border_right()
            || self.get_border_bottom()
            || self.get_border_left())
            && self.is_set(BORDER_STYLE_KEY);

        // Check if we have margins
        let has_margins = self.is_set(MARGIN_TOP_KEY)
            || self.is_set(MARGIN_RIGHT_KEY)
            || self.is_set(MARGIN_BOTTOM_KEY)
            || self.is_set(MARGIN_LEFT_KEY);

        let needs_text_styling = self.has_text_styling_keys();

        // Fast path: plain text with no layout, borders, margins, or styling.
        if !needs_text_styling
            && target_width <= 0
            && target_height <= 0
            && !has_borders
            && !has_margins
        {
            return rendered;
        }

        let margin_needs_color = has_margins
            && self.is_set(MARGIN_BACKGROUND_KEY)
            && self.get_margin_background().is_some();
        let needs_profile = needs_text_styling || has_borders || margin_needs_color;
        let profile = if needs_profile {
            match &self.r {
                Some(ctx) => ctx.color_profile(),
                None => color_profile(),
            }
        } else {
            ColorProfileKind::NoColor
        };

        let sgr_prefix = if needs_text_styling {
            self.build_text_sgr_prefix(profile)
        } else {
            String::new()
        };

        // LAYOUT FIRST: build the aligned canvas, then borders, then styling.
        let mut final_lines = self.apply_layout(&rendered, target_width, target_height);
        final_lines = self.apply_borders(final_lines, profile);
        final_lines = self.apply_text_sgr_styling(final_lines, &sgr_prefix);

        let result = Self::join_lines(&final_lines);

        // Apply all margins as final step (matches Go implementation)
        let result = self.apply_margins(&result, profile);

        if self.is_set(HYPERLINK_KEY)
            && let Some(ref link) = self.hyperlink
        {
            let param_refs: [&str; 1];
            let params: &[&str] = if self.hyperlink_params.is_empty() {
                &[]
            } else {
                param_refs = [self.hyperlink_params.as_str()];
                &param_refs
            };
            let open = ansi::hyperlink::set_hyperlink(link, params);
            let close = ansi::hyperlink::reset_hyperlink(&[]);
            return format!("{open}{result}{close}");
        }

        result
    }

    /// Returns true when text attributes or colors are configured.
    ///
    /// Used to skip OutputContext/SGR work on the plain-text fast path.
    fn has_text_styling_keys(&self) -> bool {
        const ATTRS: [(u32, PropKey); 7] = [
            (ATTR_BOLD, BOLD_KEY),
            (ATTR_FAINT, FAINT_KEY),
            (ATTR_ITALIC, ITALIC_KEY),
            (ATTR_UNDERLINE, UNDERLINE_KEY),
            (ATTR_BLINK, BLINK_KEY),
            (ATTR_REVERSE, REVERSE_KEY),
            (ATTR_STRIKETHROUGH, STRIKETHROUGH_KEY),
        ];
        for (attr, key) in ATTRS {
            if self.get_attr(attr) && self.is_set(key) {
                return true;
            }
        }
        self.is_set(FOREGROUND_KEY) || self.is_set(BACKGROUND_KEY)
    }

    /// True when the input string can be returned without any transformation.
    fn is_input_passthrough(&self, s: &str) -> bool {
        if !self.value.is_empty() {
            return false;
        }
        if self.has_text_styling_keys() {
            return false;
        }
        if self.get_width() > 0
            || self.get_height() > 0
            || self.get_max_height() > 0
            || self.get_max_width() > 0
        {
            return false;
        }
        if self.needs_padding() || self.needs_size_constraints() {
            return false;
        }
        if (self.get_border_top()
            || self.get_border_right()
            || self.get_border_bottom()
            || self.get_border_left())
            && self.is_set(BORDER_STYLE_KEY)
        {
            return false;
        }
        if self.is_set(MARGIN_TOP_KEY)
            || self.is_set(MARGIN_RIGHT_KEY)
            || self.is_set(MARGIN_BOTTOM_KEY)
            || self.is_set(MARGIN_LEFT_KEY)
        {
            return false;
        }
        if self.get_attr(ATTR_INLINE) && self.is_set(INLINE_KEY) {
            return false;
        }
        if self.is_set(TRANSFORM_KEY) && self.transform.is_some() {
            return false;
        }
        if s.contains('\r') {
            return false;
        }
        let tabw = self.get_tab_width();
        if tabw == 0 && s.contains('\t') {
            return false;
        }
        if tabw > 0 && s.contains('\t') {
            return false;
        }
        true
    }

    /// Wraps a space run with a background SGR code without running the full render pipeline.
    fn margin_background_spaces(spaces: &str, bg_sgr: &str) -> String {
        if bg_sgr.is_empty() {
            spaces.to_string()
        } else {
            format!("\x1b[{}m{}\x1b[0m", bg_sgr, spaces)
        }
    }

    /// Resolves the margin background to a bare SGR parameter string (no ESC prefix).
    fn margin_background_sgr(bg: &Color, profile: ColorProfileKind) -> String {
        Style::new()
            .background(bg.clone())
            .background_sgr(profile)
            .unwrap_or_default()
    }

    /// Applies the layout-affecting transforms that operate on plain text.
    ///
    /// This covers newline normalization, inline collapsing, the user transform,
    /// tab expansion, max-height/width truncation, word wrapping, and horizontal
    /// and vertical padding. The result is the text canvas prior to colorization.
    fn prepare_block(&self, s: &str) -> String {
        // Content to render: prefer internal value if set
        let mut rendered = if !self.value.is_empty() {
            self.value.clone()
        } else {
            s.to_string()
        };

        // Normalize newlines: convert CRLF/CR to LF
        if rendered.contains('\r') {
            rendered = rendered.replace("\r\n", "\n");
            rendered = rendered.replace('\r', "\n");
        }

        // Inline: remove newlines if inline=true
        if self.get_attr(ATTR_INLINE) && self.is_set(INLINE_KEY) {
            rendered = rendered.replace('\n', "");
        }

        // Apply transform, if any
        if self.is_set(TRANSFORM_KEY)
            && let Some(ref f) = self.transform
        {
            rendered = f(rendered);
        }

        // Tabs handling: default 4 spaces, 0 removes, -1 keeps as-is, n>0 replaces with n spaces
        let tabw = self.get_tab_width();
        if tabw == 0 && rendered.contains('\t') {
            rendered = rendered.replace('\t', "");
        } else if tabw > 0 && rendered.contains('\t') {
            let spaces = safe_repeat(' ', tabw as usize);
            rendered = rendered.replace('\t', &spaces);
        } // tabw < 0 => keep tabs as-is

        if self.needs_size_constraints() {
            rendered = self.apply_size_constraints(rendered);
        }
        if self.needs_padding() {
            rendered = self.apply_padding(rendered);
        }
        rendered
    }

    /// True when max-height, max-width, or width-driven wrapping may apply.
    fn needs_size_constraints(&self) -> bool {
        self.get_max_height() > 0 || self.get_max_width() > 0 || self.get_width() > 0
    }

    /// True when any padding side is explicitly configured.
    fn needs_padding(&self) -> bool {
        self.is_set(PADDING_TOP_KEY)
            || self.is_set(PADDING_BOTTOM_KEY)
            || self.is_set(PADDING_LEFT_KEY)
            || self.is_set(PADDING_RIGHT_KEY)
    }

    /// Applies max-height/max-width truncation and width-driven word wrapping.
    ///
    /// The configured width includes horizontal padding, so the wrap width is the
    /// configured width minus left/right padding.
    fn apply_size_constraints(&self, mut rendered: String) -> String {
        // Max height truncation
        let mh = self.get_max_height();
        if mh > 0 {
            let lines: Vec<&str> = rendered.split('\n').collect();
            if (lines.len() as i32) > mh {
                rendered = lines[..mh as usize].join("\n");
            }
        }

        // Max width truncation per line (ANSI-aware)
        let mw = self.get_max_width();
        if mw > 0 {
            let lines: Vec<&str> = rendered.split('\n').collect();
            let mut out_lines: Vec<String> = Vec::with_capacity(lines.len());
            for line in lines {
                out_lines.push(Self::truncate_visible_line(line, mw as usize));
            }
            rendered = out_lines.join("\n");
        }

        // Word wrap when width > 0
        let w_setting = self.get_width();
        if w_setting > 0 {
            let pad_l = if self.is_set(PADDING_LEFT_KEY) {
                self.get_padding_left().max(0)
            } else {
                0
            };
            let pad_r = if self.is_set(PADDING_RIGHT_KEY) {
                self.get_padding_right().max(0)
            } else {
                0
            };

            // Calculate the actual content width by subtracting horizontal padding
            let content_width = (w_setting - pad_l - pad_r).max(0);

            if content_width > 0 {
                let mut wrapped_lines: Vec<String> = Vec::new();
                for line in rendered.split('\n') {
                    let mut parts = Self::word_wrap_ansi_aware(line, content_width as usize);
                    if parts.is_empty() {
                        parts.push(String::new());
                    }
                    wrapped_lines.extend(parts);
                }
                rendered = wrapped_lines.join("\n");
            }
        }

        rendered
    }

    /// Applies horizontal (per-line) then vertical (empty-line) padding.
    ///
    /// Horizontal alignment is handled later in the styling pass.
    fn apply_padding(&self, rendered: String) -> String {
        let rendered = self.apply_horizontal_padding(rendered);
        self.apply_vertical_padding(rendered)
    }

    /// Prefixes/suffixes each line with the configured left/right padding spaces.
    fn apply_horizontal_padding(&self, mut rendered: String) -> String {
        let pad_l = if self.is_set(PADDING_LEFT_KEY) {
            self.get_padding_left().max(0) as usize
        } else {
            0
        };
        let pad_r = if self.is_set(PADDING_RIGHT_KEY) {
            self.get_padding_right().max(0) as usize
        } else {
            0
        };
        if pad_l > 0 || pad_r > 0 {
            let lines: Vec<&str> = rendered.split('\n').collect();
            let mut padded: Vec<String> = Vec::with_capacity(lines.len());
            let lp = safe_repeat(' ', pad_l);
            let rp = safe_repeat(' ', pad_r);
            for line in lines {
                padded.push(format!("{}{}{}", lp, line, rp));
            }
            rendered = padded.join("\n");
        }
        rendered
    }

    /// Prepends/appends empty lines for the configured top/bottom padding.
    ///
    /// Padding counts are capped to bound allocations on pathological inputs.
    fn apply_vertical_padding(&self, mut rendered: String) -> String {
        let pad_t = if self.is_set(PADDING_TOP_KEY) {
            self.get_padding_top().max(0) as usize
        } else {
            0
        };
        let pad_b = if self.is_set(PADDING_BOTTOM_KEY) {
            self.get_padding_bottom().max(0) as usize
        } else {
            0
        };
        if pad_t > 0 || pad_b > 0 {
            let mut lines = Vec::new();

            if pad_t > 0 {
                let safe_pad_t = pad_t.min(1000);
                for _ in 0..safe_pad_t {
                    lines.push(String::new());
                }
            }

            lines.extend(rendered.split('\n').map(|s| s.to_string()));

            if pad_b > 0 {
                let safe_pad_b = pad_b.min(1000);
                for _ in 0..safe_pad_b {
                    lines.push(String::new());
                }
            }

            rendered = lines.join("\n");
        }
        rendered
    }

    /// Maps a 0-255 ANSI color index to its SGR code for the 16-color profile.
    ///
    /// `fg` selects the foreground (30/90 base, default 39) or background (40/100
    /// base, default 49) code space. Indices 0-7 map to the standard range, 8-15
    /// to the bright range, already-encoded codes pass through, and anything else
    /// falls back to the profile default.
    fn ansi_indexed_sgr(idx: u32, fg: bool) -> String {
        let base = if fg { 30 } else { 40 };
        let bright_base = if fg { 82 } else { 92 };
        let default = if fg { 39 } else { 49 };
        if idx <= 7 {
            format!("{}", base + idx) // standard colors
        } else if idx <= 15 {
            format!("{}", bright_base + idx) // bright colors
        } else if (base..=base + 7).contains(&idx) || (base + 60..=base + 67).contains(&idx) {
            format!("{}", idx) // already a standard or bright ANSI code
        } else {
            format!("{}", default)
        }
    }

    /// Resolves a single border color token to its SGR code fragment.
    ///
    /// `fg` selects foreground (`38`) versus background (`48`) codes. Hex tokens
    /// render as truecolor, falling back to the dark indexed code when parsing
    /// fails; numeric tokens map through [`Self::ansi_indexed_sgr`] for the ANSI
    /// profile and to the 256-color code otherwise. Returns `None` for tokens that
    /// are neither hex nor numeric.
    fn border_color_code(profile: ColorProfileKind, tok: &str, fg: bool) -> Option<String> {
        let (truecolor_lead, indexed_lead, hex_fallback) = if fg {
            ("38;2", "38;5", "38;5;0")
        } else {
            ("48;2", "48;5", "48;5;0")
        };
        if tok.starts_with('#') {
            // RGB values are already 8-bit (0-255) cast to u32
            match parse_hex_rgba(tok) {
                Some((r, g, b, _)) => Some(format!("{};{};{};{}", truecolor_lead, r, g, b)),
                None => Some(hex_fallback.to_string()),
            }
        } else if let Ok(idx) = tok.parse::<u32>() {
            let idx = idx % 256;
            match profile {
                ColorProfileKind::ANSI => Some(Self::ansi_indexed_sgr(idx, fg)),
                _ => Some(format!("{};{}", indexed_lead, idx)),
            }
        } else {
            None
        }
    }

    /// Builds the full ESC SGR prefix (`\x1b[...]m`) for text body styling.
    ///
    /// Borders and margins are handled separately, after layout constraints,
    /// to match the Go implementation.
    fn build_text_sgr_prefix(&self, profile: ColorProfileKind) -> String {
        let mut codes = String::new();
        const ATTRS: [(u32, PropKey, &str); 7] = [
            (ATTR_BOLD, BOLD_KEY, "1"),
            (ATTR_FAINT, FAINT_KEY, "2"),
            (ATTR_ITALIC, ITALIC_KEY, "3"),
            (ATTR_UNDERLINE, UNDERLINE_KEY, "4"),
            (ATTR_BLINK, BLINK_KEY, "5"),
            (ATTR_REVERSE, REVERSE_KEY, "7"),
            (ATTR_STRIKETHROUGH, STRIKETHROUGH_KEY, "9"),
        ];
        for (attr, key, code) in ATTRS {
            if self.get_attr(attr) && self.is_set(key) {
                if !codes.is_empty() {
                    codes.push(';');
                }
                if attr == ATTR_UNDERLINE
                    && self.is_set(UNDERLINE_STYLE_KEY)
                    && self.underline_style > 1
                {
                    codes.push_str(&format!("4:{}", self.underline_style));
                } else {
                    codes.push_str(code);
                }
            }
        }
        if let Some(code) = self.foreground_sgr(profile) {
            if !codes.is_empty() {
                codes.push(';');
            }
            codes.push_str(&code);
        }
        if let Some(code) = self.background_sgr(profile) {
            if !codes.is_empty() {
                codes.push(';');
            }
            codes.push_str(&code);
        }
        if codes.is_empty() {
            String::new()
        } else {
            format!("\x1b[{}m", codes)
        }
    }

    /// Wraps styled content with a pre-built SGR prefix and reset suffix.
    fn wrap_with_sgr_prefix(prefix: &str, content: &str) -> String {
        const SUFFIX: &str = "\x1b[0m";
        let mut out = String::with_capacity(prefix.len() + content.len() + SUFFIX.len());
        out.push_str(prefix);
        out.push_str(content);
        out.push_str(SUFFIX);
        out
    }

    /// Returns leading/trailing ASCII space padding bounds for layout lines.
    fn ascii_space_padding_bounds(line: &str) -> (usize, usize) {
        if !line.is_ascii() {
            let leading = line.chars().take_while(|&c| c == ' ').count();
            let trailing = line.chars().rev().take_while(|&c| c == ' ').count();
            return (leading, trailing);
        }
        let bytes = line.as_bytes();
        let leading = bytes.iter().position(|&b| b != b' ').unwrap_or(bytes.len());
        let trailing = bytes
            .iter()
            .rev()
            .position(|&b| b != b' ')
            .unwrap_or(bytes.len());
        (leading, trailing)
    }

    /// Resolves the foreground text color to an SGR code for the active profile.
    ///
    /// Hex tokens render as truecolor and collapse to the dark indexed code on the
    /// 16/256-color profiles; numeric tokens use the indexed code, mapping through
    /// [`Self::ansi_indexed_sgr`] for the 16-color profile.
    fn foreground_sgr(&self, profile: ColorProfileKind) -> Option<String> {
        if matches!(profile, ColorProfileKind::NoColor) || !self.is_set(FOREGROUND_KEY) {
            return None;
        }
        let tok = self.fg_color.as_ref()?;
        if tok.starts_with('#') {
            // RGB values are already 8-bit (0-255) cast to u32
            let (r, g, b, _a) = parse_hex_rgba(tok)?;
            match profile {
                ColorProfileKind::TrueColor => Some(format!("38;2;{};{};{}", r, g, b)),
                ColorProfileKind::ANSI | ColorProfileKind::ANSI256 => Some("38;5;0".to_string()),
                ColorProfileKind::NoColor => None,
            }
        } else if let Ok(idx) = tok.parse::<u32>() {
            let idx = idx % 256;
            match profile {
                // best-effort: still use indexed if we don't have original hex
                ColorProfileKind::TrueColor | ColorProfileKind::ANSI256 => {
                    Some(format!("38;5;{}", idx))
                }
                ColorProfileKind::ANSI => Some(Self::ansi_indexed_sgr(idx, true)),
                ColorProfileKind::NoColor => None,
            }
        } else {
            None
        }
    }

    /// Resolves the background color to an SGR code for the active profile.
    ///
    /// Unlike the foreground, hex tokens are quantized to the nearest ANSI256 or
    /// ANSI16 background code on the reduced-color profiles to preserve the visual
    /// tone; numeric tokens use the indexed code or the 16-color mapping.
    fn background_sgr(&self, profile: ColorProfileKind) -> Option<String> {
        if matches!(profile, ColorProfileKind::NoColor) || !self.is_set(BACKGROUND_KEY) {
            return None;
        }
        let tok = self.bg_color.as_ref()?;
        if tok.starts_with('#') {
            // RGB values are already 8-bit (0-255) cast to u32
            let (r, g, b, _a) = parse_hex_rgba(tok)?;
            match profile {
                ColorProfileKind::TrueColor => Some(format!("48;2;{};{};{}", r, g, b)),
                ColorProfileKind::ANSI256 => {
                    let ansi256_idx = crate::color::rgb_to_ansi256(r as u8, g as u8, b as u8);
                    Some(format!("48;5;{}", ansi256_idx))
                }
                ColorProfileKind::ANSI => {
                    let ansi16_idx = crate::color::rgb_to_ansi16(r as u8, g as u8, b as u8);
                    Some(format!("{}", 40 + ansi16_idx))
                }
                ColorProfileKind::NoColor => None,
            }
        } else if let Ok(idx) = tok.parse::<u32>() {
            let idx = idx % 256;
            match profile {
                ColorProfileKind::TrueColor | ColorProfileKind::ANSI256 => {
                    Some(format!("48;5;{}", idx))
                }
                ColorProfileKind::ANSI => Some(Self::ansi_indexed_sgr(idx, false)),
                ColorProfileKind::NoColor => None,
            }
        } else {
            None
        }
    }

    /// Builds the aligned, full-size canvas using the "Layout First" approach.
    ///
    /// Each line is padded to `target_width` according to horizontal alignment, and
    /// the block is padded to `target_height` according to vertical alignment.
    fn apply_layout(&self, rendered: &str, target_width: i32, target_height: i32) -> Vec<String> {
        let lines: Vec<&str> = rendered.split('\n').collect();
        if target_width <= 0 && target_height <= 0 {
            return lines.into_iter().map(str::to_string).collect();
        }

        let mut final_lines = Vec::with_capacity(lines.len());

        // LAYOUT FIRST: Create full-width canvas with alignment padding
        for line in lines {
            let mut canvas_line = line.to_string();

            // If a width is set, create full-width canvas with alignment padding
            if target_width > 0 {
                let line_vis_width = width_visible(&canvas_line);
                let gap = (target_width as usize).saturating_sub(line_vis_width);

                if gap > 0 {
                    let h_pos = self.get_align_horizontal().value();
                    let left_gap = (gap as f64 * h_pos).round() as usize;
                    let right_gap = gap - left_gap;

                    let left_pad = safe_repeat(' ', left_gap);
                    let right_pad = safe_repeat(' ', right_gap);

                    // Create full-width canvas by adding alignment padding
                    canvas_line = format!("{}{}{}", left_pad, canvas_line, right_pad);
                }
            }

            final_lines.push(canvas_line);
        }

        // Height constraint with vertical alignment (integrated into Layout First phase)
        if target_height > 0 && (final_lines.len() as i32) < target_height {
            let gap = target_height as usize - final_lines.len();
            let v_pos = self.get_align_vertical().value();

            // Distribute padding lines based on vertical alignment
            // v_pos: 0.0=TOP (content at top, padding at bottom), 0.5=CENTER, 1.0=BOTTOM (content at bottom, padding at top)
            let top_pad_count = (gap as f64 * v_pos).round() as usize;
            let bottom_pad_count = gap - top_pad_count;

            // Determine width for padding lines to match existing canvas width
            let block_width = final_lines
                .iter()
                .map(|l| width_visible(l))
                .max()
                .unwrap_or(0);
            let empty_line = safe_repeat(' ', block_width);

            let mut height_adjusted = Vec::new();
            // Cap padding to prevent excessive allocations
            let safe_top_pad = top_pad_count.min(1000);
            let safe_bottom_pad = bottom_pad_count.min(1000);

            for _ in 0..safe_top_pad {
                height_adjusted.push(empty_line.clone());
            }
            height_adjusted.extend(final_lines);
            for _ in 0..safe_bottom_pad {
                height_adjusted.push(empty_line.clone());
            }

            final_lines = height_adjusted;
        }

        final_lines
    }

    /// Builds a horizontal (top or bottom) border edge line.
    ///
    /// Returns an empty string when the edge is disabled. Corner glyphs are used
    /// only when the corresponding side border is present; otherwise the fill glyph
    /// is substituted. A non-empty `sgr` prefix wraps the line with a reset.
    fn horizontal_border_edge(
        &self,
        enabled: bool,
        sgr: &str,
        left_corner: &str,
        fill: &str,
        right_corner: &str,
        w: usize,
    ) -> String {
        if !enabled {
            return String::new();
        }
        let left = if self.get_border_left() {
            left_corner
        } else {
            fill
        };
        let right = if self.get_border_right() {
            right_corner
        } else {
            fill
        };
        let body = format!("{}{}{}", left, safe_str_repeat(fill, w), right);
        if sgr.is_empty() {
            body
        } else {
            format!("{}{}\x1b[0m", sgr, body)
        }
    }

    /// Builds the SGR prefix for one border side from per-side then combined tokens.
    ///
    /// Returns an empty string for the no-color profile or when neither a
    /// foreground nor background color resolves.
    fn border_edge_sgr(
        &self,
        profile: ColorProfileKind,
        fg_opt: &Option<String>,
        bg_opt: &Option<String>,
        fg_combined: &Option<String>,
        bg_combined: &Option<String>,
    ) -> String {
        if matches!(profile, ColorProfileKind::NoColor) {
            return String::new();
        }
        let mut parts: Vec<String> = Vec::new();
        if let Some(tok) = fg_opt.as_ref().or(fg_combined.as_ref())
            && let Some(code) = Self::border_color_code(profile, tok, true)
        {
            parts.push(code);
        }
        if let Some(tok) = bg_opt.as_ref().or(bg_combined.as_ref())
            && let Some(code) = Self::border_color_code(profile, tok, false)
        {
            parts.push(code);
        }
        if parts.is_empty() {
            String::new()
        } else {
            format!("\x1b[{}m", parts.join(";"))
        }
    }

    /// Computes the SGR prefixes for all four border sides.
    ///
    /// Per-side colors take precedence; when no per-side color is set the main
    /// foreground/background tokens are used as the combined fallback. Returns the
    /// prefixes in `[top, right, bottom, left]` order.
    fn border_side_sgrs(&self, profile: ColorProfileKind) -> [String; 4] {
        // Use stored combined fields if set via colors.rs helpers
        let combined_fg = self.border_top_fg_color.is_some()
            || self.border_right_fg_color.is_some()
            || self.border_bottom_fg_color.is_some()
            || self.border_left_fg_color.is_some();
        let combined_bg = self.border_top_bg_color.is_some()
            || self.border_right_bg_color.is_some()
            || self.border_bottom_bg_color.is_some()
            || self.border_left_bg_color.is_some();
        let fg_combined = if combined_fg {
            None
        } else {
            self.fg_color.clone()
        };
        let bg_combined = if combined_bg {
            None
        } else {
            self.bg_color.clone()
        };

        [
            self.border_edge_sgr(
                profile,
                &self.border_top_fg_color,
                &self.border_top_bg_color,
                &fg_combined,
                &bg_combined,
            ),
            self.border_edge_sgr(
                profile,
                &self.border_right_fg_color,
                &self.border_right_bg_color,
                &fg_combined,
                &bg_combined,
            ),
            self.border_edge_sgr(
                profile,
                &self.border_bottom_fg_color,
                &self.border_bottom_bg_color,
                &fg_combined,
                &bg_combined,
            ),
            self.border_edge_sgr(
                profile,
                &self.border_left_fg_color,
                &self.border_left_bg_color,
                &fg_combined,
                &bg_combined,
            ),
        ]
    }

    /// Builds the middle (content) section of a bordered block.
    ///
    /// Each line is prefixed/suffixed with the left/right border glyphs (when
    /// enabled) and right-padded to the widest visible line so the right border
    /// aligns vertically.
    fn build_border_mid(
        &self,
        final_lines: &[String],
        left_sgr: &str,
        right_sgr: &str,
        left_glyph: &str,
        right_glyph: &str,
        w: usize,
    ) -> Vec<String> {
        let reset = "\x1b[0m";
        let left_part_base = if self.get_border_left() {
            if left_sgr.is_empty() {
                left_glyph.to_string()
            } else {
                format!("{}{}{}", left_sgr, left_glyph, reset)
            }
        } else {
            String::new()
        };
        let right_part_base = if self.get_border_right() {
            if right_sgr.is_empty() {
                right_glyph.to_string()
            } else {
                format!("{}{}{}", right_sgr, right_glyph, reset)
            }
        } else {
            String::new()
        };
        let mut out_lines: Vec<String> = Vec::with_capacity(final_lines.len());
        for l in final_lines {
            let lw = width_visible(l);
            let pad = w.saturating_sub(lw);
            let mut line_buf =
                String::with_capacity(left_part_base.len() + l.len() + pad + right_part_base.len());
            line_buf.push_str(&left_part_base);
            line_buf.push_str(l);
            if pad > 0 {
                line_buf.push_str(&safe_repeat(' ', pad));
            }
            line_buf.push_str(&right_part_base);
            out_lines.push(line_buf);
        }
        out_lines
    }

    /// Wraps the laid-out canvas in border glyphs when a border style is set.
    ///
    /// Per-side colors fall back to the combined foreground/background tokens when
    /// no explicit per-side color is configured. Lines are padded to the widest
    /// visible line so the right border aligns.
    fn apply_borders(&self, final_lines: Vec<String>, profile: ColorProfileKind) -> Vec<String> {
        let render_borders = (self.get_border_top()
            || self.get_border_right()
            || self.get_border_bottom()
            || self.get_border_left())
            && self.is_set(BORDER_STYLE_KEY);
        if !render_borders {
            return final_lines;
        }

        let b = self.get_border_style();
        // Compute target width from the maximum visible width across all lines
        let mut w: usize = 0;
        for l in &final_lines {
            w = w.max(width_visible(l));
        }

        let [top_sgr, right_sgr, bottom_sgr, left_sgr] = self.border_side_sgrs(profile);

        // Build top border (conditionally)
        let top = self.horizontal_border_edge(
            self.get_border_top(),
            &top_sgr,
            b.top_left,
            b.top,
            b.top_right,
            w,
        );

        // Add left/right borders per line, padding each line to the max width
        let mid = self.build_border_mid(&final_lines, &left_sgr, &right_sgr, b.left, b.right, w);

        // Build bottom border (conditionally)
        let bot = self.horizontal_border_edge(
            self.get_border_bottom(),
            &bottom_sgr,
            b.bottom_left,
            b.bottom,
            b.bottom_right,
            w,
        );

        let mut bordered_lines = Vec::with_capacity(
            usize::from(self.get_border_top()) + mid.len() + usize::from(self.get_border_bottom()),
        );
        if !top.is_empty() {
            bordered_lines.push(top);
        }
        bordered_lines.extend(mid);
        if !bot.is_empty() {
            bordered_lines.push(bot);
        }
        bordered_lines
    }

    /// Applies the computed SGR codes to a laid-out canvas.
    ///
    /// When a background color or styled whitespace is active the whole line is
    /// wrapped; otherwise only the non-whitespace span is colored so that
    /// alignment padding stays uncolored.
    fn apply_text_sgr_styling(&self, final_lines: Vec<String>, sgr_prefix: &str) -> Vec<String> {
        if sgr_prefix.is_empty() {
            return final_lines;
        }

        const SUFFIX: &str = "\x1b[0m";

        let style_whole_line = self.get_background().is_some()
            || self.get_color_whitespace()
            || (self.get_underline() && self.get_underline_spaces())
            || (self.get_strikethrough() && self.get_strikethrough_spaces());

        if style_whole_line {
            final_lines
                .into_iter()
                .map(|line| Self::wrap_with_sgr_prefix(sgr_prefix, &line))
                .collect()
        } else {
            final_lines
                .into_iter()
                .map(|line| {
                    let (leading_spaces, trailing_spaces) = Self::ascii_space_padding_bounds(&line);
                    let content_start = leading_spaces;
                    let content_end = line.len().saturating_sub(trailing_spaces);

                    if content_start >= content_end {
                        line
                    } else {
                        let lead = &line[..content_start];
                        let mid = &line[content_start..content_end];
                        let trail = &line[content_end..];
                        let mut out = String::with_capacity(
                            lead.len() + sgr_prefix.len() + mid.len() + SUFFIX.len() + trail.len(),
                        );
                        out.push_str(lead);
                        out.push_str(sgr_prefix);
                        out.push_str(mid);
                        out.push_str(SUFFIX);
                        out.push_str(trail);
                        out
                    }
                })
                .collect()
        }
    }

    /// Apply margins to a fully-rendered block, using margin background color if set.
    /// This matches the Go implementation's applyMargins function.
    fn apply_margins(&self, block: &str, profile: ColorProfileKind) -> String {
        let top_margin = if self.is_set(MARGIN_TOP_KEY) {
            self.get_margin_top().max(0) as usize
        } else {
            0
        };
        let right_margin = if self.is_set(MARGIN_RIGHT_KEY) {
            self.get_margin_right().max(0) as usize
        } else {
            0
        };
        let bottom_margin = if self.is_set(MARGIN_BOTTOM_KEY) {
            self.get_margin_bottom().max(0) as usize
        } else {
            0
        };
        let left_margin = if self.is_set(MARGIN_LEFT_KEY) {
            self.get_margin_left().max(0) as usize
        } else {
            0
        };

        if top_margin == 0 && right_margin == 0 && bottom_margin == 0 && left_margin == 0 {
            return block.to_string();
        }

        // Determine margin background color
        // In Go: if marginBgColor is not set, margin is transparent (no background)
        // Only inherit from main background if explicitly requested via margin_background
        let margin_bg_color = if self.is_set(MARGIN_BACKGROUND_KEY) {
            self.get_margin_background()
        } else {
            // Margins are transparent by default to match Go behavior
            None
        };

        // Pre-render margin strings once to avoid repeated render calls
        let margin_bg_sgr = margin_bg_color
            .as_ref()
            .map(|bg| Self::margin_background_sgr(bg, profile));
        let (left_margin_str, right_margin_str) = if let Some(ref bg_sgr) = margin_bg_sgr {
            let left = if left_margin > 0 {
                Self::margin_background_spaces(&safe_repeat(' ', left_margin.min(1000)), bg_sgr)
            } else {
                String::new()
            };
            let right = if right_margin > 0 {
                Self::margin_background_spaces(&safe_repeat(' ', right_margin.min(1000)), bg_sgr)
            } else {
                String::new()
            };
            (left, right)
        } else {
            // No background color, just use plain spaces
            let left = if left_margin > 0 {
                safe_repeat(' ', left_margin.min(1000))
            } else {
                String::new()
            };
            let right = if right_margin > 0 {
                safe_repeat(' ', right_margin.min(1000))
            } else {
                String::new()
            };
            (left, right)
        };

        // Apply left and right margins to each line
        let lines: Vec<String> = block
            .split('\n')
            .map(|line| format!("{}{}{}", left_margin_str, line, right_margin_str))
            .collect();

        let mut result = Vec::new();

        // Apply top margin
        if top_margin > 0 {
            let block_width = lines.iter().map(|l| width_visible(l)).max().unwrap_or(0);
            let empty_line = if block_width > 0 {
                if let Some(ref bg_sgr) = margin_bg_sgr {
                    Self::margin_background_spaces(&safe_repeat(' ', block_width.min(1000)), bg_sgr)
                } else {
                    safe_repeat(' ', block_width.min(1000))
                }
            } else {
                String::new()
            };
            // Cap margin to prevent excessive allocations
            let safe_top_margin = top_margin.min(1000);
            for _ in 0..safe_top_margin {
                result.push(empty_line.clone());
            }
        }

        result.extend(lines);

        // Apply bottom margin
        if bottom_margin > 0 {
            let block_width = result.iter().map(|l| width_visible(l)).max().unwrap_or(0);
            let empty_line = if block_width > 0 {
                if let Some(ref bg_sgr) = margin_bg_sgr {
                    Self::margin_background_spaces(&safe_repeat(' ', block_width.min(1000)), bg_sgr)
                } else {
                    safe_repeat(' ', block_width.min(1000))
                }
            } else {
                String::new()
            };
            // Cap margin to prevent excessive allocations
            let safe_bottom_margin = bottom_margin.min(1000);
            for _ in 0..safe_bottom_margin {
                result.push(empty_line.clone());
            }
        }

        Self::join_lines(&result)
    }

    /// Joins lines with a single pre-sized buffer.
    fn join_lines(lines: &[String]) -> String {
        if lines.is_empty() {
            return String::new();
        }
        if lines.len() == 1 {
            return lines[0].clone();
        }
        let total_len = lines.iter().map(String::len).sum::<usize>() + lines.len() - 1;
        let mut out = String::with_capacity(total_len);
        for (i, line) in lines.iter().enumerate() {
            if i > 0 {
                out.push('\n');
            }
            out.push_str(line);
        }
        out
    }

    /// Applies this style to a string as a convenience wrapper around `render()`.
    ///
    /// This method is a direct alias for [`render()`](Self::render) and provides
    /// the same functionality with a more concise name for common usage patterns.
    ///
    /// # Arguments
    ///
    /// * `s` - The text content to style
    ///
    /// # Returns
    ///
    /// A string containing the input text with all style properties applied,
    /// wrapped with appropriate ANSI escape sequences for terminal display.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lipgloss::Style;
    ///
    /// let style = Style::default();
    ///
    /// // These two calls are equivalent:
    /// let output1 = style.apply("Warning!");
    /// let output2 = style.render("Warning!");
    /// assert_eq!(output1, output2);
    /// ```
    pub fn apply(&self, s: &str) -> String {
        self.render(s)
    }
}
