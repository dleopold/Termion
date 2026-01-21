//! Theme definitions for the TUI.
//!
//! This module provides color themes that can be swapped to change
//! the visual appearance of the interface.

use ratatui::style::Color;

/// A complete color theme for the TUI.
#[derive(Debug, Clone)]
pub struct Theme {
    /// Name of the theme
    pub name: &'static str,

    // === Borders ===
    /// Primary border color
    pub border: Color,
    /// Dimmed/secondary border color
    pub border_dim: Color,
    /// Highlighted/focused border color  
    pub border_highlight: Color,

    // === Text ===
    /// Primary text color
    pub text: Color,
    /// Dimmed/secondary text (labels, hints)
    pub text_dim: Color,
    /// Title text color
    pub text_title: Color,

    // === Status indicators ===
    /// Success state (connected, running, passed)
    pub success: Color,
    /// Warning state (paused, attention needed)
    pub warning: Color,
    /// Error state (disconnected, failed)
    pub error: Color,
    /// Info/starting state
    pub info: Color,
    /// Idle/inactive state
    pub idle: Color,
    /// Special state (mux scan)
    pub special: Color,

    // === Charts ===
    /// Primary chart data color
    pub chart_line: Color,
    /// Chart fill/area color
    pub chart_fill: Color,
    /// Chart passed data
    pub chart_passed: Color,
    /// Chart failed data
    pub chart_failed: Color,
    /// Chart axis and labels
    pub chart_axis: Color,

    // === Channel map states ===
    /// Sequencing pores
    pub channel_sequencing: Color,
    /// Available pores
    pub channel_pore: Color,
    /// Unavailable pores
    pub channel_unavailable: Color,
    /// Inactive pores
    pub channel_inactive: Color,
    /// Adapter/event
    pub channel_adapter: Color,
    /// Other/unknown state
    pub channel_other: Color,
    /// Empty/no data
    pub channel_empty: Color,

    // === UI elements ===
    /// Selected row background
    pub selection_bg: Color,
    /// Selected row text
    pub selection_fg: Color,
    /// Keybinding hints
    pub key_hint: Color,
    /// Background color
    pub background: Color,
}

impl Default for Theme {
    fn default() -> Self {
        Self::default_theme()
    }
}

impl Theme {
    /// Default theme - cyan accent with standard terminal colors
    pub fn default_theme() -> Self {
        Self {
            name: "Default",

            border: Color::Cyan,
            border_dim: Color::DarkGray,
            border_highlight: Color::Cyan,

            text: Color::White,
            text_dim: Color::DarkGray,
            text_title: Color::Cyan,

            success: Color::Green,
            warning: Color::Yellow,
            error: Color::Red,
            info: Color::Cyan,
            idle: Color::DarkGray,
            special: Color::Magenta,

            chart_line: Color::Cyan,
            chart_fill: Color::Blue,
            chart_passed: Color::Green,
            chart_failed: Color::Red,
            chart_axis: Color::DarkGray,

            channel_sequencing: Color::Green,
            channel_pore: Color::Blue,
            channel_unavailable: Color::Magenta,
            channel_inactive: Color::Cyan,
            channel_adapter: Color::Yellow,
            channel_other: Color::DarkGray,
            channel_empty: Color::Black,

            selection_bg: Color::DarkGray,
            selection_fg: Color::White,
            key_hint: Color::Yellow,
            background: Color::Black,
        }
    }

