use crate::model::{Chapter, Config, Experiment, Stats, World};
use eframe::egui::{self, Color32, FontId, Pos2, Rect, RichText, Sense, Stroke, Vec2};
use std::time::Duration;

const BG: Color32 = Color32::from_rgb(17, 27, 34);
const PANEL: Color32 = Color32::from_rgb(23, 36, 44);
const MUTED: Color32 = Color32::from_rgb(148, 170, 177);
const GOLD: Color32 = Color32::from_rgb(231, 190, 105);
const TEAL: Color32 = Color32::from_rgb(103, 211, 187);

#[derive(Clone, Copy, PartialEq)]
enum View {
    Map,
    Charts,
    Rules,
}
#[derive(Clone, Copy, PartialEq)]
enum Coloring {
    Plain,
    Wealth,
    Vision,
    Metabolism,
    Sex,
    Culture,
}

pub struct SugarscapeApp {
    pub world: World,
    draft: Config,
    running: bool,
    speed: u32,
    last_time: Option<f64>,
    accumulator: f64,
    view: View,
    coloring: Coloring,
    show_agents: bool,
    show_grid: bool,
    selected: Option<u64>,
    mobile_params: bool,
}

impl SugarscapeApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let mut visuals = egui::Visuals::dark();
        visuals.panel_fill = BG;
        visuals.window_fill = PANEL;
        visuals.extreme_bg_color = BG;
        visuals.selection.bg_fill = Color32::from_rgb(40, 91, 89);
        visuals.selection.stroke = Stroke::new(1.0, TEAL);
        cc.egui_ctx.set_theme(egui::Theme::Dark);
        cc.egui_ctx.set_visuals_of(egui::Theme::Dark, visuals);
        cc.egui_ctx.all_styles_mut(|s| {
            s.spacing.item_spacing = Vec2::new(10.0, 9.0);
            s.spacing.button_padding = Vec2::new(12.0, 7.0);
            s.spacing.slider_width = 155.0;
            s.text_styles
                .insert(egui::TextStyle::Body, FontId::proportional(14.0));
            s.text_styles
                .insert(egui::TextStyle::Button, FontId::proportional(14.0));
            s.text_styles
                .insert(egui::TextStyle::Heading, FontId::proportional(24.0));
        });
        Self::default()
    }

    fn reset(&mut self) {
        self.draft = self.draft.clone().normalized();
        self.world = World::new(self.draft.clone());
        self.selected = None;
        self.accumulator = 0.0;
        self.last_time = None;
    }

    fn advance(&mut self, ctx: &egui::Context) {
        let time = ctx.input(|i| i.time);
        let delta = self.last_time.map_or(0.0, |t| (time - t).clamp(0.0, 0.25));
        self.last_time = Some(time);
        if self.running {
            self.accumulator += delta * self.speed as f64;
            let steps = self.accumulator.floor().min(25.0) as usize;
            for _ in 0..steps {
                self.world.step();
            }
            self.accumulator -= steps as f64;
            if self.world.agents.is_empty() {
                self.running = false;
                self.accumulator = 0.0;
            }
            ctx.request_repaint_after(Duration::from_millis(16));
        } else {
            self.accumulator = 0.0;
        }
    }

    fn controls(&mut self, ui: &mut egui::Ui) {
        ui.add_space(8.0);
        eyebrow(ui, "ПАРАМЕТРЫ СИСТЕМЫ");
        ui.label(RichText::new("Начальные условия").size(20.0));
        ui.add_space(6.0);
        egui::ScrollArea::vertical()
            .id_salt("parameters")
            .show(ui, |ui| {
                label(ui, "Среда");
                slider(ui, &mut self.draft.size, 20..=100, "Сторона мира");
                slider(ui, &mut self.draft.population, 1..=1500, "Агентов");
                slider(ui, &mut self.draft.capacity, 1..=12, "Сахар на вершине");
                ui.add_enabled_ui(self.draft.experiment != Experiment::Immediate, |ui| {
                    slider(ui, &mut self.draft.growback, 0..=12, "Прирост / шаг");
                });
                ui.checkbox(&mut self.draft.wrap, "Замкнутый мир (тор)");
                ui.add_space(8.0);
                label(ui, "Агенты");
                slider(ui, &mut self.draft.vision, 1..=20, "Макс. зрение");
                slider(ui, &mut self.draft.metabolism, 1..=12, "Макс. метаболизм");
                slider(
                    ui,
                    &mut self.draft.initial_sugar,
                    5..=100,
                    "Макс. начальный запас",
                );
                ui.label(
                    RichText::new(
                        "Зрение и метаболизм: от 1 до максимума. Запас: от 5 до максимума.",
                    )
                    .small()
                    .color(MUTED),
                );
                if self.draft.experiment.aging() {
                    slider(ui, &mut self.draft.min_age, 1..=200, "Мин. срок жизни");
                    self.draft.max_age = self.draft.max_age.max(self.draft.min_age);
                    slider(
                        ui,
                        &mut self.draft.max_age,
                        self.draft.min_age..=500,
                        "Макс. срок жизни",
                    );
                }
                if self.draft.experiment.social() {
                    ui.add_space(8.0);
                    label(ui, "Размножение S");
                    slider(ui, &mut self.draft.female_percent, 0..=100, "Доля женского пола, %");
                    slider(ui, &mut self.draft.fertility_start, 1..=100, "Начало фертильности");
                    self.draft.female_fertility_end = self.draft.female_fertility_end.max(self.draft.fertility_start);
                    self.draft.male_fertility_end = self.draft.male_fertility_end.max(self.draft.fertility_start);
                    slider(ui, &mut self.draft.female_fertility_end, self.draft.fertility_start..=200, "Конец фертильности: Ж");
                    slider(ui, &mut self.draft.male_fertility_end, self.draft.fertility_start..=200, "Конец фертильности: М");
                    ui.label(RichText::new("Для рождения нужны сосед другого пола, запас не ниже начального и свободная клетка.").small().color(MUTED));
                    if self.draft.experiment.culture() {
                        label(ui, "Культура K");
                        ui.label("Культурных признаков (нечётное)");
                        ui.add(egui::Slider::new(&mut self.draft.culture_tags, 1..=31).step_by(2.0));
                        slider(ui, &mut self.draft.culture_contacts, 0..=self.draft.culture_tags, "Копирований / сосед / шаг");
                    }
                    if self.draft.experiment == Experiment::Conflict {
                        label(ui, "Конфликт C");
                        slider(ui, &mut self.draft.combat_loot, 0..=100, "Макс. добыча с жертвы");
                    }
                    ui.checkbox(&mut self.draft.segregated, "Разделить группы на старте");
                }
                ui.add_space(8.0);
                label(ui, "Воспроизводимость");
                ui.horizontal(|ui| {
                    ui.label("Seed");
                    ui.add(egui::DragValue::new(&mut self.draft.seed).speed(1));
                    if ui
                        .small_button("+1")
                        .on_hover_text("Следующий seed; применить кнопкой ниже")
                        .clicked()
                    {
                        self.draft.seed = self.draft.seed.wrapping_add(1);
                    }
                });
                let dirty = self.draft != self.world.config;
                ui.add_space(4.0);
                if ui
                    .add_sized(
                        [ui.available_width(), 36.0],
                        egui::Button::new(
                            RichText::new(if dirty {
                                "Применить и начать заново"
                            } else {
                                "Начать заново"
                            })
                            .color(BG),
                        )
                        .fill(TEAL),
                    )
                    .clicked()
                {
                    self.running = false;
                    self.reset();
                }
                ui.label(
                    RichText::new(if dirty {
                        "Есть изменения. Применение сбросит текущий опыт."
                    } else {
                        "Параметры применяются при новом запуске."
                    })
                    .small()
                    .color(if dirty { GOLD } else { MUTED }),
                );
                ui.separator();
                label(ui, "Отображение · меняется сразу");
                ui.checkbox(&mut self.show_agents, "Показывать агентов");
                ui.checkbox(&mut self.show_grid, "Сетка");
                egui::ComboBox::from_id_salt("color")
                    .selected_text(self.coloring.title())
                    .width(210.0)
                    .show_ui(ui, |ui| {
                        for color in [
                            Coloring::Plain,
                            Coloring::Wealth,
                            Coloring::Vision,
                            Coloring::Metabolism,
                            Coloring::Sex,
                            Coloring::Culture,
                        ] {
                            ui.selectable_value(&mut self.coloring, color, color.title());
                        }
                    });
                ui.add_space(12.0);
                ui.label(
                    RichText::new("E P S T E I N  /  A X T E L L")
                        .small()
                        .color(MUTED),
                );
                ui.label(
                    RichText::new("Growing Artificial Societies · 1996")
                        .small()
                        .color(MUTED),
                );
            });
    }

    fn toolbar(&mut self, ui: &mut egui::Ui, compact: bool) {
        ui.horizontal_wrapped(|ui| {
            ui.label(RichText::new("SUGARSCAPE").size(23.0).strong().color(GOLD));
            ui.label(RichText::new("/  искусственные общества").color(MUTED));
            if compact
                && ui
                    .selectable_label(self.mobile_params, "Параметры")
                    .clicked()
            {
                self.mobile_params = !self.mobile_params;
            }
        });
        ui.add_space(3.0);
        ui.horizontal_wrapped(|ui| {
            let previous_chapter = self.draft.experiment.chapter();
            let mut chapter = previous_chapter;
            ui.label("Глава");
            egui::ComboBox::from_id_salt("chapter")
                .width(330.0)
                .selected_text(chapter.title())
                .show_ui(ui, |ui| {
                    for item in Chapter::ALL {
                        ui.selectable_value(&mut chapter, item, item.title());
                    }
                });
            if chapter != previous_chapter {
                self.draft.experiment = chapter.experiments()[0];
                self.running = false;
                self.coloring = if chapter == Chapter::Society {
                    Coloring::Culture
                } else {
                    Coloring::Plain
                };
                self.reset();
            }
        });
        ui.horizontal_wrapped(|ui| {
            let before = self.draft.experiment;
            egui::ComboBox::from_id_salt("experiment")
                .width(300.0)
                .selected_text(self.draft.experiment.title())
                .show_ui(ui, |ui| {
                    for &experiment in self.draft.experiment.chapter().experiments() {
                        ui.selectable_value(
                            &mut self.draft.experiment,
                            experiment,
                            experiment.title(),
                        );
                    }
                });
            if before != self.draft.experiment {
                self.running = false;
                self.reset();
            }
            if ui
                .add(
                    egui::Button::new(if self.running {
                        "Пауза"
                    } else {
                        "Запуск"
                    })
                    .fill(Color32::from_rgb(35, 81, 77)),
                )
                .clicked()
            {
                self.running = !self.running;
            }
            if ui.button("Шаг").on_hover_text("Один шаг (N)").clicked() {
                self.running = false;
                self.world.step();
            }
            if ui
                .button("Сброс")
                .on_hover_text("Повторить текущий опыт с тем же seed")
                .clicked()
            {
                self.running = false;
                self.draft = self.world.config.clone();
                self.reset();
            }
            ui.add(egui::Slider::new(&mut self.speed, 1..=100).text("шаг/с"));
            ui.label(
                RichText::new(format!("t = {}", self.world.tick))
                    .monospace()
                    .color(TEAL),
            );
        });
    }

    fn content(&mut self, ui: &mut egui::Ui) {
        ui.add_space(8.0);
        ui.horizontal_wrapped(|ui| {
            ui.label(RichText::new(self.world.config.experiment.chapter().title()).size(23.0));
            ui.label(
                RichText::new(if self.world.config.experiment.social() {
                    "ЛАБОРАТОРИЯ 02"
                } else {
                    "ЛАБОРАТОРИЯ 01"
                })
                .small()
                .color(TEAL),
            );
        });
        ui.label(RichText::new(self.world.config.experiment.description()).color(MUTED));
        ui.add_space(4.0);
        let s = self.world.history.back().unwrap().clone();
        ui.horizontal_wrapped(|ui| {
            metric(ui, "ПОПУЛЯЦИЯ", format!("{}", s.population));
            metric(ui, "СРЕДНИЙ ЗАПАС", format!("{:.1}", s.mean_wealth));
            metric(ui, "ДЖИНИ", format!("{:.3}", s.gini));
            metric(ui, "САХАР В МИРЕ", format!("{}", s.sugar));
        });
        if self.world.config.experiment.social() {
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new(format!("Синих: {}   Красных: {}   Фертильных: {}   Рождений: {}   Потерь в конфликте: {}",
                    s.population - s.red_population, s.red_population, s.fertile_population, s.births, s.combat_deaths)).color(TEAL));
            });
        }
        ui.add_space(5.0);
        ui.horizontal(|ui| {
            ui.selectable_value(&mut self.view, View::Map, "Карта мира");
            ui.selectable_value(&mut self.view, View::Charts, "Динамика");
            ui.selectable_value(&mut self.view, View::Rules, "Правила модели");
        });
        ui.separator();
        egui::ScrollArea::vertical().id_salt("content").show(ui, |ui| {
            match self.view {
                View::Map => {
                    if ui.available_width() > 770.0 {
                        ui.horizontal_top(|ui| {
                            let width = (ui.available_width() - 260.0).min(680.0);
                            ui.vertical(|ui| { ui.set_width(width); self.map(ui, width); });
                            ui.vertical(|ui| { ui.set_width(240.0); self.inspector(ui); self.small_charts(ui); });
                        });
                    } else {
                        let width = ui.available_width().min(650.0);
                        self.map(ui, width);
                        self.inspector(ui);
                        self.small_charts(ui);
                    }
                }
                View::Charts => {
                    chart(ui, "Численность населения", &self.world, |s| s.population as f64, TEAL, 180.0);
                    chart(ui, "Средний запас сахара", &self.world, |s| s.mean_wealth, GOLD, 160.0);
                    chart(ui, "Неравенство · коэффициент Джини", &self.world, |s| s.gini, TEAL, 140.0);
                    if self.world.config.experiment.social() {
                        chart(ui, "Фертильные агенты", &self.world, |s| s.fertile_population as f64, TEAL, 140.0);
                        chart(ui, "Красная культурная группа", &self.world, |s| s.red_population as f64, Color32::from_rgb(223,101,87), 140.0);
                        chart(ui, "Рождения · накопительно", &self.world, |s| s.births as f64, GOLD, 140.0);
                        if self.world.config.experiment == Experiment::Conflict {
                            chart(ui, "Потери в конфликте · накопительно", &self.world, |s| s.combat_deaths as f64, GOLD, 140.0);
                        }
                    }
                    histogram(ui, &self.world, 150.0);
                    ui.label(RichText::new("История: последние 1000 шагов. Графики включают начальное состояние t = 0.").small().color(MUTED));
                }
                View::Rules => self.rules(ui),
            }
        });
    }

    fn map(&mut self, ui: &mut egui::Ui, width: f32) {
        let size = width.min((ui.ctx().content_rect().height() - 300.0).max(280.0));
        let (response, painter) = ui.allocate_painter(Vec2::splat(size), Sense::click());
        let rect = response.rect;
        let n = self.world.config.size;
        let cell = size / n as f32;
        for (i, c) in self.world.cells.iter().enumerate() {
            let p = rect.min + Vec2::new((i % n) as f32 * cell, (i / n) as f32 * cell);
            let gap = if self.show_grid && cell > 4.0 {
                0.6
            } else {
                0.0
            };
            painter.rect_filled(
                Rect::from_min_size(p, Vec2::splat(cell - gap)),
                0.0,
                sugar_color(c.sugar as f32 / self.world.config.capacity as f32),
            );
        }
        if let Some(a) = self
            .world
            .agents
            .iter()
            .find(|a| Some(a.id) == self.selected)
        {
            for (p, _) in self.world.visible(a.pos, a.vision) {
                let min = rect.min + Vec2::new((p % n) as f32 * cell, (p / n) as f32 * cell);
                painter.rect_filled(
                    Rect::from_min_size(min, Vec2::splat(cell)),
                    0.0,
                    Color32::from_rgba_unmultiplied(87, 225, 215, 65),
                );
            }
        }
        if self.show_agents {
            let wealth_max = self
                .world
                .agents
                .iter()
                .map(|a| a.sugar)
                .max()
                .unwrap_or(1)
                .max(1) as f32;
            for a in &self.world.agents {
                let pos =
                    rect.min + Vec2::new((a.pos % n) as f32 + 0.5, (a.pos / n) as f32 + 0.5) * cell;
                let t = match self.coloring {
                    Coloring::Plain | Coloring::Sex | Coloring::Culture => 0.5,
                    Coloring::Wealth => a.sugar as f32 / wealth_max,
                    Coloring::Vision => a.vision as f32 / self.world.config.vision as f32,
                    Coloring::Metabolism => {
                        a.metabolism as f32 / self.world.config.metabolism as f32
                    }
                };
                let color = if self.coloring == Coloring::Culture {
                    if a.red(self.world.config.culture_tags) {
                        Color32::from_rgb(236, 110, 100)
                    } else {
                        Color32::from_rgb(99, 179, 241)
                    }
                } else if self.coloring == Coloring::Sex {
                    if a.female {
                        Color32::from_rgb(229, 153, 220)
                    } else {
                        Color32::from_rgb(99, 208, 190)
                    }
                } else if self.coloring == Coloring::Plain {
                    Color32::from_rgb(223, 101, 87)
                } else {
                    mix(
                        Color32::from_rgb(134, 193, 225),
                        Color32::from_rgb(166, 43, 83),
                        t,
                    )
                };
                painter.circle_filled(pos, (cell * 0.30).max(1.3), Color32::from_rgb(25, 32, 36));
                painter.circle_filled(pos, (cell * 0.23).max(1.0), color);
                if Some(a.id) == self.selected {
                    painter.circle_stroke(pos, cell * 0.47, Stroke::new(2.0, Color32::WHITE));
                }
            }
        }
        if let Some(pos) = response.hover_pos() {
            let x = ((pos.x - rect.min.x) / cell) as usize;
            let y = ((pos.y - rect.min.y) / cell) as usize;
            if x < n && y < n {
                let p = y * n + x;
                let agent = self.world.agents.iter().find(|a| a.pos == p);
                response.clone().on_hover_ui(|ui| {
                    ui.label(format!(
                        "Участок ({x}, {y}) · сахар {} / {}",
                        self.world.cells[p].sugar, self.world.cells[p].capacity
                    ));
                    if let Some(a) = agent {
                        ui.label(format!(
                            "Агент #{} · запас {} · зрение {} · метаболизм {}",
                            a.id, a.sugar, a.vision, a.metabolism
                        ));
                    }
                    ui.small("Нажмите для наблюдения за агентом");
                });
                if response.clicked() {
                    self.selected = agent.map(|a| a.id);
                }
            }
        }
        ui.horizontal_wrapped(|ui| {
            ui.label(RichText::new("САХАР").small().color(MUTED));
            for i in 0..=4 {
                let (r, p) = ui.allocate_painter(Vec2::new(22.0, 10.0), Sense::hover());
                p.rect_filled(r.rect, 2.0, sugar_color(i as f32 / 4.0));
            }
            ui.label(RichText::new(format!("0 → {}", self.world.config.capacity)).small());
            ui.label(
                RichText::new("Агенты")
                    .small()
                    .color(Color32::from_rgb(223, 101, 87)),
            );
        });
        if self.coloring == Coloring::Culture {
            ui.label(
                RichText::new("Синий: большинство 0 · Красный: большинство 1")
                    .small()
                    .color(MUTED),
            );
        } else if self.coloring == Coloring::Sex {
            ui.label(
                RichText::new("Розовый: женский пол · Бирюзовый: мужской пол")
                    .small()
                    .color(MUTED),
            );
        }
        ui.label(
            RichText::new(format!(
                "{} × {} участков · {} · seed {}",
                n,
                n,
                if self.world.config.wrap {
                    "тор"
                } else {
                    "закрытые границы"
                },
                self.world.config.seed
            ))
            .small()
            .color(MUTED),
        );
    }

    fn inspector(&self, ui: &mut egui::Ui) {
        label(ui, "Наблюдение");
        if let Some(id) = self.selected {
            if let Some(a) = self.world.agents.iter().find(|a| a.id == id) {
                ui.label(
                    RichText::new(format!("Агент #{}", a.id))
                        .size(19.0)
                        .color(TEAL),
                );
                ui.label(format!("Запас: {}   Возраст: {}", a.sugar, a.age));
                ui.label(format!(
                    "Зрение: {}   Метаболизм: {}",
                    a.vision, a.metabolism
                ));
                if self.world.config.experiment.aging() {
                    ui.label(format!("Срок жизни: {}", a.max_age));
                }
                if self.world.config.experiment.social() {
                    ui.label(format!(
                        "Пол: {} · поколение {}",
                        if a.female { "Ж" } else { "М" },
                        a.generation
                    ));
                    ui.label(format!(
                        "Группа: {} · начальный запас {}",
                        if a.red(self.world.config.culture_tags) {
                            "красная"
                        } else {
                            "синяя"
                        },
                        a.endowment
                    ));
                    ui.label(
                        RichText::new(format!(
                            "{:0width$b}",
                            a.tags,
                            width = self.world.config.culture_tags as usize
                        ))
                        .monospace(),
                    );
                    ui.label(if a.fertile(&self.world.config) {
                        "Может размножаться"
                    } else {
                        "Сейчас не фертилен / мало запаса"
                    });
                    if let Some([p, q]) = a.parents {
                        ui.label(format!("Родители: #{p}, #{q}"));
                    }
                }
                ui.label(
                    RichText::new("Бирюзовым отмечена область зрения.")
                        .small()
                        .color(MUTED),
                );
            } else {
                ui.label(format!("Агент #{id} погиб."));
            }
        } else {
            ui.label(
                RichText::new(
                    "Выберите агента на карте, чтобы увидеть его запас, возраст и область зрения.",
                )
                .color(MUTED),
            );
        }
        ui.separator();
        let s = self.world.history.back().unwrap();
        ui.label(format!("Среднее зрение    {:.2}", s.mean_vision));
        ui.label(format!("Метаболизм          {:.2}", s.mean_metabolism));
        ui.label(format!(
            "От голода              {}",
            self.world.starvation_deaths
        ));
        ui.label(format!("От старости           {}", self.world.age_deaths));
        ui.label(format!("Новых агентов       {}", self.world.births));
        if self.world.config.experiment == Experiment::Conflict {
            ui.label(format!("В конфликте          {}", self.world.combat_deaths));
        }
    }

    fn small_charts(&self, ui: &mut egui::Ui) {
        ui.add_space(10.0);
        chart(
            ui,
            "Популяция",
            &self.world,
            |s| s.population as f64,
            TEAL,
            95.0,
        );
        histogram(ui, &self.world, 95.0);
    }

    fn rules(&self, ui: &mut egui::Ui) {
        ui.set_max_width(780.0);
        let rules: &[(&str, &str)] = if self.world.config.experiment.social() {
            &[
                (
                    "01 / Порядок шага",
                    "Восстановление сахара → движение или конфликт в случайном порядке → потребление и смерть → передача культуры (если включена) → рождения. Новые агенты начинают действовать на следующем шаге. Автоматического замещения умерших в этой главе нет.",
                ),
                (
                    "02 / Размножение S",
                    "Проверяются пары соседей по четырём направлениям, один раз за шаг. Нужны разные полы, возраст в заданном интервале, запас каждого не ниже его начального и свободная клетка возле одного из родителей. При наличии условий агент может иметь несколько детей за шаг с разными соседями.",
                ),
                (
                    "03 / Потомство",
                    "Каждый родитель передаёт половину своего начального запаса с округлением вверх; сахар сохраняется. Зрение, метаболизм и каждый культурный признак выбираются независимо от одного из родителей. Пол и срок жизни задаются случайно по настройкам. Возраст ребёнка — 0; родители и поколение сохраняются.",
                ),
                (
                    "04 / Культура K",
                    "Культура — строка нечётной длины из 0 и 1. Большинство нулей определяет синюю группу, большинство единиц — красную. В случайном порядке каждый агент передаёт соседям случайные признаки. Число попыток копирования задаётся ползунком; 0 отключает обмен. Принадлежность к группе меняется вместе с признаками.",
                ),
                (
                    "05 / Конфликт C",
                    "В опыте «Межгрупповой конфликт» правило движения допускает нападение на более бедного члена другой группы. Награда — сахар клетки плюс запас жертвы в пределах лимита добычи. Выбирается ближайшая клетка с максимальной наградой. Союзников и равных или более богатых противников атаковать нельзя.",
                ),
                (
                    "06 / Возмездие",
                    "В этой реализации проверяется область зрения нападающего из целевой клетки. Если в ней остаётся противник богаче нападающего после сбора и добычи (до метаболизма), цель исключается. Жертва удаляется сразу, даже если её ход ещё не наступил. Остаток её запаса исчезает; наследование после смерти пока не реализовано.",
                ),
                (
                    "07 / Что исследовать",
                    "Сравните три опыта при одном seed. Проверьте состав полов, возрастное окно и доступность ресурса. Затем включите обмен культурой и конфликт. Отслеживайте рождения, культурные группы и потери. Это учебная интерпретация правил S, K, C на процедурной карте, а не точная реплика рисунков книги.",
                ),
            ]
        } else {
            &[
                (
                    "01 / Среда",
                    "Двумерная решётка с двумя сахарными вершинами. У каждого участка есть ёмкость. Перед движением сахар восстанавливается на α единиц либо сразу до ёмкости. По умолчанию противоположные края соединены.",
                ),
                (
                    "02 / Движение M",
                    "На каждом шаге агенты действуют последовательно в новом случайном порядке. Агент видит участки по четырём направлениям. Среди свободных участков и своей клетки он выбирает самый богатый сахаром; при равенстве — ближайший, затем случайный. Другие агенты не заслоняют обзор.",
                ),
                (
                    "03 / Потребление",
                    "Агент собирает весь сахар выбранного участка и вычитает свой метаболизм из запаса. При нулевом запасе он погибает. Зрение и метаболизм назначаются при создании агента и не меняются.",
                ),
                (
                    "04 / Замещение R",
                    "Только в опыте «Жизнь и смерть» действует предел возраста. После движений умерших заменяют новые агенты на случайных свободных участках. Это замещение, без размножения и наследования. Новые агенты начинают действовать на следующем шаге.",
                ),
                (
                    "05 / Что исследовать",
                    "Сравните мгновенное и медленное восстановление. Увеличьте метаболизм или численность. Проследите, как отбор изменяет среднее зрение и метаболизм выживших. Проверьте, почему при одинаковых правилах возникает неравенство запасов.",
                ),
                (
                    "06 / Воспроизводимость",
                    "Один seed и одинаковые параметры дают одинаковую траекторию в браузере и настольном приложении. Скорость меняет только частоту шагов. Изменения параметров применяются новым запуском. Джини считается среди живых агентов; при пустой популяции показатели равны нулю.",
                ),
            ]
        };
        for &(title, text) in rules {
            ui.add_space(10.0);
            ui.label(RichText::new(title).size(19.0).color(GOLD));
            ui.label(text);
        }
        ui.add_space(15.0);
        ui.label(RichText::new("Учебная реализация: карта построена процедурно, а не оцифрована из книги. Набор экспериментов будет расширяться по мере чтения.").color(MUTED));
        ui.hyperlink_to(
            "Epstein & Axtell · Growing Artificial Societies",
            "https://www.bloomsbury.com/au/growing-artificial-societies-9780262550253/",
        );
        ui.hyperlink_to(
            "Правила Sugarscape · формальная спецификация (Kehoe)",
            "https://arxiv.org/abs/1505.06012",
        );
        ui.hyperlink_to(
            "Сравнение · NetLogo Immediate Growback",
            "https://ccl.northwestern.edu/netlogo/models/Sugarscape1ImmediateGrowback",
        );
    }
}

