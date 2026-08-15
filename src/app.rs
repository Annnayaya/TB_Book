use crate::config::AppSettings;
use crate::core::charset::CharsetHelper;
use crate::core::comic_engine::{ComicArchive, ReadingDirection, ZoomMode};
use crate::core::library::{BookMetadata, BookType, LibraryDatabase};
use crate::core::text_engine::TextBook;
use crate::input::{HandheldButton, InputState};
use crate::platform::led::LedController;
use crate::platform::rumble::RumbleController;
use crate::ui::canvas::{Canvas, SCREEN_HEIGHT, SCREEN_WIDTH};
use crate::ui::comic_view::ComicView;
use crate::ui::settings_view::{SettingsTab, SettingsView};
use crate::ui::shelf_view::ShelfView;
use crate::ui::text_view::TextView;
use crate::ui::widgets::Widgets;
use std::fs;

pub enum AppScreen {
    Shelf,
    ReadingText,
    ReadingComic,
    Settings,
}

pub struct App {
    pub screen: AppScreen,
    pub canvas: Canvas,
    pub settings: AppSettings,
    pub library: LibraryDatabase,
    pub books: Vec<BookMetadata>,

    pub shelf_view: ShelfView,
    pub text_view: TextView,
    pub comic_view: ComicView,
    pub settings_view: SettingsView,

    pub text_book: Option<TextBook>,
    pub comic_book: Option<ComicArchive>,

    pub toast_message: Option<String>,
    pub toast_timer: f32,
}

impl App {
    pub fn new() -> Self {
        let canvas = Canvas::new();
        let settings = AppSettings::load();
        let library = LibraryDatabase::load();
        let settings_view = SettingsView::new(&settings.library_path);

        let mut app = Self {
            screen: AppScreen::Shelf,
            canvas,
            settings: settings.clone(),
            library,
            books: Vec::new(),
            shelf_view: ShelfView::new(),
            text_view: TextView::new(),
            comic_view: ComicView::new(),
            settings_view,
            text_book: None,
            comic_book: None,
            toast_message: None,
            toast_timer: 0.0,
        };

        app.refresh_books();
        app.update_rgb_led();
        app
    }

    pub fn refresh_books(&mut self) {
        let mut scan_paths = vec![self.settings.library_path.clone()];

        // Also add fallback paths if not already scanning them
        for fallback in &["books", "assets/samples", "/mnt/SDCARD/Roms/BOOKS"] {
            let pb = std::path::PathBuf::from(fallback);
            if !scan_paths.contains(&pb) && pb.exists() {
                scan_paths.push(pb);
            }
        }

        let mut all_books = Vec::new();
        for p in scan_paths {
            let found = LibraryDatabase::scan_books(p);
            all_books.extend(found);
        }

        // Apply saved reading progress
        for book in &mut all_books {
            let key = book.path.to_string_lossy().to_string();
            if let Some(hist) = self.library.history.get(&key) {
                book.current_page = hist.current_page;
                book.total_pages = hist.total_pages;
                book.percent = hist.percent;
                book.zoom_level = hist.zoom_level;
                book.last_read_timestamp = hist.last_read_timestamp;
            }
        }

        self.books = all_books;
        if self.shelf_view.selected_index >= self.books.len() {
            self.shelf_view.selected_index = self.books.len().saturating_sub(1);
        }
    }

    pub fn show_toast<S: Into<String>>(&mut self, msg: S) {
        self.toast_message = Some(msg.into());
        self.toast_timer = 2.0;
    }

    pub fn update_rgb_led(&self) {
        if !self.settings.rgb_led_enabled {
            LedController::turn_off();
            return;
        }

        match self.settings.theme {
            crate::ui::theme::ThemeMode::PaperSepia => LedController::set_rgb(180, 110, 50),
            crate::ui::theme::ThemeMode::OledDark => LedController::set_rgb(20, 40, 100),
            crate::ui::theme::ThemeMode::BambooSage => LedController::set_rgb(40, 120, 60),
            crate::ui::theme::ThemeMode::MangaPureDark => LedController::turn_off(),
        }
    }

