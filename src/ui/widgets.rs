use crate::ui::canvas::{Canvas, SCREEN_HEIGHT, SCREEN_WIDTH};
use crate::ui::theme::ThemePalette;
use std::time::{SystemTime, UNIX_EPOCH};

pub struct Widgets;

impl Widgets {
    /// Modern top and bottom floating HUD for 4:3 1024x768 display
    pub fn draw_header(canvas: &mut Canvas, title: &str, palette: &ThemePalette) {
        // Subtle top bar
        canvas.draw_rect_alpha(0, 0, SCREEN_WIDTH as i32, 40, palette.hud_bg, 220);
        canvas.draw_horizontal_line(0, SCREEN_WIDTH as i32, 40, palette.border);

        // Title
        canvas.draw_text(title, 24, 8, 20.0, palette.hud_text);

        // System Time & Battery Simulation
        let now_str = get_current_time_string();
        let time_w = canvas.measure_text_width(&now_str, 20.0);
        canvas.draw_text(&now_str, (SCREEN_WIDTH as i32) - time_w - 90, 8, 20.0, palette.hud_text);

        // Battery icon & percentage
        let bat_x = (SCREEN_WIDTH as i32) - 70;
        canvas.draw_rounded_border(bat_x, 11, 40, 18, 3, palette.hud_text, 2);
        canvas.draw_rect(bat_x + 40, 15, 3, 10, palette.hud_text);
        canvas.draw_rect(bat_x + 3, 14, 28, 12, palette.accent); // ~80% battery
    }

    pub fn draw_footer_status(
        canvas: &mut Canvas,
        info_left: &str,
        progress_text: &str,
        percent: f32,
        palette: &ThemePalette,
    ) {
        let footer_y = (SCREEN_HEIGHT as i32) - 36;

        canvas.draw_rect_alpha(0, footer_y, SCREEN_WIDTH as i32, 36, palette.hud_bg, 220);
        canvas.draw_horizontal_line(0, SCREEN_WIDTH as i32, footer_y, palette.border);

        // Left info
        canvas.draw_text(info_left, 24, footer_y + 8, 18.0, palette.hud_text);

        // Right progress text
        let prog_w = canvas.measure_text_width(progress_text, 18.0);
        let prog_x = (SCREEN_WIDTH as i32) - prog_w - 24;
        canvas.draw_text(progress_text, prog_x, footer_y + 8, 18.0, palette.hud_text);

        // Mini progress bar in middle
        let bar_w = 200;
        let bar_x = (SCREEN_WIDTH as i32) / 2 - bar_w / 2;
        let bar_y = footer_y + 14;
        canvas.draw_rounded_rect(bar_x, bar_y, bar_w, 8, 4, palette.card_bg);
        let fill_w = ((bar_w as f32) * percent.clamp(0.0, 1.0)) as i32;
        if fill_w > 0 {
            canvas.draw_rounded_rect(bar_x, bar_y, fill_w, 8, 4, palette.accent);
        }
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

            // Key badge
            let badge_w = 40;
            let badge_x = ix + (iw - badge_w) / 2;
            canvas.draw_rounded_rect(badge_x, iy + 6, badge_w, 24, 6, palette.accent);
            canvas.draw_text(key_label, badge_x + 10, iy + 9, 16.0, 0xFFFFFF);

            // Item Name
            let name_w = canvas.measure_text_width(name, 18.0);
            let name_x = ix + (iw - name_w) / 2;
            let text_col = if i == selected_index { palette.accent } else { palette.text_primary };
            canvas.draw_text(name, name_x, iy + 36, 18.0, text_col);
        }
    }

    /// Minimap Navigator Overlay (漫画放大时的鹰眼小地图)
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
        let tw = canvas.measure_text_width(message, 20.0) + 48;
        let th = 48;
        let tx = (SCREEN_WIDTH as i32 - tw) / 2;
        let ty = 60;

        canvas.draw_rounded_rect_alpha(tx + 2, ty + 4, tw, th, 12, 0x000000, 80);
        canvas.draw_rounded_rect(tx, ty, tw, th, 12, palette.toast_bg);
        canvas.draw_rounded_border(tx, ty, tw, th, 12, palette.accent, 2);
        canvas.draw_text(message, tx + 24, ty + 12, 20.0, palette.toast_text);
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
