use crate::platform::battery::BatteryStatus;
use crate::ui::canvas::{Canvas, SCREEN_HEIGHT, SCREEN_WIDTH};
use crate::ui::theme::ThemePalette;
use std::time::{SystemTime, UNIX_EPOCH};

pub struct Widgets;

impl Widgets {
    /// Modern top and bottom floating HUD for 4:3 1024x768 display
    pub fn draw_header(
        canvas: &mut Canvas,
        title: &str,
        palette: &ThemePalette,
        battery: Option<BatteryStatus>,
    ) {
        // Subtle top bar
        canvas.draw_rect_alpha(0, 0, SCREEN_WIDTH as i32, 44, palette.hud_bg, 235);
        canvas.draw_horizontal_line(0, SCREEN_WIDTH as i32, 44, palette.border);

        // System time and real battery information. If sysfs cannot provide a
        // valid battery value, the entire battery area is intentionally hidden.
        let now_str = get_current_time_string();
        let time_w = canvas.measure_text_width(&now_str, 20.0);
        let right_margin = 22;
        let mut time_x = SCREEN_WIDTH as i32 - right_margin - time_w;

        if let Some(status) = battery {
            let battery_text = if status.charging {
                format!("⚡ {}%", status.percent)
            } else {
                format!("{}%", status.percent)
            };
            let text_w = canvas.measure_text_width(&battery_text, 18.0);
            let icon_w = 34;
            let icon_h = 17;
            let icon_x = SCREEN_WIDTH as i32 - right_margin - icon_w - 3;
            let icon_y = 13;
            let text_x = icon_x - text_w - 9;
            let battery_color = if status.percent <= 15 {
                0xC95548
            } else {
                palette.accent
            };

            canvas.draw_text(&battery_text, text_x, 10, 18.0, battery_color);
            canvas.draw_rounded_border(icon_x, icon_y, icon_w, icon_h, 3, palette.hud_text, 2);
            canvas.draw_rect(icon_x + icon_w, icon_y + 5, 3, 7, palette.hud_text);

            let inner_w = icon_w - 6;
            let fill_w = ((inner_w as f32) * status.percent as f32 / 100.0).round() as i32;
            if fill_w > 0 {
                canvas.draw_rounded_rect(
                    icon_x + 3,
                    icon_y + 3,
                    fill_w,
                    icon_h - 6,
                    2,
                    battery_color,
                );
            }

            time_x = text_x - time_w - 24;
        }

        canvas.draw_text(&now_str, time_x, 9, 20.0, palette.hud_text);

        // Keep long book/folder titles away from the system status area.
        let title_max_width = (time_x - 42).max(80);
        let fitted_title = Self::fit_text(canvas, title, title_max_width, 20.0);
        canvas.draw_text(&fitted_title, 24, 9, 20.0, palette.hud_text);
    }

    pub fn draw_footer_status(
        canvas: &mut Canvas,
        info_left: &str,
        progress_text: &str,
        percent: f32,
        palette: &ThemePalette,
    ) {
        let footer_y = (SCREEN_HEIGHT as i32) - 44;

        canvas.draw_rect_alpha(0, footer_y, SCREEN_WIDTH as i32, 44, palette.hud_bg, 235);
        canvas.draw_horizontal_line(0, SCREEN_WIDTH as i32, footer_y, palette.border);

        // Right progress text
        let prog_w = canvas.measure_text_width(progress_text, 18.0);
        let prog_x = (SCREEN_WIDTH as i32) - prog_w - 24;
        canvas.draw_text(progress_text, prog_x, footer_y + 6, 18.0, palette.hud_text);

        // Left information is shortened before it can overlap the page count.
        let info_max_width = (prog_x - 48).max(80);
        let fitted_info = Self::fit_text(canvas, info_left, info_max_width, 18.0);
        canvas.draw_text(&fitted_info, 24, footer_y + 6, 18.0, palette.hud_text);

        // Full-width progress track is easier to read at a glance.
        let bar_x = 24;
        let bar_w = SCREEN_WIDTH as i32 - 48;
        let bar_y = footer_y + 32;
        canvas.draw_rounded_rect(bar_x, bar_y, bar_w, 6, 3, palette.card_bg);
        let fill_w = ((bar_w as f32) * percent.clamp(0.0, 1.0)) as i32;
        if fill_w > 0 {
            canvas.draw_rounded_rect(bar_x, bar_y, fill_w, 6, 3, palette.accent);
        }
    }

    pub fn fit_text(canvas: &Canvas, text: &str, max_width: i32, size_px: f32) -> String {
        if max_width <= 0 || text.is_empty() {
            return String::new();
        }
        if canvas.measure_text_width(text, size_px) <= max_width {
            return text.to_string();
        }

        let ellipsis = "…";
        let ellipsis_width = canvas.measure_text_width(ellipsis, size_px);
        if ellipsis_width > max_width {
            return String::new();
        }

        let mut fitted = String::new();
        for ch in text.chars() {
            let mut candidate = fitted.clone();
            candidate.push(ch);
            candidate.push_str(ellipsis);
            if canvas.measure_text_width(&candidate, size_px) > max_width {
                break;
            }
            fitted.push(ch);
        }
        fitted.push_str(ellipsis);
        fitted
    }