impl Default for SugarscapeApp {
    fn default() -> Self {
        let draft = Config::default();
        Self {
            world: World::new(draft.clone()),
            draft,
            running: false,
            speed: 10,
            last_time: None,
            accumulator: 0.0,
            view: View::Map,
            coloring: Coloring::Plain,
            show_agents: true,
            show_grid: false,
            selected: None,
            mobile_params: false,
        }
    }
}

impl eframe::App for SugarscapeApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        if !ctx.egui_wants_keyboard_input() {
            if ctx.input(|i| i.key_pressed(egui::Key::Space)) {
                self.running = !self.running;
            }
            if ctx.input(|i| i.key_pressed(egui::Key::N)) {
                self.running = false;
                self.world.step();
            }
        }
        self.advance(&ctx);
        let compact = ui.available_width() < 1000.0;
        egui::Panel::top("toolbar")
            .frame(egui::Frame::new().fill(PANEL).inner_margin(14))
            .show(ui, |ui| self.toolbar(ui, compact));
        egui::Panel::bottom("footer")
            .frame(egui::Frame::new().fill(PANEL).inner_margin(8))
            .show(ui, |ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.label(
                        RichText::new(if self.running {
                            "Опыт идёт"
                        } else {
                            "Пауза"
                        })
                        .small()
                        .color(TEAL),
                    );
                    ui.label(
                        RichText::new("Пробел: запуск / пауза · N: один шаг")
                            .small()
                            .color(MUTED),
                    );
                    ui.label(
                        RichText::new("Rust + egui / wgpu · WASM + Desktop")
                            .small()
                            .color(MUTED),
                    );
                });
            });
        if !compact {
            egui::Panel::right("parameters")
                .default_size(285.0)
                .min_size(260.0)
                .max_size((ui.available_width() - 500.0).min(650.0))
                .resizable(true)
                .frame(egui::Frame::new().fill(PANEL).inner_margin(16))
                .show(ui, |ui| self.controls(ui));
        }
        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(BG).inner_margin(18))
            .show(ui, |ui| {
                if compact && self.mobile_params {
                    self.controls(ui);
                } else {
                    self.content(ui);
                }
            });
    }
}

