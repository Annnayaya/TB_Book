use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ThemeMode {
    PaperSepia,   // 羊皮纸暖白 (日间)
    OledDark,     // 深空暗黑 (夜间)
    BambooSage,   // 竹青雅致 (护眼)
    MangaPureDark,// 纯黑漫画底色
}

#[derive(Debug, Clone, Copy)]
pub struct ThemePalette {
    pub background: u32,
    pub card_bg: u32,
    pub card_selected_bg: u32,
    pub text_primary: u32,
    pub text_secondary: u32,
    pub text_muted: u32,
    pub accent: u32,
    pub border: u32,
    pub hud_bg: u32,
    pub hud_text: u32,
    pub toast_bg: u32,
    pub toast_text: u32,
    pub minimap_bg: u32,
    pub minimap_border: u32,
    pub minimap_viewport: u32,
}

impl ThemeMode {
    pub fn palette(&self) -> ThemePalette {
        match self {
            ThemeMode::PaperSepia => ThemePalette {
                background: 0xF5F0E6,        // 暖米白
                card_bg: 0xEAE2D5,           // 浅卡片
                card_selected_bg: 0xDCD1C0,  // 高亮卡片
                text_primary: 0x2D2B28,      // 炭黑主字
                text_secondary: 0x6B655D,    // 次级文字
                text_muted: 0x9E978E,        // 暗淡文字
                accent: 0xB86A34,            // 暖橙琥珀主色
                border: 0xD5C9B5,            // 边框
                hud_bg: 0xE2D8C7,            // HUD
                hud_text: 0x33302C,
                toast_bg: 0x2E2A25,
                toast_text: 0xF5F0E6,
                minimap_bg: 0x802E2A25,
                minimap_border: 0xB86A34,
                minimap_viewport: 0xE0B86A34,
            },
            ThemeMode::OledDark => ThemePalette {
                background: 0x121316,        // 深灰黑
                card_bg: 0x1C1E24,           // 卡片背景
                card_selected_bg: 0x282C37,  // 选中背景
                text_primary: 0xD6D9DE,      // 浅灰亮白字
                text_secondary: 0x8E94A0,    // 次级灰
                text_muted: 0x565B66,        // 辅助灰
                accent: 0x4A88E8,            // 极客蓝
                border: 0x2B2F3B,
                hud_bg: 0x1A1C22,
                hud_text: 0xC8CCD4,
                toast_bg: 0x2A2D36,
                toast_text: 0xFFFFFF,
                minimap_bg: 0xA0101114,
                minimap_border: 0x4A88E8,
                minimap_viewport: 0xE04A88E8,
            },
            ThemeMode::BambooSage => ThemePalette {
                background: 0xEEF3EE,        // 淡青玉色
                card_bg: 0xDFE7DF,
                card_selected_bg: 0xCDDBCD,
                text_primary: 0x1F2A22,      // 墨黛字
                text_secondary: 0x4E6053,
                text_muted: 0x829487,
                accent: 0x3B774E,            // 苍翠绿
                border: 0xC5D5C5,
                hud_bg: 0xD6E2D6,
                hud_text: 0x1F2A22,
                toast_bg: 0x233327,
                toast_text: 0xEEF3EE,
                minimap_bg: 0x80233327,
                minimap_border: 0x3B774E,
                minimap_viewport: 0xE03B774E,
            },
            ThemeMode::MangaPureDark => ThemePalette {
                background: 0x080809,        // 纯黑
                card_bg: 0x141416,
                card_selected_bg: 0x222228,
                text_primary: 0xEAEAEA,
                text_secondary: 0x999999,
                text_muted: 0x555555,
                accent: 0xE53935,            // 经典红
                border: 0x24242A,
                hud_bg: 0x121215,
                hud_text: 0xE0E0E0,
                toast_bg: 0x1F1F24,
                toast_text: 0xFFFFFF,
                minimap_bg: 0x90000000,
                minimap_border: 0xE53935,
                minimap_viewport: 0xE0E53935,
            },
        }
    }

    pub fn next(&self) -> Self {
        match self {
            ThemeMode::PaperSepia => ThemeMode::OledDark,
            ThemeMode::OledDark => ThemeMode::BambooSage,
            ThemeMode::BambooSage => ThemeMode::MangaPureDark,
            ThemeMode::MangaPureDark => ThemeMode::PaperSepia,
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            ThemeMode::PaperSepia => "羊皮纸 (暖白)",
            ThemeMode::OledDark => "深空黑 (夜间)",
            ThemeMode::BambooSage => "竹青雅 (护眼)",
            ThemeMode::MangaPureDark => "纯黑 (漫画)",
        }
    }
}