    pub fn update(&mut self, input: &InputState, dt: f32) {
        if self.toast_timer > 0.0 {
            self.toast_timer -= dt;
            if self.toast_timer <= 0.0 {
                self.toast_message = None;
            }
        }

        if let Some(comic) = &mut self.comic_book {
            if comic.minimap_timer > 0.0 {
                comic.minimap_timer -= dt;
            }
        }

        match self.screen {
            AppScreen::Shelf => self.handle_shelf_input(input),
            AppScreen::ReadingText => self.handle_text_input(input),
            AppScreen::ReadingComic => self.handle_comic_input(input, dt),
            AppScreen::Settings => self.handle_settings_input(input),
        }
    }

    fn handle_shelf_input(&mut self, input: &InputState) {
        let cols = 3;
        let count = self.books.len();

        if input.is_pressed(HandheldButton::DpadRight) {
            if count > 0 && self.shelf_view.selected_index + 1 < count {
                self.shelf_view.selected_index += 1;
            }
        } else if input.is_pressed(HandheldButton::DpadLeft) {
            if self.shelf_view.selected_index > 0 {
                self.shelf_view.selected_index -= 1;
            }
        } else if input.is_pressed(HandheldButton::DpadDown) {
            if self.shelf_view.selected_index + cols < count {
                self.shelf_view.selected_index += cols;
            }
        } else if input.is_pressed(HandheldButton::DpadUp) {
            if self.shelf_view.selected_index >= cols {
                self.shelf_view.selected_index -= cols;
            }
        } else if input.is_pressed(HandheldButton::ButtonA) {
            self.open_selected_book();
        } else if input.is_pressed(HandheldButton::Menu) || input.is_pressed(HandheldButton::Select) {
            self.settings_view.tab = SettingsTab::General;
            self.settings_view.browse_path = self.settings.library_path.clone();
            self.settings_view.refresh_folder_entries();
            self.screen = AppScreen::Settings;
            RumbleController::pulse(20);
        } else if input.is_pressed(HandheldButton::ButtonY) {
            self.settings.theme = self.settings.theme.next();
            self.settings.save();
            self.update_rgb_led();
            self.show_toast(format!("主题: {}", self.settings.theme.name()));
        } else if input.is_pressed(HandheldButton::FnKey) {
            self.refresh_books();
            self.show_toast("已刷新书架文件");
        }
    }

