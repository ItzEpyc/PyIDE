use eframe::egui;
use egui::text::{LayoutJob, TextFormat};

const PYTHON_KEYWORDS: &[&str] = &[
    "and", "as", "assert", "async", "await", "break", "class", "continue", "def", "del", "elif",
    "else", "except", "False", "finally", "for", "from", "global", "if", "import", "in", "is",
    "lambda", "nonlocal", "not", "or", "pass", "raise", "return", "True", "try", "while", "with",
    "yield", "None",
];

pub const EDITOR_FONT_SIZE: f32 = 20.0;

pub fn highlight_python(
    code: &str,
    ui: &egui::Ui,
    font_size: f32,
    wrap_width: f32,
) -> std::sync::Arc<egui::Galley> {
    let style = ui.style();
    let normal = TextFormat {
        font_id: egui::FontId::monospace(font_size),
        color: style.visuals.text_color(),
        ..Default::default()
    };
    let keyword = TextFormat {
        color: egui::Color32::from_rgb(110, 170, 255),
        ..normal.clone()
    };
    let string = TextFormat {
        color: egui::Color32::from_rgb(160, 210, 140),
        ..normal.clone()
    };
    let comment = TextFormat {
        color: egui::Color32::from_rgb(130, 140, 150),
        ..normal.clone()
    };
    let number = TextFormat {
        color: egui::Color32::from_rgb(235, 180, 110),
        ..normal.clone()
    };
    let mut job = LayoutJob::default();
    let mut triple_quote = None;

    for line in code.split_inclusive('\n') {
        let bytes = line.as_bytes();
        let mut index = 0;

        while index < bytes.len() {
            if let Some(quote) = triple_quote {
                let delimiter = [quote; 3];
                let end = line[index..]
                    .as_bytes()
                    .windows(3)
                    .position(|window| window == delimiter)
                    .map(|offset| index + offset + 3);
                let end = end.unwrap_or(line.len());
                job.append(&line[index..end], 0.0, string.clone());
                index = end;
                if end >= 3 && &bytes[end - 3..end] == delimiter {
                    triple_quote = None;
                }
                continue;
            }

            let current = line[index..].chars().next().unwrap();
            if current == '#' {
                job.append(&line[index..], 0.0, comment.clone());
                break;
            }

            if current == '\'' || current == '"' {
                let quote = current as u8;
                let is_triple = bytes.get(index..index + 3) == Some(&[quote; 3]);
                let delimiter_len = if is_triple { 3 } else { 1 };
                let mut end = index + delimiter_len;
                let mut closed = false;
                while end < bytes.len() {
                    if bytes[end] == b'\\' {
                        end = (end + 2).min(bytes.len());
                    } else if is_triple && bytes.get(end..end + 3) == Some(&[quote; 3]) {
                        end += 3;
                        closed = true;
                        break;
                    } else if !is_triple && bytes[end] == quote {
                        end += 1;
                        closed = true;
                        break;
                    } else {
                        end += line[end..].chars().next().unwrap().len_utf8();
                    }
                }

                job.append(&line[index..end], 0.0, string.clone());
                index = end;
                if is_triple && !closed {
                    triple_quote = Some(quote);
                }
                continue;
            }

            if current.is_ascii_alphabetic() || current == '_' {
                let start = index;
                index += current.len_utf8();
                while index < bytes.len() {
                    let next = line[index..].chars().next().unwrap();
                    if !next.is_ascii_alphanumeric() && next != '_' {
                        break;
                    }
                    index += next.len_utf8();
                }
                let word = &line[start..index];
                job.append(
                    word,
                    0.0,
                    if PYTHON_KEYWORDS.contains(&word) {
                        keyword.clone()
                    } else {
                        normal.clone()
                    },
                );
                continue;
            }

            if current.is_ascii_digit() {
                let start = index;
                index += current.len_utf8();
                while index < bytes.len() {
                    let next = line[index..].chars().next().unwrap();
                    if !next.is_ascii_alphanumeric() && next != '_' && next != '.' {
                        break;
                    }
                    index += next.len_utf8();
                }
                job.append(&line[start..index], 0.0, number.clone());
                continue;
            }

            let end = index + current.len_utf8();
            job.append(&line[index..end], 0.0, normal.clone());
            index = end;
        }
    }

    job.wrap.max_width = wrap_width;
    ui.fonts(|fonts| fonts.layout_job(job))
}

#[cfg(test)]
mod tests {
    use super::EDITOR_FONT_SIZE;

    #[test]
    fn editor_font_size_is_twenty() {
        assert_eq!(EDITOR_FONT_SIZE, 20.0);
    }
}