    /// Catppuccin Mocha - warm pastel theme
    pub fn catppuccin_mocha() -> Self {
        Self {
            name: "Catppuccin Mocha",

            border: Color::Rgb(180, 190, 254),           // Lavender
            border_dim: Color::Rgb(88, 91, 112),         // Surface2
            border_highlight: Color::Rgb(203, 166, 247), // Mauve

            text: Color::Rgb(205, 214, 244),       // Text
            text_dim: Color::Rgb(147, 153, 178),   // Overlay1
            text_title: Color::Rgb(180, 190, 254), // Lavender

            success: Color::Rgb(166, 227, 161), // Green
            warning: Color::Rgb(249, 226, 175), // Yellow
            error: Color::Rgb(243, 139, 168),   // Red
            info: Color::Rgb(137, 220, 235),    // Sky
            idle: Color::Rgb(88, 91, 112),      // Surface2
            special: Color::Rgb(203, 166, 247), // Mauve

            chart_line: Color::Rgb(137, 220, 235),   // Sky
            chart_fill: Color::Rgb(116, 199, 236),   // Sapphire
            chart_passed: Color::Rgb(166, 227, 161), // Green
            chart_failed: Color::Rgb(243, 139, 168), // Red
            chart_axis: Color::Rgb(147, 153, 178),   // Overlay1

            channel_sequencing: Color::Rgb(166, 227, 161), // Green
            channel_pore: Color::Rgb(116, 199, 236),       // Sapphire
            channel_unavailable: Color::Rgb(203, 166, 247), // Mauve
            channel_inactive: Color::Rgb(137, 220, 235),   // Sky
            channel_adapter: Color::Rgb(249, 226, 175),    // Yellow
            channel_other: Color::Rgb(88, 91, 112),        // Surface2
            channel_empty: Color::Rgb(30, 30, 46),         // Base

            selection_bg: Color::Rgb(69, 71, 90),    // Surface1
            selection_fg: Color::Rgb(205, 214, 244), // Text
            key_hint: Color::Rgb(249, 226, 175),     // Yellow
            background: Color::Rgb(30, 30, 46),      // Base
        }
    }

    /// Dracula - purple/pink dark theme
    pub fn dracula() -> Self {
        Self {
            name: "Dracula",

            border: Color::Rgb(189, 147, 249),           // Purple
            border_dim: Color::Rgb(68, 71, 90),          // Current Line
            border_highlight: Color::Rgb(255, 121, 198), // Pink

            text: Color::Rgb(248, 248, 242),       // Foreground
            text_dim: Color::Rgb(98, 114, 164),    // Comment
            text_title: Color::Rgb(189, 147, 249), // Purple

            success: Color::Rgb(80, 250, 123),  // Green
            warning: Color::Rgb(241, 250, 140), // Yellow
            error: Color::Rgb(255, 85, 85),     // Red
            info: Color::Rgb(139, 233, 253),    // Cyan
            idle: Color::Rgb(98, 114, 164),     // Comment
            special: Color::Rgb(255, 121, 198), // Pink

            chart_line: Color::Rgb(139, 233, 253),  // Cyan
            chart_fill: Color::Rgb(189, 147, 249),  // Purple
            chart_passed: Color::Rgb(80, 250, 123), // Green
            chart_failed: Color::Rgb(255, 85, 85),  // Red
            chart_axis: Color::Rgb(98, 114, 164),   // Comment

            channel_sequencing: Color::Rgb(80, 250, 123), // Green
            channel_pore: Color::Rgb(139, 233, 253),      // Cyan
            channel_unavailable: Color::Rgb(255, 121, 198), // Pink
            channel_inactive: Color::Rgb(189, 147, 249),  // Purple
            channel_adapter: Color::Rgb(241, 250, 140),   // Yellow
            channel_other: Color::Rgb(98, 114, 164),      // Comment
            channel_empty: Color::Rgb(40, 42, 54),        // Background

            selection_bg: Color::Rgb(68, 71, 90), // Current Line
            selection_fg: Color::Rgb(248, 248, 242), // Foreground
            key_hint: Color::Rgb(241, 250, 140),  // Yellow
            background: Color::Rgb(40, 42, 54),   // Background
        }
    }

