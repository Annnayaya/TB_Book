#![allow(dead_code)]

mod app;
mod config;
mod core;
mod input;
mod platform;
mod ui;

use app::App;
use input::InputManager;
use minifb::{Window, WindowOptions};
use std::time::Instant;
use ui::canvas::{SCREEN_HEIGHT, SCREEN_WIDTH};

fn main() {
    println!("======================================================");
    println!("  BrickReader - Trimui Brick 4:3 掌机阅读器");
    println!("  分辨率: 1024x768 (4:3 视窗)");
    println!("======================================================");
    println!("  [PC 模拟器操作快捷键]");
    println!("  方向键 ↑ ↓ ← → : 模拟掌机 十字键 (D-Pad)");
    println!("  J / 空格键      : 模拟掌机 A 键 (确认 / 翻页 / 进入目录)");
    println!("  K / Backspace   : 模拟掌机 B 键 (返回 / 取消 / 保存返回)");
    println!("  U / X 键        : 模拟掌机 X 键 (一键还原 1.0x 全屏 / 设为书库目录)");
    println!("  I / Y 键        : 模拟掌机 Y 键 (切换日漫RTL/国漫LTR / 切换主题)");
    println!("  Q / PageUp      : 模拟掌机 L1 键 (上一页)");
    println!("  E / PageDown    : 模拟掌机 R1 键 (下一页)");
    println!("  1 / [ 键        : 模拟掌机 L2 键 (平滑缩小 / 字号减小)");
    println!("  3 / ] 键        : 模拟掌机 R2 键 (平滑放大 / 字号增大)");
    println!("  ESC / M 键      : 模拟掌机 Menu 键 (呼出设置中心 / 控制底栏)");
    println!("  F / F1 键       : 模拟掌机 FN 键 (刷新书库 / 智能切白边)");
    println!("======================================================");

    let mut window = match Window::new(
        "BrickReader - Trimui Brick 4:3 Simulator (1024x768)",
        SCREEN_WIDTH,
        SCREEN_HEIGHT,
        WindowOptions {
            resize: false,
            scale: minifb::Scale::X1,
            ..WindowOptions::default()
        },
    ) {
        Ok(win) => win,
        Err(err) => {
            eprintln!("无法创建模拟器图形窗口: {:?}", err);
            return;
        }
    };

    window.set_target_fps(60);

    let mut app = App::new();
    let mut input_manager = InputManager::new();
    let mut last_frame = Instant::now();

    while window.is_open() && !window.is_key_down(minifb::Key::End) {
        let now = Instant::now();
        let dt = (now - last_frame).as_secs_f32();
        last_frame = now;

        // Poll keys
        let keys = window.get_keys();
        let input_state = input_manager.update(&keys);

        // Update logic & Render
        app.update(&input_state, dt);
        app.render();

        // Update Framebuffer
        if let Err(e) = window.update_with_buffer(&app.canvas.buffer, SCREEN_WIDTH, SCREEN_HEIGHT) {
            eprintln!("帧缓冲更新错误: {:?}", e);
            break;
        }
    }
}