impl Coloring {
    fn title(self) -> &'static str {
        match self {
            Self::Plain => "Цвет: одинаковый",
            Self::Wealth => "Цвет: запас сахара",
            Self::Vision => "Цвет: зрение",
            Self::Metabolism => "Цвет: метаболизм",
            Self::Sex => "Цвет: пол",
            Self::Culture => "Цвет: культурная группа",
        }
    }
}
fn label(ui: &mut egui::Ui, s: &str) {
    ui.label(RichText::new(s).strong().color(GOLD));
}
fn eyebrow(ui: &mut egui::Ui, s: &str) {
    ui.label(RichText::new(s).small().color(TEAL));
}
fn slider<T: egui::emath::Numeric>(
    ui: &mut egui::Ui,
    value: &mut T,
    range: std::ops::RangeInclusive<T>,
    title: &str,
) {
    ui.label(title);
    ui.add(egui::Slider::new(value, range));
}
fn metric(ui: &mut egui::Ui, title: &str, value: String) {
    egui::Frame::new()
        .fill(PANEL)
        .corner_radius(6)
        .inner_margin(12)
        .show(ui, |ui| {
            ui.vertical(|ui| {
                ui.set_min_width(120.0);
                ui.label(RichText::new(title).size(10.0).color(MUTED));
                ui.label(RichText::new(value).size(25.0).color(GOLD));
            });
        });
}
fn mix(a: Color32, b: Color32, t: f32) -> Color32 {
    let f = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t.clamp(0.0, 1.0)) as u8;
    Color32::from_rgb(f(a.r(), b.r()), f(a.g(), b.g()), f(a.b(), b.b()))
}
fn sugar_color(t: f32) -> Color32 {
    mix(
        Color32::from_rgb(35, 48, 48),
        Color32::from_rgb(230, 191, 99),
        t,
    )
}