    fn handle_settings_input(&mut self, input: &InputState) {
        match self.settings_view.tab {
            SettingsTab::General => {
                let max_items = 7;
                if input.is_pressed(HandheldButton::DpadDown) {
                    self.settings_view.selected_index = (self.settings_view.selected_index + 1) % max_items;
                } else if input.is_pressed(HandheldButton::DpadUp) {
                    self.settings_view.selected_index = (self.settings_view.selected_index + max_items - 1) % max_items;
                } else if input.is_pressed(HandheldButton::DpadLeft) || input.is_pressed(HandheldButton::L2) {
                    match self.settings_view.selected_index {
                        1 => {
                            // Font size -2
                            self.settings.font_size = (self.settings.font_size - 2.0).max(18.0);
                            self.settings.save();
                        }
                        2 => {
                            // Line spacing -0.1
                            self.settings.line_spacing = (self.settings.line_spacing - 0.1).max(1.1);
                            self.settings.save();
                        }
                        3 => {
                            // Theme prev
                            self.settings.theme = self.settings.theme.next();
                            self.settings.save();
                            self.update_rgb_led();
                        }
                        4 => {
                            // Manga direction
                            self.settings.default_reading_direction = match self.settings.default_reading_direction {
                                ReadingDirection::RightToLeft => ReadingDirection::LeftToRight,
                                ReadingDirection::LeftToRight => ReadingDirection::RightToLeft,
                            };
                            self.settings.save();
                        }
                        5 => {
                            // RGB LED
                            self.settings.rgb_led_enabled = !self.settings.rgb_led_enabled;
                            self.settings.save();
                            self.update_rgb_led();
                        }
                        _ => {}
                    }
                } else if input.is_pressed(HandheldButton::DpadRight) || input.is_pressed(HandheldButton::R2) {
                    match self.settings_view.selected_index {
                        1 => {
                            // Font size +2
                            self.settings.font_size = (self.settings.font_size + 2.0).min(48.0);
                            self.settings.save();
                        }
                        2 => {
                            // Line spacing +0.1
                            self.settings.line_spacing = (self.settings.line_spacing + 0.1).min(2.2);
                            self.settings.save();
                        }
                        3 => {
                            // Theme next
                            self.settings.theme = self.settings.theme.next();
                            self.settings.save();
                            self.update_rgb_led();
                        }
                        4 => {
                            // Manga direction
                            self.settings.default_reading_direction = match self.settings.default_reading_direction {
                                ReadingDirection::RightToLeft => ReadingDirection::LeftToRight,
                                ReadingDirection::LeftToRight => ReadingDirection::RightToLeft,
                            };
                            self.settings.save();
                        }
                        5 => {
                            // RGB LED
                            self.settings.rgb_led_enabled = !self.settings.rgb_led_enabled;
                            self.settings.save();
                            self.update_rgb_led();
                        }
                        _ => {}
                    }
                } else if input.is_pressed(HandheldButton::ButtonA) {
                    match self.settings_view.selected_index {
                        0 => {
                            // Open Folder Picker
                            self.settings_view.tab = SettingsTab::FolderPicker;
                            self.settings_view.browse_path = self.settings.library_path.clone();
                            self.settings_view.refresh_folder_entries();
                        }
                        6 => {
                            // Save & Exit
                            self.settings.save();
                            self.refresh_books();
                            self.screen = AppScreen::Shelf;
                            self.show_toast("设置已保存");
                        }
                        _ => {}
                    }
                } else if input.is_pressed(HandheldButton::ButtonB) {
                    self.settings.save();
                    self.refresh_books();
                    self.screen = AppScreen::Shelf;
                }
            }
            SettingsTab::FolderPicker => {
                let count = self.settings_view.folder_entries.len();
                if input.is_pressed(HandheldButton::DpadDown) {
                    if count > 0 && self.settings_view.folder_selected_index + 1 < count {
                        self.settings_view.folder_selected_index += 1;
                    }
                } else if input.is_pressed(HandheldButton::DpadUp) {
                    if self.settings_view.folder_selected_index > 0 {
                        self.settings_view.folder_selected_index -= 1;
                    }
                } else if input.is_pressed(HandheldButton::ButtonA) {
                    // Enter folder
                    if self.settings_view.folder_selected_index < count {
                        let selected_path = self.settings_view.folder_entries[self.settings_view.folder_selected_index].clone();
                        self.settings_view.browse_path = selected_path;
                        self.settings_view.folder_selected_index = 0;
                        self.settings_view.refresh_folder_entries();
                    }
                } else if input.is_pressed(HandheldButton::ButtonX) || input.is_pressed(HandheldButton::Start) {
                    // Choose current browse path as library path!
                    let chosen = self.settings_view.browse_path.clone();
                    self.settings.set_library_path(&chosen);
                    self.refresh_books();
                    self.settings_view.tab = SettingsTab::General;
                    self.show_toast(format!("已设为书库: {}", chosen.display()));
                    RumbleController::pulse(25);
                } else if input.is_pressed(HandheldButton::ButtonB) {
                    self.settings_view.tab = SettingsTab::General;
                }
            }
        }
    }

