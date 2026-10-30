use crate::editor::{highlight_python, EDITOR_FONT_SIZE};
use eframe::egui;
use rfd::FileDialog;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::time::Duration;

pub struct PythonIde {
    code: String,
    file_path: String,
    output: String,
    status: String,
    dirty: bool,
    running: bool,
    run_result: Option<Receiver<Result<String, String>>>,
}

impl Default for PythonIde {
    fn default() -> Self {
        Self {
            code: String::new(),
            file_path: String::new(),
            output: String::new(),
            status: "Готово".to_owned(),
            dirty: false,
            running: false,
            run_result: None,
        }
    }
}

impl PythonIde {
    fn new_file(&mut self) {
        self.code.clear();
        self.file_path.clear();
        self.output.clear();
        self.status = "Новый файл".to_owned();
        self.dirty = false;
    }

    fn open_file(&mut self) {
        let Some(path) = FileDialog::new()
            .add_filter("Python файлы", &["py"])
            .pick_file()
        else {
            return;
        };

        match fs::read_to_string(&path) {
            Ok(code) => {
                self.code = code;
                self.file_path = path.display().to_string();
                self.output.clear();
                self.status = format!("Открыт файл: {}", path.display());
                self.dirty = false;
            }
            Err(error) => {
                self.status = format!("Ошибка открытия файла: {error}");
            }
        }
    }

    fn save_file(&mut self) {
        let path = if self.file_path.trim().is_empty() {
            let Some(path) = FileDialog::new()
                .add_filter("Python файлы", &["py"])
                .set_file_name("script.py")
                .save_file()
            else {
                return;
            };
            path
        } else {
            PathBuf::from(self.file_path.trim())
        };

        match fs::write(&path, &self.code) {
            Ok(()) => {
                self.file_path = path.display().to_string();
                self.status = format!("Сохранено: {}", path.display());
                self.dirty = false;
            }
            Err(error) => {
                self.status = format!("Ошибка сохранения файла: {error}");
            }
        }
    }

    fn run_code(&mut self) {
        let working_directory = if self.file_path.trim().is_empty() {
            match std::env::current_dir() {
                Ok(path) => path,
                Err(error) => {
                    self.status = format!("Не удалось определить рабочую папку: {error}");
                    return;
                }
            }
        } else {
            let path = Path::new(self.file_path.trim());
            match path
                .parent()
                .filter(|parent| !parent.as_os_str().is_empty())
            {
                Some(parent) => parent.to_path_buf(),
                None => match std::env::current_dir() {
                    Ok(path) => path,
                    Err(error) => {
                        self.status = format!("Не удалось определить рабочую папку: {error}");
                        return;
                    }
                },
            }
        };

        let code = self.code.clone();
        let (sender, receiver) = mpsc::channel();
        self.run_result = Some(receiver);
        self.running = true;
        self.output.clear();
        self.status = "Выполняется...".to_owned();

        std::thread::spawn(move || {
            let result = Command::new("python")
                .args(["-u", "-"])
                .current_dir(working_directory)
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .and_then(|mut child| {
                    child
                        .stdin
                        .take()
                        .ok_or_else(|| {
                            std::io::Error::other("Не удалось открыть stdin процесса Python")
                        })?
                        .write_all(code.as_bytes())?;
                    child.wait_with_output()
                })
                .map(|result| {
                    let mut text = String::from_utf8_lossy(&result.stdout).into_owned();
                    let errors = String::from_utf8_lossy(&result.stderr);
                    if !errors.is_empty() {
                        if !text.is_empty() && !text.ends_with('\n') {
                            text.push('\n');
                        }
                        text.push_str(&errors);
                    }

                    if result.status.success() {
                        if text.is_empty() {
                            "Выполнение завершено успешно (вывода нет).".to_owned()
                        } else {
                            text
                        }
                    } else {
                        format!("Процесс завершился с кодом {}.\n{text}", result.status)
                    }
                })
                .map_err(|error| format!("Не удалось запустить Python: {error}"));

            let _ = sender.send(result);
        });
    }