fn chart(
    ui: &mut egui::Ui,
    title: &str,
    world: &World,
    value: fn(&Stats) -> f64,
    color: Color32,
    height: f32,
) {
    ui.label(RichText::new(title).color(MUTED));
    let width = ui.available_width();
    let (response, painter) = ui.allocate_painter(Vec2::new(width, height), Sense::hover());
    let r = response.rect;
    painter.rect_filled(r, 4.0, PANEL);
    let plot = r.shrink2(Vec2::new(9.0, 18.0));
    let max = world.history.iter().map(value).fold(1.0_f64, f64::max);
    for i in 0..=2 {
        let y = plot.bottom() - plot.height() * i as f32 / 2.0;
        painter.line_segment(
            [Pos2::new(plot.left(), y), Pos2::new(plot.right(), y)],
            Stroke::new(0.5, Color32::from_rgb(45, 63, 70)),
        );
    }
    let len = world.history.len();
    let points: Vec<_> = world
        .history
        .iter()
        .enumerate()
        .map(|(i, s)| {
            Pos2::new(
                plot.left() + plot.width() * i as f32 / (len.saturating_sub(1).max(1)) as f32,
                plot.bottom() - plot.height() * (value(s) / max) as f32,
            )
        })
        .collect();
    if points.len() > 1 {
        painter.add(egui::Shape::line(points, Stroke::new(1.7, color)));
    } else if let Some(p) = points.first() {
        painter.circle_filled(*p, 2.5, color);
    }
    painter.text(
        r.left_top() + Vec2::new(7.0, 3.0),
        egui::Align2::LEFT_TOP,
        format!("{max:.1}"),
        FontId::monospace(10.0),
        MUTED,
    );
    painter.text(
        r.left_bottom() + Vec2::new(7.0, -2.0),
        egui::Align2::LEFT_BOTTOM,
        format!("t {}", world.history.front().map_or(0, |s| s.tick)),
        FontId::monospace(10.0),
        MUTED,
    );
    painter.text(
        r.right_bottom() + Vec2::new(-7.0, -2.0),
        egui::Align2::RIGHT_BOTTOM,
        format!("{}", world.tick),
        FontId::monospace(10.0),
        MUTED,
    );
}