    /// Tokyo Night - cool blue/purple theme
    pub fn tokyo_night() -> Self {
        Self {
            name: "Tokyo Night",

            border: Color::Rgb(122, 162, 247),           // Blue
            border_dim: Color::Rgb(59, 66, 97),          // Comment
            border_highlight: Color::Rgb(125, 207, 255), // Cyan

            text: Color::Rgb(192, 202, 245),       // Foreground
            text_dim: Color::Rgb(86, 95, 137),     // Dark5
            text_title: Color::Rgb(122, 162, 247), // Blue

            success: Color::Rgb(158, 206, 106), // Green
            warning: Color::Rgb(224, 175, 104), // Orange
            error: Color::Rgb(247, 118, 142),   // Red
            info: Color::Rgb(125, 207, 255),    // Cyan
            idle: Color::Rgb(86, 95, 137),      // Dark5
            special: Color::Rgb(187, 154, 247), // Purple

            chart_line: Color::Rgb(125, 207, 255),   // Cyan
            chart_fill: Color::Rgb(122, 162, 247),   // Blue
            chart_passed: Color::Rgb(158, 206, 106), // Green
            chart_failed: Color::Rgb(247, 118, 142), // Red
            chart_axis: Color::Rgb(86, 95, 137),     // Dark5

            channel_sequencing: Color::Rgb(158, 206, 106), // Green
            channel_pore: Color::Rgb(122, 162, 247),       // Blue
            channel_unavailable: Color::Rgb(187, 154, 247), // Purple
            channel_inactive: Color::Rgb(125, 207, 255),   // Cyan
            channel_adapter: Color::Rgb(224, 175, 104),    // Orange
            channel_other: Color::Rgb(86, 95, 137),        // Dark5
            channel_empty: Color::Rgb(26, 27, 38),         // Background

            selection_bg: Color::Rgb(41, 46, 66),    // Selection
            selection_fg: Color::Rgb(192, 202, 245), // Foreground
            key_hint: Color::Rgb(224, 175, 104),     // Orange
            background: Color::Rgb(26, 27, 38),      // Background
        }
    }

    /// Gruvbox Dark - warm retro theme
    pub fn gruvbox() -> Self {
        Self {
            name: "Gruvbox",

            border: Color::Rgb(254, 128, 25),            // Orange
            border_dim: Color::Rgb(80, 73, 69),          // Bg2
            border_highlight: Color::Rgb(142, 192, 124), // Aqua

            text: Color::Rgb(235, 219, 178),      // Fg
            text_dim: Color::Rgb(146, 131, 116),  // Gray
            text_title: Color::Rgb(254, 128, 25), // Orange

            success: Color::Rgb(184, 187, 38),  // Green
            warning: Color::Rgb(250, 189, 47),  // Yellow
            error: Color::Rgb(251, 73, 52),     // Red
            info: Color::Rgb(131, 165, 152),    // Blue
            idle: Color::Rgb(146, 131, 116),    // Gray
            special: Color::Rgb(211, 134, 155), // Purple

            chart_line: Color::Rgb(142, 192, 124),  // Aqua
            chart_fill: Color::Rgb(131, 165, 152),  // Blue
            chart_passed: Color::Rgb(184, 187, 38), // Green
            chart_failed: Color::Rgb(251, 73, 52),  // Red
            chart_axis: Color::Rgb(146, 131, 116),  // Gray

            channel_sequencing: Color::Rgb(184, 187, 38), // Green
            channel_pore: Color::Rgb(131, 165, 152),      // Blue
            channel_unavailable: Color::Rgb(211, 134, 155), // Purple
            channel_inactive: Color::Rgb(142, 192, 124),  // Aqua
            channel_adapter: Color::Rgb(250, 189, 47),    // Yellow
            channel_other: Color::Rgb(80, 73, 69),        // Bg2
            channel_empty: Color::Rgb(40, 40, 40),        // Bg0

            selection_bg: Color::Rgb(60, 56, 54),    // Bg1
            selection_fg: Color::Rgb(235, 219, 178), // Fg
            key_hint: Color::Rgb(250, 189, 47),      // Yellow
            background: Color::Rgb(40, 40, 40),      // Bg0
        }
    }