    fn poll_run_result(&mut self, ctx: &egui::Context) {
        let result = match self.run_result.as_ref() {
            Some(receiver) => match receiver.try_recv() {
                Ok(result) => Some(result),
                Err(TryRecvError::Disconnected) => {
                    Some(Err("Процесс Python завершился без результата.".to_owned()))
                }
                Err(TryRecvError::Empty) => None,
            },
            None => None,
        };

        if let Some(result) = result {
            self.running = false;
            self.run_result = None;
            match result {
                Ok(output) => {
                    self.output = output;
                    self.status = "Выполнение завершено".to_owned();
                }
                Err(error) => {
                    self.output = error.clone();
                    self.status = error;
                }
            }
        }

        if self.running {
            ctx.request_repaint_after(Duration::from_millis(100));
        }
    }
}

impl eframe::App for PythonIde {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.poll_run_result(ctx);

        egui::TopBottomPanel::top("toolbar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                if ui.button("Новый").clicked() {
                    self.new_file();
                }
                if ui.button("Открыть").clicked() {
                    self.open_file();
                }
                if ui.button("Сохранить").clicked() {
                    self.save_file();
                }
                if ui
                    .add_enabled(!self.running, egui::Button::new("Запустить Python"))
                    .clicked()
                {
                    self.run_code();
                }
                ui.separator();
                ui.label("Файл:");
                ui.text_edit_singleline(&mut self.file_path)
                    .on_hover_text("Путь к Python-файлу; его папка станет рабочей папкой");
            });
        });

        egui::TopBottomPanel::bottom("output")
            .resizable(true)
            .default_height(190.0)
            .show(ctx, |ui| {
                ui.heading("Вывод");
                egui::ScrollArea::vertical()
                    .stick_to_bottom(true)
                    .show(ui, |ui| {
                        ui.monospace(&self.output);
                    });
            });

        egui::TopBottomPanel::bottom("status").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label(&self.status);
                if self.dirty {
                    ui.label("Изменения не сохранены");
                }
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            let viewport_size = ui.available_size();
            let font_id = egui::FontId::monospace(EDITOR_FONT_SIZE);
            let (row_height, digit_width) =
                ui.fonts(|fonts| (fonts.row_height(&font_id), fonts.glyph_width(&font_id, '0')));
            let line_count = self.code.split('\n').count();
            let gutter_width = digit_width * (line_count.ilog10() + 1) as f32 + 16.0;
            let editor_width = (viewport_size.x - gutter_width).max(80.0);
            let visible_rows = (viewport_size.y / row_height).ceil() as usize;
            let estimated_galley =
                highlight_python(&self.code, ui, EDITOR_FONT_SIZE, editor_width - 8.0);
            let content_height = viewport_size.y.max(estimated_galley.size().y + 8.0);
            let code = &mut self.code;
            let mut code_changed = false;
            let mut layouter = |ui: &egui::Ui, text: &str, wrap_width: f32| {
                highlight_python(text, ui, EDITOR_FONT_SIZE, wrap_width)
            };

            egui::ScrollArea::both()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    ui.allocate_ui_with_layout(
                        egui::vec2(gutter_width + editor_width, content_height),
                        egui::Layout::left_to_right(egui::Align::Min),
                        |ui| {
                            let (_, gutter_rect) =
                                ui.allocate_space(egui::vec2(gutter_width, content_height));
                            let output = egui::TextEdit::multiline(code)
                                .font(font_id.clone())
                                .code_editor()
                                .frame(false)
                                .desired_width(editor_width)
                                .desired_rows(visible_rows)
                                .layouter(&mut layouter)
                                .show(ui);

                            let gutter = ui.painter_at(gutter_rect);
                            gutter.rect_filled(gutter_rect, 0.0, ui.visuals().faint_bg_color);
                            let mut line_number = 1;
                            for (index, row) in output.galley.rows.iter().enumerate() {
                                if index == 0 || output.galley.rows[index - 1].ends_with_newline {
                                    gutter.text(
                                        egui::pos2(
                                            gutter_rect.right() - 6.0,
                                            output.galley_pos.y + row.rect.top(),
                                        ),
                                        egui::Align2::RIGHT_TOP,
                                        line_number.to_string(),
                                        font_id.clone(),
                                        ui.visuals().weak_text_color(),
                                    );
                                }
                                if row.ends_with_newline {
                                    line_number += 1;
                                }
                            }

                            if output.response.changed() {
                                code_changed = true;
                            }
                        },
                    );
                });
            if code_changed {
                self.dirty = true;
            }
        });
    }
}

pub fn run() -> eframe::Result<()> {
    let options = eframe::NativeOptions::default();
    eframe::run_native(
        "Rust Python IDE",
        options,
        Box::new(|_creation_context| Ok(Box::<PythonIde>::default())),
    )
}