fn histogram(ui: &mut egui::Ui, world: &World, height: f32) {
    ui.label(RichText::new("Распределение запасов").color(MUTED));
    let (response, painter) =
        ui.allocate_painter(Vec2::new(ui.available_width(), height), Sense::hover());
    let r = response.rect;
    painter.rect_filled(r, 4.0, PANEL);
    let max = world
        .agents
        .iter()
        .map(|a| a.sugar)
        .max()
        .unwrap_or(1)
        .max(1);
    let mut bins = [0_usize; 16];
    for a in &world.agents {
        bins[((a.sugar as f64 / max as f64 * 16.0) as usize).min(15)] += 1;
    }
    let count = *bins.iter().max().unwrap_or(&1).max(&1) as f32;
    let plot = r.shrink2(Vec2::new(8.0, 19.0));
    let bw = plot.width() / 16.0;
    for (i, b) in bins.iter().enumerate() {
        let bar = Rect::from_min_max(
            Pos2::new(
                plot.left() + i as f32 * bw,
                plot.bottom() - *b as f32 / count * plot.height(),
            ),
            Pos2::new(plot.left() + (i + 1) as f32 * bw - 2.0, plot.bottom()),
        );
        painter.rect_filled(bar, 1.0, GOLD);
    }
    painter.text(
        r.left_top() + Vec2::new(8.0, 3.0),
        egui::Align2::LEFT_TOP,
        format!("макс. {} агентов / интервал", count as usize),
        FontId::monospace(10.0),
        MUTED,
    );
    painter.text(
        r.left_bottom() + Vec2::new(8.0, -2.0),
        egui::Align2::LEFT_BOTTOM,
        "0",
        FontId::monospace(10.0),
        MUTED,
    );
    painter.text(
        r.right_bottom() + Vec2::new(-8.0, -2.0),
        egui::Align2::RIGHT_BOTTOM,
        format!("{max} сахара"),
        FontId::monospace(10.0),
        MUTED,
    );
}
