#![allow(dead_code)]

mod app;
mod config;
mod core;
mod input;
mod platform;
mod ui;

pub const APP_DISPLAY_NAME: &str = concat!("tbb", env!("CARGO_PKG_VERSION"));

use app::App;
use std::time::{Duration, Instant};
use ui::canvas::{SCREEN_HEIGHT, SCREEN_WIDTH};

fn main() {
    println!("======================================================");
    println!("  {} - Trimui Brick 4:3 掌机阅读器", APP_DISPLAY_NAME);
    println!("  分辨率: 1024x768 (4:3 视窗)");
    println!("======================================================");

    #[cfg(target_os = "linux")]
    if let Err(error) = run_linux_framebuffer() {
        eprintln!("❌ {} 启动失败: {error}", APP_DISPLAY_NAME);
        std::process::exit(1);
    }

    #[cfg(not(target_os = "linux"))]
    run_desktop_simulator();
}

#[cfg(target_os = "linux")]
fn run_linux_framebuffer() -> Result<(), String> {
    use platform::evdev::LinuxInputManager;
    use platform::fb::FramebufferDisplay;

    println!("==> 正在启动 Linux 原生 Framebuffer & Evdev 后端...");

    let mut display = FramebufferDisplay::open_default()?;

    let mut input_manager = LinuxInputManager::new();
    let mut app = App::new();
    let mut last_frame = Instant::now();
    let target_frame_duration = Duration::from_micros(16666); // ~60 FPS

    println!("✓ {} 原生运行就绪，进入主循环", APP_DISPLAY_NAME);

    while !app.should_exit {
        let frame_start = Instant::now();
        let dt = (frame_start - last_frame).as_secs_f32().min(0.1);
        last_frame = frame_start;

        // Poll physical keys via evdev
        let input_state = input_manager.update();

        // Update logic & Render
        app.update(&input_state, dt);
        app.render();

        // Present to /dev/fb0
        display.present(&app.canvas.buffer, SCREEN_WIDTH, SCREEN_HEIGHT);

        // Sleep to maintain steady 60 FPS and conserve handheld battery
        let elapsed = frame_start.elapsed();
        if elapsed < target_frame_duration {
            std::thread::sleep(target_frame_duration - elapsed);
        }
    }

    println!("==> {} 正常退出", APP_DISPLAY_NAME);
    Ok(())
}

#[cfg(not(target_os = "linux"))]
fn run_desktop_simulator() {
    use input::InputManager;
    use minifb::{Window, WindowOptions};

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
    println!("  Tab / Enter     : 模拟 Select / Start 键 (文本章节目录)");
    println!("  F / F1 键       : 模拟掌机 FN 键 (刷新书库 / 智能切白边)");
    println!("======================================================");

    let mut window = match Window::new(
        &format!(
            "{} - Trimui Brick 4:3 Simulator (1024x768)",
            APP_DISPLAY_NAME
        ),
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

    while window.is_open() && !window.is_key_down(minifb::Key::End) && !app.should_exit {
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
