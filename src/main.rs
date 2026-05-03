use eframe::egui;
use eframe::egui::{Color32, FontId, RichText, Stroke, Vec2};
use std::fs;
use std::path::{Path, PathBuf};

// ---------------------------------------------------------------------------
// Data types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
struct GradeCategory {
    name: String,
    weight: f64,
    grades: Vec<f64>,
}

impl GradeCategory {
    fn new(name: &str, weight: f64) -> Self {
        Self { name: name.to_string(), weight, grades: Vec::new() }
    }

    fn average(&self) -> Option<f64> {
        if self.grades.is_empty() { return None; }
        Some(self.grades.iter().sum::<f64>() / self.grades.len() as f64)
    }

    fn weighted_value(&self) -> Option<f64> {
        self.average().map(|avg| avg * self.weight)
    }
}

#[derive(Debug, Clone)]
struct Subject {
    name: String,
    categories: Vec<GradeCategory>,
}

impl Subject {
    fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            categories: vec![
                GradeCategory::new("A", 0.6),
                GradeCategory::new("B", 0.3),
                GradeCategory::new("C", 0.1),
            ],
        }
    }

    fn final_grade(&self) -> Option<f64> {
        let active: Vec<_> = self.categories.iter().filter(|c| !c.grades.is_empty()).collect();
        if active.is_empty() { return None; }
        let total_weight: f64 = active.iter().map(|c| c.weight).sum();
        if total_weight == 0.0 { return None; }
        let weighted_sum: f64 = active.iter().filter_map(|c| c.weighted_value()).sum();
        Some(weighted_sum / total_weight)
    }

    fn total_grades(&self) -> usize {
        self.categories.iter().map(|c| c.grades.len()).sum()
    }

    fn predict_needed(&self, target: f64, category_name: &str) -> Option<f64> {
        let cat = self.categories.iter().find(|c| c.name == category_name)?;
        let other_weighted: f64 = self.categories.iter()
            .filter(|c| c.name != category_name && !c.grades.is_empty())
            .filter_map(|c| c.weighted_value())
            .sum();
        let other_weight: f64 = self.categories.iter()
            .filter(|c| c.name != category_name && !c.grades.is_empty())
            .map(|c| c.weight)
            .sum();
        let total_weight = other_weight + cat.weight;
        let needed = (target * total_weight - other_weighted) / cat.weight;
        Some(needed)
    }
}

// ---------------------------------------------------------------------------
// JSON persistence
// ---------------------------------------------------------------------------

fn save_subject(subject: &Subject, dir: &Path) -> std::io::Result<()> {
    fs::create_dir_all(dir)?;
    let path = subject_path(dir, &subject.name);
    let mut json = String::from("{\n");
    json.push_str(&format!("  \"name\": \"{}\",\n", subject.name));
    json.push_str("  \"categories\": [\n");
    for (i, cat) in subject.categories.iter().enumerate() {
        json.push_str("    {\n");
        json.push_str(&format!("      \"name\": \"{}\",\n", cat.name));
        json.push_str(&format!("      \"weight\": {},\n", cat.weight));
        let grades_str = cat.grades.iter().map(|g| g.to_string()).collect::<Vec<_>>().join(", ");
        json.push_str(&format!("      \"grades\": [{}]\n", grades_str));
        json.push_str(if i + 1 < subject.categories.len() { "    },\n" } else { "    }\n" });
    }
    json.push_str("  ]\n}\n");
    fs::write(path, json)
}

fn load_subject(path: &Path) -> Option<Subject> {
    let content = fs::read_to_string(path).ok()?;
    parse_subject_json(&content)
}

fn parse_subject_json(json: &str) -> Option<Subject> {
    let name = extract_string(json, "\"name\"")?;
    let mut subject = Subject { name, categories: Vec::new() };
    let cats_start = json.find("\"categories\"")?;
    let array_start = json[cats_start..].find('[')? + cats_start;
    let array_end = json[array_start..].rfind(']')? + array_start;
    let array_content = &json[array_start + 1..array_end];
    let mut pos = 0;
    while let Some(open) = array_content[pos..].find('{') {
        let abs_open = pos + open;
        let close = find_matching_brace(array_content, abs_open)?;
        let block = &array_content[abs_open..=close];
        let cat_name = extract_string(block, "\"name\"")?;
        let weight = extract_number(block, "\"weight\"")?;
        let grades = extract_number_array(block, "\"grades\"");
        subject.categories.push(GradeCategory { name: cat_name, weight, grades });
        pos = abs_open + 1;
    }
    Some(subject)
}