    /// Modern Frosted Bottom Floating Dock (呼出的半透明磨砂控制底栏)
    pub fn draw_floating_menu(
        canvas: &mut Canvas,
        items: &[(&str, &str)],
        selected_index: usize,
        palette: &ThemePalette,
    ) {
        let dock_w = 880;
        let dock_h = 90;
        let dock_x = (SCREEN_WIDTH as i32 - dock_w) / 2;
        let dock_y = (SCREEN_HEIGHT as i32) - dock_h - 45;

        canvas.draw_rounded_rect_alpha(dock_x + 4, dock_y + 6, dock_w, dock_h, 16, 0x000000, 70);
        canvas.draw_rounded_rect_alpha(dock_x, dock_y, dock_w, dock_h, 16, palette.card_bg, 245);
        canvas.draw_rounded_border(dock_x, dock_y, dock_w, dock_h, 16, palette.accent, 2);

        let item_w = dock_w / (items.len() as i32);
        for (i, (key_label, name)) in items.iter().enumerate() {
            let ix = dock_x + (i as i32) * item_w + 10;
            let iy = dock_y + 12;
            let iw = item_w - 20;
            let ih = dock_h - 24;

            if i == selected_index {
                canvas.draw_rounded_rect(ix, iy, iw, ih, 10, palette.card_selected_bg);
                canvas.draw_rounded_border(ix, iy, iw, ih, 10, palette.accent, 2);
            }

            // Key badge adapts to labels such as L2/R2 and SEL/ST.
            let key_w = canvas.measure_text_width(key_label, 16.0);
            let badge_w = (key_w + 20).clamp(40, iw - 12);
            let badge_x = ix + (iw - badge_w) / 2;
            canvas.draw_rounded_rect(badge_x, iy + 6, badge_w, 24, 6, palette.accent);
            canvas.draw_text(
                key_label,
                badge_x + (badge_w - key_w) / 2,
                iy + 9,
                16.0,
                0xFFFFFF,
            );

            // Item Name
            let fitted_name = Self::fit_text(canvas, name, iw - 12, 18.0);
            let name_w = canvas.measure_text_width(&fitted_name, 18.0);
            let name_x = ix + (iw - name_w) / 2;
            let text_col = if i == selected_index {
                palette.accent
            } else {
                palette.text_primary
            };
            canvas.draw_text(&fitted_name, name_x, iy + 36, 18.0, text_col);
        }
    }

    /// Minimap Navigator Overlay (漫画放大时的鹰眼小地图)
    #[allow(clippy::too_many_arguments)]
    pub fn draw_minimap(
        canvas: &mut Canvas,
        img_w: f32,
        img_h: f32,
        pan_x: f32,
        pan_y: f32,
        view_w: f32,
        view_h: f32,
        palette: &ThemePalette,
    ) {
        let map_w = 160;
        let map_h = ((160.0 * (img_h / img_w)).min(140.0)) as i32;
        let map_x = (SCREEN_WIDTH as i32) - map_w - 24;
        let map_y = 55;

        canvas.draw_rounded_rect_alpha(map_x, map_y, map_w, map_h, 8, palette.minimap_bg, 210);
        canvas.draw_rounded_border(map_x, map_y, map_w, map_h, 8, palette.minimap_border, 2);

        let norm_x = (pan_x / img_w).clamp(0.0, 1.0);
        let norm_y = (pan_y / img_h).clamp(0.0, 1.0);
        let norm_w = (view_w / img_w).clamp(0.0, 1.0);
        let norm_h = (view_h / img_h).clamp(0.0, 1.0);

        let vx = map_x + (norm_x * map_w as f32) as i32;
        let vy = map_y + (norm_y * map_h as f32) as i32;
        let vw = ((norm_w * map_w as f32) as i32).max(8).min(map_w);
        let vh = ((norm_h * map_h as f32) as i32).max(8).min(map_h);

        canvas.draw_rect_alpha(vx, vy, vw, vh, palette.minimap_viewport, 100);
        canvas.draw_rounded_border(vx, vy, vw, vh, 2, palette.minimap_border, 2);
    }

    /// Modern Toast Popup
    pub fn draw_toast(canvas: &mut Canvas, message: &str, palette: &ThemePalette) {
        let message = Self::fit_text(canvas, message, SCREEN_WIDTH as i32 - 96, 20.0);
        let tw = canvas.measure_text_width(&message, 20.0) + 48;
        let th = 48;
        let tx = (SCREEN_WIDTH as i32 - tw) / 2;
        let ty = 60;

        canvas.draw_rounded_rect_alpha(tx + 2, ty + 4, tw, th, 12, 0x000000, 80);
        canvas.draw_rounded_rect(tx, ty, tw, th, 12, palette.toast_bg);
        canvas.draw_rounded_border(tx, ty, tw, th, 12, palette.accent, 2);
        canvas.draw_text(&message, tx + 24, ty + 12, 20.0, palette.toast_text);
    }
}

fn get_current_time_string() -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    // UTC+8 offset (China Standard Time) or local offset approximation
    let local_secs = secs + 8 * 3600;
    let hours = (local_secs / 3600) % 24;
    let minutes = (local_secs / 60) % 60;
    format!("{:02}:{:02}", hours, minutes)
}
