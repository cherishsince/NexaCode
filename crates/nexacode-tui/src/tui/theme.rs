//! Theme system

use nexacode_core::Theme as CoreTheme;
use ratatui::style::{Color, Style, Stylize};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Theme {
    Dark,
    Light,
}

impl Default for Theme {
    fn default() -> Self {
        Self::Light
    }
}

impl From<CoreTheme> for Theme {
    fn from(theme: CoreTheme) -> Self {
        match theme {
            CoreTheme::Dark => Theme::Dark,
            CoreTheme::Light => Theme::Light,
        }
    }
}

impl Theme {
    pub fn background(&self) -> Color {
        match self {
            Self::Dark => Color::from_u32(0x0A0A0A),
            Self::Light => Color::from_u32(0xFAFAFA),
        }
    }

    pub fn foreground(&self) -> Color {
        match self {
            Self::Dark => Color::from_u32(0xFAFAFA),
            Self::Light => Color::from_u32(0x111827),
        }
    }

    pub fn primary(&self) -> Color {
        match self {
            Self::Dark => Color::from_u32(0x10B981),
            Self::Light => Color::from_u32(0x059669),
        }
    }

    pub fn secondary(&self) -> Color {
        match self {
            Self::Dark => Color::from_u32(0x6B7280),
            Self::Light => Color::from_u32(0x6B7280),
        }
    }

    pub fn warning(&self) -> Color {
        match self {
            Self::Dark => Color::from_u32(0xF59E0B),
            Self::Light => Color::from_u32(0xD97706),
        }
    }

    pub fn info(&self) -> Color {
        match self {
            Self::Dark => Color::from_u32(0x06B6D4),
            Self::Light => Color::from_u32(0x0891B2),
        }
    }

    pub fn purple(&self) -> Color {
        match self {
            Self::Dark => Color::from_u32(0xA855F7),
            Self::Light => Color::from_u32(0x9333EA),
        }
    }

    pub fn border(&self) -> Color {
        match self {
            Self::Dark => Color::from_u32(0x374151),
            Self::Light => Color::from_u32(0xE5E7EB),
        }
    }

    pub fn base_style(&self) -> Style {
        Style::default().fg(self.foreground()).bg(self.background())
    }

    pub fn title_style(&self) -> Style {
        Style::default().fg(self.primary()).bold()
    }

    pub fn prompt_style(&self) -> Style {
        Style::default().fg(self.primary()).bold()
    }

    pub fn comment_style(&self) -> Style {
        Style::default().fg(self.secondary())
    }
}