fn extract_string(src: &str, key: &str) -> Option<String> {
    let key_pos = src.find(key)?;
    let after_key = &src[key_pos + key.len()..];
    let colon = after_key.find(':')?;
    let after_colon = after_key[colon + 1..].trim_start();
    if !after_colon.starts_with('"') { return None; }
    let inner = &after_colon[1..];
    let end = inner.find('"')?;
    Some(inner[..end].to_string())
}

fn extract_number(src: &str, key: &str) -> Option<f64> {
    let key_pos = src.find(key)?;
    let after_key = &src[key_pos + key.len()..];
    let colon = after_key.find(':')?;
    let after_colon = after_key[colon + 1..].trim_start();
    let end = after_colon.find(|c: char| c == ',' || c == '\n' || c == '}').unwrap_or(after_colon.len());
    after_colon[..end].trim().parse().ok()
}

fn extract_number_array(src: &str, key: &str) -> Vec<f64> {
    let key_pos = match src.find(key) { Some(p) => p, None => return vec![] };
    let after_key = &src[key_pos + key.len()..];
    let bracket_open = match after_key.find('[') { Some(p) => p, None => return vec![] };
    let bracket_close = match after_key.find(']') { Some(p) => p, None => return vec![] };
    let inner = &after_key[bracket_open + 1..bracket_close];
    inner.split(',').filter_map(|s| s.trim().parse::<f64>().ok()).collect()
}

fn find_matching_brace(s: &str, open: usize) -> Option<usize> {
    let mut depth = 0;
    for (i, c) in s[open..].char_indices() {
        match c {
            '{' => depth += 1,
            '}' => { depth -= 1; if depth == 0 { return Some(open + i); } }
            _ => {}
        }
    }
    None
}

fn subject_path(dir: &Path, name: &str) -> PathBuf {
    dir.join(format!("{}.json", name.to_lowercase()))
}

fn list_subjects(dir: &Path) -> Vec<String> {
    let Ok(entries) = fs::read_dir(dir) else { return vec![] };
    let mut names: Vec<String> = entries
        .filter_map(|e| e.ok())
        .filter_map(|e| {
            let p = e.path();
            if p.extension()? == "json" { Some(p.file_stem()?.to_string_lossy().to_string()) } else { None }
        })
        .collect();
    names.sort();
    names
}

// ---------------------------------------------------------------------------
// CSV export
// ---------------------------------------------------------------------------

fn export_csv(dir: &Path) -> Result<PathBuf, String> {
    let subjects = list_subjects(dir);
    if subjects.is_empty() {
        return Err("No subjects to export.".to_string());
    }

    let mut csv = String::from("Subject,Category,Weight,Grades,Average,Weighted,Final Grade\n");

    for name in &subjects {
        let path = subject_path(dir, name);
        let Some(subject) = load_subject(&path) else { continue };
        let final_grade = subject.final_grade().map(|g| format!("{:.2}", g)).unwrap_or_default();

        for (i, cat) in subject.categories.iter().enumerate() {
            let grades_str = cat.grades.iter().map(|g| format!("{:.2}", g)).collect::<Vec<_>>().join("; ");
            let avg = cat.average().map(|a| format!("{:.2}", a)).unwrap_or_default();
            let weighted = cat.weighted_value().map(|w| format!("{:.2}", w)).unwrap_or_default();
            let final_col = if i == 0 { final_grade.clone() } else { String::new() };
            csv.push_str(&format!(
                "{},{},{:.0}%,\"{}\",{},{},{}\n",
                subject.name, cat.name, cat.weight * 100.0, grades_str, avg, weighted, final_col
            ));
        }
    }

    let out_path = dir.join("grades_export.csv");
    fs::write(&out_path, csv).map_err(|e| format!("Write error: {}", e))?;
    Ok(out_path)
}

// ---------------------------------------------------------------------------
// Color helpers
// ---------------------------------------------------------------------------

fn grade_color(grade: f64) -> Color32 {
    if grade <= 1.5 {
        Color32::from_rgb(80, 200, 120)
    } else if grade <= 2.5 {
        Color32::from_rgb(100, 180, 255)
    } else if grade <= 3.5 {
        Color32::from_rgb(255, 200, 60)
    } else if grade <= 4.5 {
        Color32::from_rgb(255, 140, 50)
    } else {
        Color32::from_rgb(255, 80, 80)
    }
}