    fn open_selected_book(&mut self) {
        if self.shelf_view.selected_index >= self.books.len() {
            return;
        }

        let meta = self.books[self.shelf_view.selected_index].clone();
        match meta.book_type {
            BookType::Text => {
                if let Ok(bytes) = fs::read(&meta.path) {
                    let (content, enc_name) = CharsetHelper::decode_bytes(&bytes);
                    let mut book = TextBook::from_string(meta.title.clone(), content, enc_name.to_string());
                    book.settings.font_size = self.settings.font_size;
                    book.settings.line_spacing = self.settings.line_spacing;
                    book.settings.margin_x = self.settings.margin_x;
                    book.settings.margin_y = self.settings.margin_y;

                    book.repaginate(&self.canvas);
                    book.jump_to_page(meta.current_page);

                    self.text_book = Some(book);
                    self.screen = AppScreen::ReadingText;
                    RumbleController::pulse(20);
                    self.show_toast(format!("正在阅读: {}", meta.title));
                } else {
                    self.show_toast("无法打开文本文件");
                }
            }
            BookType::Comic => {
                match ComicArchive::open(&meta.path) {
                    Ok(mut comic) => {
                        comic.reading_direction = self.settings.default_reading_direction;
                        comic.jump_page(meta.current_page);
                        self.comic_book = Some(comic);
                        self.screen = AppScreen::ReadingComic;
                        RumbleController::pulse(20);
                        self.show_toast(format!("漫画已就绪: {}", meta.title));
                    }
                    Err(e) => {
                        self.show_toast(format!("漫画解析失败: {}", e));
                    }
                }
            }
            BookType::Unknown => {
                self.show_toast("不支持的文件格式");
            }
        }
    }

    fn handle_text_input(&mut self, input: &InputState) {
        if input.is_pressed(HandheldButton::Menu) {
            self.text_view.show_menu = !self.text_view.show_menu;
            return;
        }

        if self.text_view.show_menu {
            if input.is_pressed(HandheldButton::DpadRight) {
                self.text_view.menu_selection = (self.text_view.menu_selection + 1) % 4;
            } else if input.is_pressed(HandheldButton::DpadLeft) {
                self.text_view.menu_selection = (self.text_view.menu_selection + 3) % 4;
            } else if input.is_pressed(HandheldButton::ButtonB) {
                self.text_view.show_menu = false;
            } else if input.is_pressed(HandheldButton::ButtonY) {
                self.settings.theme = self.settings.theme.next();
                self.settings.save();
                self.update_rgb_led();
                self.show_toast(format!("主题: {}", self.settings.theme.name()));
            }
            return;
        }

        let mut toast_msg = None;
        let mut exit_to_shelf = false;
        let mut page_changed = false;

        if let Some(book) = &mut self.text_book {
            if input.is_pressed(HandheldButton::R1)
                || input.is_pressed(HandheldButton::ButtonA)
                || input.is_pressed(HandheldButton::DpadRight)
                || input.is_pressed(HandheldButton::DpadDown)
            {
                page_changed = book.next_page();
            } else if input.is_pressed(HandheldButton::L1)
                || input.is_pressed(HandheldButton::DpadLeft)
                || input.is_pressed(HandheldButton::DpadUp)
            {
                page_changed = book.prev_page();
            } else if input.is_pressed(HandheldButton::R2) {
                book.settings.font_size = (book.settings.font_size + 2.0).min(48.0);
                self.settings.font_size = book.settings.font_size;
                self.settings.save();
                book.repaginate(&self.canvas);
                toast_msg = Some(format!("字号: {:.0}px", book.settings.font_size));
            } else if input.is_pressed(HandheldButton::L2) {
                book.settings.font_size = (book.settings.font_size - 2.0).max(18.0);
                self.settings.font_size = book.settings.font_size;
                self.settings.save();
                book.repaginate(&self.canvas);
                toast_msg = Some(format!("字号: {:.0}px", book.settings.font_size));
            } else if input.is_pressed(HandheldButton::ButtonB) {
                exit_to_shelf = true;
            } else if input.is_pressed(HandheldButton::ButtonY) {
                self.settings.theme = self.settings.theme.next();
                self.settings.save();
                self.update_rgb_led();
                toast_msg = Some(format!("主题: {}", self.settings.theme.name()));
            }
        }

        if exit_to_shelf {
            self.save_current_progress();
            self.screen = AppScreen::Shelf;
            self.refresh_books();
            return;
        }

        if page_changed {
            RumbleController::pulse(15);
            self.save_current_progress();
        }

        if let Some(msg) = toast_msg {
            self.show_toast(msg);
        }
    }

