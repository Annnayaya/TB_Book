use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ThemeMode {
    PaperSepia,    // 羊皮纸暖白 (日间)
    RicePaper,     // 柔和宣纸灰 (低对比)
    BambooSage,    // 竹青雅致 (护眼)
    ForestNight,   // 松墨深绿 (暗色护眼)
    AmberNight,    // 琥珀暖夜 (低蓝光)
    OledDark,      // 深空暗黑 (夜间)
    MangaPureDark, // 纯黑漫画底色
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
                background: 0xF5F0E6,       // 暖米白
                card_bg: 0xEAE2D5,          // 浅卡片
                card_selected_bg: 0xDCD1C0, // 高亮卡片
                text_primary: 0x2D2B28,     // 炭黑主字
                text_secondary: 0x6B655D,   // 次级文字
                text_muted: 0x9E978E,       // 暗淡文字
                accent: 0xB86A34,           // 暖橙琥珀主色
                border: 0xD5C9B5,           // 边框
                hud_bg: 0xE2D8C7,           // HUD
                hud_text: 0x33302C,
                toast_bg: 0x2E2A25,
                toast_text: 0xF5F0E6,
                minimap_bg: 0x802E2A25,
                minimap_border: 0xB86A34,
                minimap_viewport: 0xE0B86A34,
            },
            ThemeMode::RicePaper => ThemePalette {
                background: 0xF2F0E7,
                card_bg: 0xE6E1D3,
                card_selected_bg: 0xD8D2C1,
                text_primary: 0x30322D,
                text_secondary: 0x66695E,
                text_muted: 0x8B8D82,
                accent: 0x727A4F,
                border: 0xCEC8B8,
                hud_bg: 0xE0DCCF,
                hud_text: 0x30322D,
                toast_bg: 0x30342A,
                toast_text: 0xF2F0E7,
                minimap_bg: 0x8030342A,
                minimap_border: 0x727A4F,
                minimap_viewport: 0xE0727A4F,
            },
            ThemeMode::OledDark => ThemePalette {
                background: 0x121316,       // 深灰黑
                card_bg: 0x1C1E24,          // 卡片背景
                card_selected_bg: 0x282C37, // 选中背景
                text_primary: 0xD6D9DE,     // 浅灰亮白字
                text_secondary: 0x8E94A0,   // 次级灰
                text_muted: 0x565B66,       // 辅助灰
                accent: 0x4A88E8,           // 极客蓝
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
                background: 0xEEF3EE, // 淡青玉色
                card_bg: 0xDFE7DF,
                card_selected_bg: 0xCDDBCD,
                text_primary: 0x1F2A22, // 墨黛字
                text_secondary: 0x4E6053,
                text_muted: 0x829487,
                accent: 0x3B774E, // 苍翠绿
                border: 0xC5D5C5,
                hud_bg: 0xD6E2D6,
                hud_text: 0x1F2A22,
                toast_bg: 0x233327,
                toast_text: 0xEEF3EE,
                minimap_bg: 0x80233327,
                minimap_border: 0x3B774E,
                minimap_viewport: 0xE03B774E,
            },
            ThemeMode::ForestNight => ThemePalette {
                background: 0x101814,
                card_bg: 0x18231D,
                card_selected_bg: 0x23342A,
                text_primary: 0xD1DBC7,
                text_secondary: 0x91A18F,
                text_muted: 0x607064,
                accent: 0x76A37B,
                border: 0x2D4034,
                hud_bg: 0x141F19,
                hud_text: 0xD1DBC7,
                toast_bg: 0x26382D,
                toast_text: 0xE5EDDF,
                minimap_bg: 0x90101814,
                minimap_border: 0x76A37B,
                minimap_viewport: 0xE076A37B,
            },
            ThemeMode::AmberNight => ThemePalette {
                background: 0x18130E,
                card_bg: 0x241C15,
                card_selected_bg: 0x33271D,
                text_primary: 0xDBC9AA,
                text_secondary: 0xA99472,
                text_muted: 0x71614D,
                accent: 0xC58A42,
                border: 0x3D3023,
                hud_bg: 0x201810,
                hud_text: 0xDBC9AA,
                toast_bg: 0x3A2A1C,
                toast_text: 0xF1DFC0,
                minimap_bg: 0x9018130E,
                minimap_border: 0xC58A42,
                minimap_viewport: 0xE0C58A42,
            },
            ThemeMode::MangaPureDark => ThemePalette {
                background: 0x080809, // 纯黑
                card_bg: 0x141416,
                card_selected_bg: 0x222228,
                text_primary: 0xEAEAEA,
                text_secondary: 0x999999,
                text_muted: 0x555555,
                accent: 0xE53935, // 经典红
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
            ThemeMode::PaperSepia => ThemeMode::RicePaper,
            ThemeMode::RicePaper => ThemeMode::BambooSage,
            ThemeMode::BambooSage => ThemeMode::ForestNight,
            ThemeMode::ForestNight => ThemeMode::AmberNight,
            ThemeMode::AmberNight => ThemeMode::OledDark,
            ThemeMode::OledDark => ThemeMode::MangaPureDark,
            ThemeMode::MangaPureDark => ThemeMode::PaperSepia,
        }
    }

    pub fn previous(&self) -> Self {
        match self {
            ThemeMode::PaperSepia => ThemeMode::MangaPureDark,
            ThemeMode::RicePaper => ThemeMode::PaperSepia,
            ThemeMode::BambooSage => ThemeMode::RicePaper,
            ThemeMode::ForestNight => ThemeMode::BambooSage,
            ThemeMode::AmberNight => ThemeMode::ForestNight,
            ThemeMode::OledDark => ThemeMode::AmberNight,
            ThemeMode::MangaPureDark => ThemeMode::OledDark,
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            ThemeMode::PaperSepia => "羊皮纸 (暖白)",
            ThemeMode::RicePaper => "宣纸灰 (柔和)",
            ThemeMode::BambooSage => "竹青雅 (护眼)",
            ThemeMode::ForestNight => "松墨绿 (暗色护眼)",
            ThemeMode::AmberNight => "琥珀夜 (低蓝光)",
            ThemeMode::OledDark => "深空黑 (夜间)",
            ThemeMode::MangaPureDark => "纯黑 (漫画)",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::ThemeMode;

    #[test]
    fn next_and_previous_are_inverse_for_every_theme() {
        let themes = [
            ThemeMode::PaperSepia,
            ThemeMode::RicePaper,
            ThemeMode::BambooSage,
            ThemeMode::ForestNight,
            ThemeMode::AmberNight,
            ThemeMode::OledDark,
            ThemeMode::MangaPureDark,
        ];

        for theme in themes {
            assert_eq!(theme.next().previous(), theme);
            assert_eq!(theme.previous().next(), theme);
        }
    }
}