// ---------------------------------------------------------------------------
// App state
// ---------------------------------------------------------------------------

#[derive(PartialEq)]
enum View {
    SubjectList,
    SubjectDetail,
    NewSubject,
    Overview,
}

struct App {
    grades_dir: PathBuf,
    subjects: Vec<String>,
    selected_subject: Option<Subject>,
    view: View,

    new_subject_name: String,
    new_grade_input: String,
    new_grade_category: String,
    status_message: String,

    predictor_target: String,
    predictor_category: String,

    confirm_delete: bool,
    confirm_delete_grade: Option<(usize, usize)>,

    export_message: String,
}

impl App {
    fn new(grades_dir: PathBuf) -> Self {
        let subjects = list_subjects(&grades_dir);
        Self {
            subjects,
            grades_dir,
            selected_subject: None,
            view: View::SubjectList,
            new_subject_name: String::new(),
            new_grade_input: String::new(),
            new_grade_category: "A".to_string(),
            status_message: String::new(),
            predictor_target: String::new(),
            predictor_category: "A".to_string(),
            confirm_delete: false,
            confirm_delete_grade: None,
            export_message: String::new(),
        }
    }

    fn reload_subjects(&mut self) {
        self.subjects = list_subjects(&self.grades_dir);
    }

    fn open_subject(&mut self, name: &str) {
        let path = subject_path(&self.grades_dir, name);
        if let Some(subject) = load_subject(&path) {
            self.selected_subject = Some(subject);
            self.view = View::SubjectDetail;
            self.status_message.clear();
            self.new_grade_input.clear();
            self.predictor_target.clear();
            self.confirm_delete = false;
            self.confirm_delete_grade = None;
        }
    }

    fn save_current(&mut self) {
        if let Some(ref subject) = self.selected_subject.clone() {
            match save_subject(subject, &self.grades_dir) {
                Ok(_) => self.status_message = "Saved.".to_string(),
                Err(e) => self.status_message = format!("Error: {}", e),
            }
        }
    }

    fn add_grade(&mut self) {
        let input = self.new_grade_input.trim().to_string();
        match input.parse::<f64>() {
            Ok(g) if (1.0..=5.0).contains(&g) => {
                if let Some(ref mut subject) = self.selected_subject {
                    let cat_name = self.new_grade_category.clone();
                    if let Some(cat) = subject.categories.iter_mut().find(|c| c.name == cat_name) {
                        // Insert sorted: best (lowest) first
                        let pos = cat.grades.partition_point(|&x| x <= g);
                        cat.grades.insert(pos, g);
                        self.new_grade_input.clear();
                        self.status_message = format!("Added {:.2} to {}.", g, cat_name);
                    }
                }
                self.save_current();
            }
            Ok(_) => self.status_message = "Grade must be between 1.0 and 5.0.".to_string(),
            Err(_) => self.status_message = "Invalid input.".to_string(),
        }
    }
}

// ---------------------------------------------------------------------------
// UI
// ---------------------------------------------------------------------------

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let mut style = (*ctx.style()).clone();
        style.visuals.window_fill = Color32::from_rgb(18, 18, 24);
        style.visuals.panel_fill = Color32::from_rgb(18, 18, 24);
        style.visuals.override_text_color = Some(Color32::from_rgb(220, 220, 230));
        style.visuals.widgets.inactive.bg_fill = Color32::from_rgb(35, 35, 48);
        style.visuals.widgets.hovered.bg_fill = Color32::from_rgb(50, 50, 68);
        style.visuals.widgets.active.bg_fill = Color32::from_rgb(90, 90, 180);
        style.visuals.widgets.inactive.rounding = egui::Rounding::same(6.0);
        style.visuals.widgets.hovered.rounding = egui::Rounding::same(6.0);
        style.visuals.widgets.active.rounding = egui::Rounding::same(6.0);
        ctx.set_style(style);

        match self.view {
            View::SubjectList => self.draw_subject_list(ctx),
            View::SubjectDetail => self.draw_subject_detail(ctx),
            View::NewSubject => self.draw_new_subject(ctx),
            View::Overview => self.draw_overview(ctx),
        }
    }
}

impl App {
    // -----------------------------------------------------------------------
    // Subject list
    // -----------------------------------------------------------------------