    fn handle_comic_input(&mut self, input: &InputState, _dt: f32) {
        if input.is_pressed(HandheldButton::Menu) {
            self.comic_view.show_menu = !self.comic_view.show_menu;
            return;
        }

        if self.comic_view.show_menu {
            if input.is_pressed(HandheldButton::DpadRight) {
                self.comic_view.menu_selection = (self.comic_view.menu_selection + 1) % 5;
            } else if input.is_pressed(HandheldButton::DpadLeft) {
                self.comic_view.menu_selection = (self.comic_view.menu_selection + 4) % 5;
            } else if input.is_pressed(HandheldButton::ButtonB) {
                self.comic_view.show_menu = false;
            } else if input.is_pressed(HandheldButton::ButtonX) {
                if let Some(comic) = &mut self.comic_book {
                    comic.reset_zoom();
                    self.show_toast("已还原 1.0x 全屏");
                }
                self.comic_view.show_menu = false;
            } else if input.is_pressed(HandheldButton::ButtonY) {
                if let Some(comic) = &mut self.comic_book {
                    comic.toggle_reading_direction();
                    let dir_name = match comic.reading_direction {
                        ReadingDirection::RightToLeft => "日漫 (RTL 从右向左)",
                        ReadingDirection::LeftToRight => "国漫 (LTR 从左向右)",
                    };
                    self.show_toast(format!("阅读顺序: {}", dir_name));
                }
            } else if input.is_pressed(HandheldButton::FnKey) {
                if let Some(comic) = &mut self.comic_book {
                    comic.toggle_auto_crop();
                    let status = if comic.auto_crop { "开启" } else { "关闭" };
                    self.show_toast(format!("智能切白边: {}", status));
                }
            }
            return;
        }

        let sw = SCREEN_WIDTH as f32;
        let sh = SCREEN_HEIGHT as f32;
        let mut toast_msg = None;
        let mut exit_to_shelf = false;
        let mut page_changed = false;

        if let Some(comic) = &mut self.comic_book {
            if input.is_pressed(HandheldButton::R2) || input.is_held(HandheldButton::R2) {
                comic.zoom_in(0.15, sw, sh);
                RumbleController::pulse(10);
            } else if input.is_pressed(HandheldButton::L2) || input.is_held(HandheldButton::L2) {
                comic.zoom_out(0.15, sw, sh);
                RumbleController::pulse(10);
            } else if input.is_pressed(HandheldButton::ButtonX) {
                comic.reset_zoom();
                RumbleController::pulse(15);
                toast_msg = Some("已还原 1.0x 全屏".to_string());
            } else if input.is_pressed(HandheldButton::ButtonY) {
                comic.toggle_reading_direction();
                let dir_name = match comic.reading_direction {
                    ReadingDirection::RightToLeft => "日漫 [RTL]",
                    ReadingDirection::LeftToRight => "国漫 [LTR]",
                };
                toast_msg = Some(format!("切换为: {}", dir_name));
            } else if input.is_pressed(HandheldButton::FnKey) {
                comic.toggle_auto_crop();
                let status = if comic.auto_crop { "已开启" } else { "已关闭" };
                toast_msg = Some(format!("智能切白边: {}", status));
            } else if input.is_pressed(HandheldButton::ButtonB) {
                if comic.zoom_level > 1.05 {
                    comic.reset_zoom();
                } else {
                    exit_to_shelf = true;
                }
            }

            if !exit_to_shelf {
                let is_zoomed = comic.zoom_level > 1.05;
                if is_zoomed {
                    let pan_speed = 35.0;
                    if input.is_pressed(HandheldButton::DpadRight) || input.is_held(HandheldButton::DpadRight) {
                        comic.pan(pan_speed, 0.0, sw, sh);
                    }
                    if input.is_pressed(HandheldButton::DpadLeft) || input.is_held(HandheldButton::DpadLeft) {
                        comic.pan(-pan_speed, 0.0, sw, sh);
                    }
                    if input.is_pressed(HandheldButton::DpadDown) || input.is_held(HandheldButton::DpadDown) {
                        comic.pan(0.0, pan_speed, sw, sh);
                    }
                    if input.is_pressed(HandheldButton::DpadUp) || input.is_held(HandheldButton::DpadUp) {
                        comic.pan(0.0, -pan_speed, sw, sh);
                    }

                    if input.is_pressed(HandheldButton::R1) || input.is_pressed(HandheldButton::ButtonA) {
                        page_changed = comic.next_page();
                    } else if input.is_pressed(HandheldButton::L1) {
                        page_changed = comic.prev_page();
                    }
                } else {
                    if input.is_pressed(HandheldButton::R1)
                        || input.is_pressed(HandheldButton::ButtonA)
                        || input.is_pressed(HandheldButton::DpadRight)
                    {
                        page_changed = match comic.reading_direction {
                            ReadingDirection::RightToLeft => comic.next_page(),
                            ReadingDirection::LeftToRight => comic.next_page(),
                        };
                    } else if input.is_pressed(HandheldButton::L1)
                        || input.is_pressed(HandheldButton::DpadLeft)
                    {
                        page_changed = match comic.reading_direction {
                            ReadingDirection::RightToLeft => comic.prev_page(),
                            ReadingDirection::LeftToRight => comic.prev_page(),
                        };
                    } else if input.is_pressed(HandheldButton::DpadDown) {
                        comic.set_zoom_preset(ZoomMode::FitWidth, sw, sh);
                    }
                }
            }
        }

        if exit_to_shelf {
            self.save_current_progress();
            self.screen = AppScreen::Shelf;
            self.refresh_books();
            return;
        }

        if page_changed {
            RumbleController::pulse(20);
            self.save_current_progress();
        }

        if let Some(msg) = toast_msg {
            self.show_toast(msg);
        }
    }