    /// Nord - cool, muted arctic theme
    pub fn nord() -> Self {
        Self {
            name: "Nord",

            border: Color::Rgb(136, 192, 208),   // Frost cyan
            border_dim: Color::Rgb(76, 86, 106), // Polar Night 3
            border_highlight: Color::Rgb(143, 188, 187), // Frost teal

            text: Color::Rgb(236, 239, 244),       // Snow Storm 0
            text_dim: Color::Rgb(76, 86, 106),     // Polar Night 3
            text_title: Color::Rgb(136, 192, 208), // Frost cyan

            success: Color::Rgb(163, 190, 140), // Aurora green
            warning: Color::Rgb(235, 203, 139), // Aurora yellow
            error: Color::Rgb(191, 97, 106),    // Aurora red
            info: Color::Rgb(129, 161, 193),    // Frost blue
            idle: Color::Rgb(76, 86, 106),      // Polar Night 3
            special: Color::Rgb(180, 142, 173), // Aurora purple

            chart_line: Color::Rgb(136, 192, 208), // Frost cyan
            chart_fill: Color::Rgb(129, 161, 193), // Frost blue
            chart_passed: Color::Rgb(163, 190, 140), // Aurora green
            chart_failed: Color::Rgb(191, 97, 106), // Aurora red
            chart_axis: Color::Rgb(76, 86, 106),   // Polar Night 3

            channel_sequencing: Color::Rgb(163, 190, 140), // Aurora green
            channel_pore: Color::Rgb(129, 161, 193),       // Frost blue
            channel_unavailable: Color::Rgb(180, 142, 173), // Aurora purple
            channel_inactive: Color::Rgb(136, 192, 208),   // Frost cyan
            channel_adapter: Color::Rgb(235, 203, 139),    // Aurora yellow
            channel_other: Color::Rgb(76, 86, 106),        // Polar Night 3
            channel_empty: Color::Rgb(46, 52, 64),         // Polar Night 0

            selection_bg: Color::Rgb(59, 66, 82), // Polar Night 2
            selection_fg: Color::Rgb(236, 239, 244), // Snow Storm 0
            key_hint: Color::Rgb(235, 203, 139),  // Aurora yellow
            background: Color::Rgb(46, 52, 64),   // Polar Night 0
        }
    }

    /// High contrast neon theme
    pub fn neon() -> Self {
        Self {
            name: "Neon",

            border: Color::Rgb(0, 255, 255),           // Cyan
            border_dim: Color::Rgb(64, 64, 64),        // Gray
            border_highlight: Color::Rgb(255, 0, 255), // Magenta

            text: Color::Rgb(255, 255, 255),     // White
            text_dim: Color::Rgb(128, 128, 128), // Gray
            text_title: Color::Rgb(0, 255, 255), // Cyan

            success: Color::Rgb(0, 255, 0),   // Green
            warning: Color::Rgb(255, 255, 0), // Yellow
            error: Color::Rgb(255, 0, 0),     // Red
            info: Color::Rgb(0, 255, 255),    // Cyan
            idle: Color::Rgb(128, 128, 128),  // Gray
            special: Color::Rgb(255, 0, 255), // Magenta

            chart_line: Color::Rgb(0, 255, 255),   // Cyan
            chart_fill: Color::Rgb(0, 128, 255),   // Blue
            chart_passed: Color::Rgb(0, 255, 0),   // Green
            chart_failed: Color::Rgb(255, 0, 0),   // Red
            chart_axis: Color::Rgb(128, 128, 128), // Gray

            channel_sequencing: Color::Rgb(0, 255, 0), // Green
            channel_pore: Color::Rgb(0, 128, 255),     // Blue
            channel_unavailable: Color::Rgb(255, 0, 255), // Magenta
            channel_inactive: Color::Rgb(0, 255, 255), // Cyan
            channel_adapter: Color::Rgb(255, 255, 0),  // Yellow
            channel_other: Color::Rgb(128, 128, 128),  // Gray
            channel_empty: Color::Rgb(0, 0, 0),        // Black

            selection_bg: Color::Rgb(48, 48, 48),    // Dark gray
            selection_fg: Color::Rgb(255, 255, 255), // White
            key_hint: Color::Rgb(255, 255, 0),       // Yellow
            background: Color::Rgb(0, 0, 0),         // Black
        }
    }

    /// Get a theme by name
    pub fn by_name(name: &str) -> Option<Self> {
        match name.to_lowercase().as_str() {
            "default" => Some(Self::default_theme()),
            "catppuccin" | "catppuccin-mocha" | "catppuccin_mocha" => {
                Some(Self::catppuccin_mocha())
            }
            "dracula" => Some(Self::dracula()),
            "tokyo-night" | "tokyo_night" | "tokyonight" => Some(Self::tokyo_night()),
            "gruvbox" => Some(Self::gruvbox()),
            "nord" => Some(Self::nord()),
            "neon" => Some(Self::neon()),
            _ => None,
        }
    }

    /// List all available theme names
    pub fn available_themes() -> &'static [&'static str] {
        &[
            "default",
            "catppuccin",
            "dracula",
            "tokyo-night",
            "gruvbox",
            "nord",
            "neon",
        ]
    }
}