    fn draw_subject_list(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.add_space(16.0);

            ui.horizontal(|ui| {
                ui.label(RichText::new("📊 Grade Calculator").font(FontId::proportional(26.0)).color(Color32::from_rgb(180, 160, 255)));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button(RichText::new("＋ New Subject").font(FontId::proportional(14.0))).clicked() {
                        self.new_subject_name.clear();
                        self.view = View::NewSubject;
                    }
                    ui.add_space(8.0);
                    if ui.button(RichText::new("📈 Overview").font(FontId::proportional(14.0))).clicked() {
                        self.view = View::Overview;
                    }
                    ui.add_space(8.0);
                    // Export button
                    if ui.button(RichText::new("⬇ Export CSV").font(FontId::proportional(14.0))).clicked() {
                        match export_csv(&self.grades_dir) {
                            Ok(path) => self.export_message = format!("Exported to {}", path.display()),
                            Err(e) => self.export_message = e,
                        }
                    }
                });
            });

            if !self.export_message.is_empty() {
                ui.add_space(4.0);
                ui.label(RichText::new(&self.export_message).font(FontId::proportional(12.0)).color(Color32::from_rgb(80, 200, 120)));
            }

            ui.add_space(8.0);
            ui.separator();
            ui.add_space(12.0);

            if self.subjects.is_empty() {
                ui.add_space(60.0);
                ui.vertical_centered(|ui| {
                    ui.label(RichText::new("No subjects yet.").font(FontId::proportional(18.0)).color(Color32::GRAY));
                    ui.add_space(8.0);
                    ui.label(RichText::new("Click '＋ New Subject' to get started.").color(Color32::GRAY));
                });
                return;
            }

            // Load all subject data before drawing
            let subject_data: Vec<(String, Option<f64>, usize)> = self.subjects.iter().map(|name| {
                let path = subject_path(&self.grades_dir, name);
                let sub = load_subject(&path);
                let grade = sub.as_ref().and_then(|s| s.final_grade());
                let count = sub.as_ref().map(|s| s.total_grades()).unwrap_or(0);
                (name.clone(), grade, count)
            }).collect();

            let mut to_open: Option<String> = None;

            egui::ScrollArea::vertical().show(ui, |ui| {
                for (name, grade, count) in &subject_data {
                    let response = egui::Frame::none()
                        .fill(Color32::from_rgb(28, 28, 40))
                        .rounding(egui::Rounding::same(10.0))
                        .inner_margin(egui::Margin::symmetric(16.0, 12.0))
                        .show(ui, |ui| {
                            ui.set_min_width(ui.available_width());
                            ui.horizontal(|ui| {
                                ui.vertical(|ui| {
                                    ui.label(RichText::new(name).font(FontId::proportional(17.0)));
                                    ui.label(RichText::new(format!("{} grade(s)", count)).font(FontId::proportional(11.0)).color(Color32::GRAY));
                                });
                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                    match grade {
                                        Some(g) => ui.label(
                                            RichText::new(format!("{:.2}", g))
                                                .font(FontId::proportional(22.0))
                                                .color(grade_color(*g))
                                        ),
                                        None => ui.label(RichText::new("no grades").color(Color32::GRAY)),
                                    };
                                });
                            });
                        });

                    if response.response.interact(egui::Sense::click()).clicked() {
                        to_open = Some(name.clone());
                    }
                    if response.response.interact(egui::Sense::hover()).hovered() {
                        ctx.output_mut(|o| o.cursor_icon = egui::CursorIcon::PointingHand);
                    }
                    ui.add_space(6.0);
                }
            });

            if let Some(name) = to_open {
                self.open_subject(&name);
            }
        });
    }

    // -----------------------------------------------------------------------
    // New subject
    // -----------------------------------------------------------------------

    fn draw_new_subject(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.add_space(16.0);
            ui.horizontal(|ui| {
                if ui.button("← Back").clicked() { self.view = View::SubjectList; }
                ui.add_space(8.0);
                ui.label(RichText::new("New Subject").font(FontId::proportional(22.0)).color(Color32::from_rgb(180, 160, 255)));
            });
            ui.add_space(24.0);
            ui.separator();
            ui.add_space(24.0);
            ui.label("Subject name:");
            ui.add_space(4.0);
            let response = ui.add(
                egui::TextEdit::singleline(&mut self.new_subject_name)
                    .desired_width(280.0)
                    .font(FontId::proportional(16.0))
                    .hint_text("e.g. Math, Physics...")
            );
            if response.lost_focus() && ctx.input(|i| i.key_pressed(egui::Key::Enter)) {
                self.create_subject();
            }
            ui.add_space(16.0);
            if ui.button(RichText::new("Create Subject").font(FontId::proportional(15.0))).clicked() {
                self.create_subject();
            }
            if !self.status_message.is_empty() {
                ui.add_space(12.0);
                ui.label(RichText::new(&self.status_message).color(Color32::from_rgb(255, 100, 100)));
            }
        });
    }

    fn create_subject(&mut self) {
        let name = self.new_subject_name.trim().to_string();
        if name.is_empty() { self.status_message = "Name cannot be empty.".to_string(); return; }
        let path = subject_path(&self.grades_dir, &name);
        if path.exists() { self.status_message = format!("'{}' already exists.", name); return; }
        let subject = Subject::new(&name);
        match save_subject(&subject, &self.grades_dir) {
            Ok(_) => { self.reload_subjects(); self.open_subject(&name); }
            Err(e) => self.status_message = format!("Error: {}", e),
        }
    }

    // -----------------------------------------------------------------------
    // Overview / GPA
    // -----------------------------------------------------------------------

    fn draw_overview(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.add_space(16.0);
            ui.horizontal(|ui| {
                if ui.button("← Back").clicked() { self.view = View::SubjectList; }
                ui.add_space(8.0);
                ui.label(RichText::new("📈 Overview").font(FontId::proportional(22.0)).color(Color32::from_rgb(180, 160, 255)));
            });
            ui.add_space(12.0);
            ui.separator();
            ui.add_space(12.0);

            // Load all subjects with grades
            let mut all_subjects: Vec<(String, f64)> = self.subjects.iter().filter_map(|name| {
                let path = subject_path(&self.grades_dir, name);
                let sub = load_subject(&path)?;
                let grade = sub.final_grade()?;
                Some((name.clone(), grade))
            }).collect();

            if all_subjects.is_empty() {
                ui.vertical_centered(|ui| {
                    ui.add_space(40.0);
                    ui.label(RichText::new("No grades entered yet across any subject.").color(Color32::GRAY));
                });
                return;
            }

            // Overall GPA = simple average of all final grades
            let gpa = all_subjects.iter().map(|(_, g)| g).sum::<f64>() / all_subjects.len() as f64;

            // GPA banner
            egui::Frame::none()
                .fill(Color32::from_rgb(28, 28, 40))
                .rounding(egui::Rounding::same(12.0))
                .inner_margin(egui::Margin::symmetric(20.0, 16.0))
                .show(ui, |ui| {
                    ui.set_min_width(ui.available_width());
                    ui.horizontal(|ui| {
                        ui.vertical(|ui| {
                            ui.label(RichText::new("Overall GPA").font(FontId::proportional(13.0)).color(Color32::GRAY));
                            ui.label(RichText::new("Average across all subjects with grades").font(FontId::proportional(11.0)).color(Color32::GRAY));
                        });
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(RichText::new(format!("{:.2}", gpa)).font(FontId::proportional(36.0)).color(grade_color(gpa)));
                        });
                    });
                });

            ui.add_space(16.0);
            ui.label(RichText::new("All subjects").font(FontId::proportional(14.0)).color(Color32::GRAY));
            ui.add_space(8.0);

            // Sort best to worst
            all_subjects.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());

            egui::ScrollArea::vertical().show(ui, |ui| {
                for (name, grade) in &all_subjects {
                    egui::Frame::none()
                        .fill(Color32::from_rgb(24, 24, 36))
                        .rounding(egui::Rounding::same(8.0))
                        .inner_margin(egui::Margin::symmetric(16.0, 10.0))
                        .show(ui, |ui| {
                            ui.set_min_width(ui.available_width());
                            ui.horizontal(|ui| {
                                ui.label(RichText::new(name).font(FontId::proportional(15.0)));
                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                    ui.label(
                                        RichText::new(format!("{:.2}", grade))
                                            .font(FontId::proportional(18.0))
                                            .color(grade_color(*grade))
                                    );
                                    // Show difference from GPA
                                    let diff = grade - gpa;
                                    let (diff_str, diff_color) = if diff.abs() < 0.005 {
                                        ("=".to_string(), Color32::GRAY)
                                    } else if diff < 0.0 {
                                        (format!("{:.2}", diff), Color32::from_rgb(80, 200, 120))
                                    } else {
                                        (format!("+{:.2}", diff), Color32::from_rgb(255, 140, 50))
                                    };
                                    ui.label(RichText::new(diff_str).font(FontId::proportional(12.0)).color(diff_color));
                                });
                            });

                            // Mini progress bar: fill width proportional to grade (1=full green, 5=full red)
                            let fraction = ((grade - 1.0) / 4.0) as f32;
                            let bar_color = grade_color(*grade);
                            let (rect, _) = ui.allocate_exact_size(
                                egui::vec2(ui.available_width(), 4.0),
                                egui::Sense::hover(),
                            );
                            let fill_rect = egui::Rect::from_min_size(
                                rect.min,
                                egui::vec2(rect.width() * fraction, rect.height()),
                            );
                            ui.painter().rect_filled(rect, 2.0, Color32::from_rgb(40, 40, 55));
                            ui.painter().rect_filled(fill_rect, 2.0, bar_color);
                        });
                    ui.add_space(6.0);
                }
            });
        });
    }

    // -----------------------------------------------------------------------
    // Subject detail
    // -----------------------------------------------------------------------

    fn draw_subject_detail(&mut self, ctx: &egui::Context) {
        let Some(subject) = self.selected_subject.clone() else { return; };

        egui::TopBottomPanel::top("top_bar").show(ctx, |ui| {
            ui.add_space(10.0);
            ui.horizontal(|ui| {
                if ui.button("← Back").clicked() {
                    self.view = View::SubjectList;
                    self.selected_subject = None;
                    self.reload_subjects();
                }
                ui.add_space(8.0);
                ui.label(RichText::new(&subject.name).font(FontId::proportional(22.0)).color(Color32::from_rgb(180, 160, 255)));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button(RichText::new("🗑 Delete").color(Color32::from_rgb(255, 80, 80))).clicked() {
                        self.confirm_delete = true;
                    }
                });
            });
            ui.add_space(10.0);
        });

        if self.confirm_delete {
            egui::Window::new("Confirm Delete")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
                .show(ctx, |ui| {
                    ui.label(format!("Delete '{}' permanently?", subject.name));
                    ui.add_space(12.0);
                    ui.horizontal(|ui| {
                        if ui.button(RichText::new("Yes, delete").color(Color32::from_rgb(255, 80, 80))).clicked() {
                            let path = subject_path(&self.grades_dir, &subject.name);
                            let _ = fs::remove_file(path);
                            self.reload_subjects();
                            self.selected_subject = None;
                            self.confirm_delete = false;
                            self.view = View::SubjectList;
                        }
                        if ui.button("Cancel").clicked() { self.confirm_delete = false; }
                    });
                });
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.add_space(12.0);

                // Final grade banner
                match subject.final_grade() {
                    Some(g) => {
                        egui::Frame::none()
                            .fill(Color32::from_rgb(28, 28, 40))
                            .rounding(egui::Rounding::same(12.0))
                            .inner_margin(egui::Margin::symmetric(20.0, 14.0))
                            .show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    ui.vertical(|ui| {
                                        ui.label(RichText::new("Final Grade").font(FontId::proportional(15.0)).color(Color32::GRAY));
                                        ui.label(RichText::new("1.0 = best  ·  5.0 = worst").font(FontId::proportional(11.0)).color(Color32::GRAY));
                                    });
                                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                        ui.label(RichText::new(format!("{:.2}", g)).font(FontId::proportional(32.0)).color(grade_color(g)));
                                    });
                                });
                            });
                    }
                    None => {
                        ui.label(RichText::new("No grades entered yet.").color(Color32::GRAY));
                    }
                }

                ui.add_space(16.0);

                // Category cards
                let mut grade_to_remove: Option<(usize, usize)> = None;

                for (cat_idx, cat) in subject.categories.iter().enumerate() {
                    egui::Frame::none()
                        .fill(Color32::from_rgb(24, 24, 36))
                        .rounding(egui::Rounding::same(10.0))
                        .stroke(Stroke::new(1.0, Color32::from_rgb(50, 50, 70)))
                        .inner_margin(egui::Margin::symmetric(16.0, 12.0))
                        .show(ui, |ui| {
                            ui.set_min_width(ui.available_width());

                            ui.horizontal(|ui| {
                                ui.label(RichText::new(&cat.name).font(FontId::proportional(17.0)));
                                ui.label(RichText::new(format!("{}%", (cat.weight * 100.0) as u32)).color(Color32::GRAY));
                                ui.label(RichText::new(format!("{} grade(s)", cat.grades.len())).font(FontId::proportional(11.0)).color(Color32::GRAY));
                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                    match cat.average() {
                                        Some(avg) => { ui.label(RichText::new(format!("avg {:.2}", avg)).font(FontId::proportional(15.0)).color(grade_color(avg))); }
                                        None => { ui.label(RichText::new("no grades").color(Color32::GRAY)); }
                                    };
                                });
                            });

                            if !cat.grades.is_empty() {
                                ui.add_space(8.0);
                                ui.horizontal_wrapped(|ui| {
                                    for (grade_idx, grade) in cat.grades.iter().enumerate() {
                                        let confirm_this = self.confirm_delete_grade == Some((cat_idx, grade_idx));
                                        if confirm_this {
                                            if ui.button(RichText::new("Confirm?").color(Color32::from_rgb(255, 80, 80))).clicked() {
                                                grade_to_remove = Some((cat_idx, grade_idx));
                                                self.confirm_delete_grade = None;
                                            }
                                            if ui.button("✕").clicked() {
                                                self.confirm_delete_grade = None;
                                            }
                                        } else {
                                            egui::Frame::none()
                                                .fill(Color32::from_rgb(35, 35, 52))
                                                .rounding(egui::Rounding::same(6.0))
                                                .inner_margin(egui::Margin::symmetric(8.0, 4.0))
                                                .show(ui, |ui| {
                                                    ui.horizontal(|ui| {
                                                        ui.label(RichText::new(format!("{:.2}", grade)).color(grade_color(*grade)).font(FontId::proportional(14.0)));
                                                        if ui.small_button("✕").clicked() {
                                                            self.confirm_delete_grade = Some((cat_idx, grade_idx));
                                                        }
                                                    });
                                                });
                                        }
                                    }
                                });
                            }
                        });
                    ui.add_space(8.0);
                }

                if let Some((cat_idx, grade_idx)) = grade_to_remove {
                    if let Some(ref mut s) = self.selected_subject {
                        s.categories[cat_idx].grades.remove(grade_idx);
                    }
                    self.save_current();
                }

                ui.add_space(4.0);
                ui.separator();
                ui.add_space(12.0);

                // Add grade
                ui.label(RichText::new("Add Grade").font(FontId::proportional(16.0)));
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    egui::ComboBox::from_id_source("cat_select")
                        .selected_text(&self.new_grade_category)
                        .width(70.0)
                        .show_ui(ui, |ui| {
                            for cat_name in ["A", "B", "C"] {
                                ui.selectable_value(&mut self.new_grade_category, cat_name.to_string(), cat_name);
                            }
                        });

                    let response = ui.add(
                        egui::TextEdit::singleline(&mut self.new_grade_input)
                            .desired_width(100.0)
                            .hint_text("1.0 – 5.0")
                            .font(FontId::proportional(15.0))
                    );
                    // Press Enter to add
                    if response.lost_focus() && ctx.input(|i| i.key_pressed(egui::Key::Enter)) {
                        self.add_grade();
                    }

                    if ui.button("Add").clicked() {
                        self.add_grade();
                    }
                });

                if !self.status_message.is_empty() {
                    ui.add_space(6.0);
                    let color = if self.status_message.starts_with("Error") || self.status_message.starts_with("Invalid") || self.status_message.starts_with("Grade must") {
                        Color32::from_rgb(255, 100, 100)
                    } else {
                        Color32::from_rgb(80, 200, 120)
                    };
                    ui.label(RichText::new(&self.status_message).color(color));
                }

                ui.add_space(16.0);
                ui.separator();
                ui.add_space(12.0);

                // Grade predictor
                ui.label(RichText::new("🎯 Grade Predictor").font(FontId::proportional(16.0)));
                ui.add_space(4.0);
                ui.label(RichText::new("What do I need in a category to reach a target final grade?").font(FontId::proportional(12.0)).color(Color32::GRAY));
                ui.add_space(8.0);

                ui.horizontal(|ui| {
                    ui.label("Target:");
                    ui.add(egui::TextEdit::singleline(&mut self.predictor_target).desired_width(70.0).hint_text("e.g. 2.0"));
                    ui.label("in:");
                    egui::ComboBox::from_id_source("pred_cat")
                        .selected_text(&self.predictor_category)
                        .width(70.0)
                        .show_ui(ui, |ui| {
                            for cat_name in ["A", "B", "C"] {
                                ui.selectable_value(&mut self.predictor_category, cat_name.to_string(), cat_name);
                            }
                        });
                });

                ui.add_space(8.0);

                if let Ok(target) = self.predictor_target.trim().parse::<f64>() {
                    if (1.0..=5.0).contains(&target) {
                        if let Some(needed) = subject.predict_needed(target, &self.predictor_category) {
                            let cat = subject.categories.iter().find(|c| c.name == self.predictor_category);
                            let existing_grades: Vec<f64> = cat.map(|c| c.grades.clone()).unwrap_or_default();
                            let existing_count = existing_grades.len();
                            let existing_sum: f64 = existing_grades.iter().sum();
                            let cat_weight = cat.map(|c| c.weight).unwrap_or(0.0);

                            // Compute best possible final grade (getting 1.0 next)
                            let best_next_avg = if existing_count == 0 { 1.0 } else { (existing_sum + 1.0) / (existing_count + 1) as f64 };
                            let other_weighted: f64 = subject.categories.iter()
                                .filter(|c| c.name != self.predictor_category && !c.grades.is_empty())
                                .filter_map(|c| c.weighted_value()).sum();
                            let other_weight: f64 = subject.categories.iter()
                                .filter(|c| c.name != self.predictor_category && !c.grades.is_empty())
                                .map(|c| c.weight).sum();
                            let total_weight = other_weight + cat_weight;
                            let best_final = if total_weight > 0.0 { (other_weighted + best_next_avg * cat_weight) / total_weight } else { 0.0 };

                            let (text, color) = if needed < 1.0 {
                                (format!(
                                    "Not achievable — even a perfect 1.0 in {} won't reach {:.2}.\nBest possible final grade with a 1.0 next: {:.2}.",
                                    self.predictor_category, target, best_final
                                ), Color32::from_rgb(255, 80, 80))
                            } else if needed > 5.0 {
                                (format!(
                                    "Not achievable — you'd need {:.2} in {}, which is out of range.\nBest possible final grade with a 1.0 next: {:.2}.",
                                    needed, self.predictor_category, best_final
                                ), Color32::from_rgb(255, 80, 80))
                            } else {
                                let next_grade = if existing_count == 0 {
                                    needed.clamp(1.0, 5.0)
                                } else {
                                    (needed * (existing_count + 1) as f64 - existing_sum).clamp(1.0, 5.0)
                                };
                                let new_avg = if existing_count == 0 { next_grade } else { (existing_sum + next_grade) / (existing_count + 1) as f64 };

                                let mut detail = format!("You need an average of {:.2} in {}.\n", needed, self.predictor_category);

                                let is_clean = (next_grade * 10.0 - (next_grade * 10.0).round()).abs() < 0.01;
                                if is_clean {
                                    detail.push_str(&format!("Get a {:.1} next in {} → new average: {:.2}.", next_grade, self.predictor_category, new_avg));
                                } else {
                                    let floor = next_grade.floor().clamp(1.0, 5.0);
                                    let ceil = next_grade.ceil().clamp(1.0, 5.0);
                                    let avg_floor = if existing_count == 0 { floor } else { (existing_sum + floor) / (existing_count + 1) as f64 };
                                    let avg_ceil = if existing_count == 0 { ceil } else { (existing_sum + ceil) / (existing_count + 1) as f64 };
                                    detail.push_str(&format!(
                                        "Get a {:.0} → average {:.2}  |  get a {:.0} → average {:.2}.",
                                        floor, avg_floor, ceil, avg_ceil
                                    ));
                                }

                                (detail, grade_color(needed))
                            };

                            egui::Frame::none()
                                .fill(Color32::from_rgb(28, 28, 40))
                                .rounding(egui::Rounding::same(8.0))
                                .inner_margin(egui::Margin::symmetric(14.0, 10.0))
                                .show(ui, |ui| {
                                    ui.label(RichText::new(text).color(color).font(FontId::proportional(14.0)));
                                });
                        }
                    } else {
                        ui.label(RichText::new("Target must be between 1.0 and 5.0.").color(Color32::from_rgb(255, 100, 100)));
                    }
                }

                ui.add_space(20.0);
            });
        });
    }
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

fn main() -> eframe::Result<()> {
    let grades_dir = PathBuf::from("grades");
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Grade Calculator")
            .with_inner_size([480.0, 640.0])
            .with_min_inner_size([380.0, 400.0]),
        ..Default::default()
    };
    eframe::run_native(
        "Grade Calculator",
        options,
        Box::new(|_cc| Box::new(App::new(grades_dir))),
    )
}