    fn save_current_progress(&mut self) {
        if let Some(book) = &self.text_book {
            if self.shelf_view.selected_index < self.books.len() {
                let path = self.books[self.shelf_view.selected_index].path.clone();
                self.library.update_progress(&path, book.current_page, book.pages.len(), 1.0);
            }
        } else if let Some(comic) = &self.comic_book {
            if self.shelf_view.selected_index < self.books.len() {
                let path = self.books[self.shelf_view.selected_index].path.clone();
                self.library.update_progress(&path, comic.current_page, comic.page_entries.len(), comic.zoom_level);
            }
        }
    }

    pub fn render(&mut self) {
        let palette = self.settings.theme.palette();

        match self.screen {
            AppScreen::Shelf => self.shelf_view.render(&mut self.canvas, &self.books, &self.settings.library_path, &palette),
            AppScreen::ReadingText => {
                if let Some(book) = &self.text_book {
                    self.text_view.render(&mut self.canvas, book, &palette);
                }
            }
            AppScreen::ReadingComic => {
                if let Some(comic) = &self.comic_book {
                    self.comic_view.render(&mut self.canvas, comic, &palette);
                }
            }
            AppScreen::Settings => {
                self.settings_view.render(&mut self.canvas, &self.settings, &palette);
            }
        }

        if let Some(msg) = &self.toast_message {
            Widgets::draw_toast(&mut self.canvas, msg, &palette);
        }
    }
}
