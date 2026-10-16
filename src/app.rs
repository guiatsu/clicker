use std::{
    collections::BTreeSet,
    sync::mpsc::{self, Receiver, Sender, TryRecvError},
    time::{Duration, Instant},
};

use eframe::egui::{self, Color32, RichText, Stroke, Vec2};

use crate::{
    engine::{EngineEvent, EngineSession, EngineStats},
    hotkeys::{HotkeyEvent, HotkeyService},
    input,
    overlay::Overlay,
    positions,
    profiles::{ProfileStore, load_or_default},
    settings::{
        ActivationMode, ClickKind, DestinationMode, Hotkey, MouseButton, Point, Settings,
        normalize_key,
    },
};

const BG: Color32 = Color32::from_rgb(12, 17, 22);
const SURFACE: Color32 = Color32::from_rgb(21, 29, 36);
const SURFACE_RAISED: Color32 = Color32::from_rgb(29, 40, 48);
const BORDER: Color32 = Color32::from_rgb(46, 62, 72);
const TEXT: Color32 = Color32::from_rgb(231, 239, 242);
const MUTED: Color32 = Color32::from_rgb(145, 163, 172);
const TEAL: Color32 = Color32::from_rgb(79, 216, 202);
const TEAL_DARK: Color32 = Color32::from_rgb(29, 126, 119);
const AMBER: Color32 = Color32::from_rgb(247, 185, 92);
const RED: Color32 = Color32::from_rgb(239, 103, 111);

const CAPTURE_SETTLE_TIME: Duration = Duration::from_millis(420);
const CAPTURE_TIMEOUT: Duration = Duration::from_secs(12);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ShortcutTarget {
    StartStop,
    AddPosition,
    ToggleOverlay,
    AddCoordinate,
    ClearPositions,
}

struct ShortcutCapture {
    target: ShortcutTarget,
    keys: BTreeSet<u16>,
    down_keys: Vec<u16>,
    started_at: Instant,
    last_transition: Instant,
}

impl ShortcutCapture {
    fn new(target: ShortcutTarget) -> Self {
        let now = Instant::now();
        Self {
            target,
            keys: BTreeSet::new(),
            down_keys: Vec::new(),
            started_at: now,
            last_transition: now,
        }
    }

    fn preview(&self) -> String {
        Hotkey::from_keys(self.keys.iter().copied()).label()
    }
}

pub struct AutoClickerApp {
    profiles: ProfileStore,
    profile_name_input: String,
    profile_error: Option<String>,
    engine: Option<EngineSession>,
    engine_sender: Sender<EngineEvent>,
    engine_events: Receiver<EngineEvent>,
    hotkeys: HotkeyService,
    overlay: Overlay,
    stats: EngineStats,
    session_total_before: u64,
    total_clicks: u64,
    status: String,
    notice: Option<String>,
    storage_warning: Option<String>,
    shortcut_capture: Option<ShortcutCapture>,
    confirm_clear_positions: bool,
    confirm_delete_profile: bool,
    coordinate_dialog: bool,
    coordinate_input: Point,
    focus_pending: bool,
}

impl AutoClickerApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let mut visuals = egui::Visuals::dark();
        visuals.panel_fill = BG;
        visuals.window_fill = SURFACE;
        visuals.faint_bg_color = SURFACE_RAISED;
        visuals.extreme_bg_color = Color32::from_rgb(8, 12, 16);
        visuals.override_text_color = Some(TEXT);
        visuals.weak_text_color = Some(MUTED);
        visuals.selection.bg_fill = TEAL_DARK;
        visuals.selection.stroke = Stroke::new(1.0_f32, TEAL);
        visuals.window_stroke = Stroke::new(1.0_f32, BORDER);
        visuals.widgets.noninteractive.bg_fill = SURFACE;
        visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0_f32, BORDER);
        visuals.widgets.noninteractive.corner_radius = egui::CornerRadius::same(10);
        visuals.widgets.inactive.bg_fill = SURFACE_RAISED;
        visuals.widgets.inactive.bg_stroke = Stroke::new(1.0_f32, BORDER);
        visuals.widgets.inactive.corner_radius = egui::CornerRadius::same(7);
        visuals.widgets.hovered.bg_fill = Color32::from_rgb(38, 57, 65);
        visuals.widgets.hovered.bg_stroke = Stroke::new(1.0_f32, TEAL);
        visuals.widgets.hovered.corner_radius = egui::CornerRadius::same(7);
        visuals.widgets.active.bg_fill = TEAL_DARK;
        visuals.widgets.active.bg_stroke = Stroke::new(1.0_f32, TEAL);
        visuals.widgets.active.corner_radius = egui::CornerRadius::same(7);
        cc.egui_ctx.set_visuals(visuals);
        cc.egui_ctx.style_mut(|style| {
            style.spacing.item_spacing = Vec2::new(10.0, 9.0);
            style.spacing.button_padding = Vec2::new(12.0, 8.0);
            style.spacing.interact_size = Vec2::new(22.0, 22.0);
        });

        let (profiles, storage_warning) = load_or_default();
        let profile_name_input = profiles.active_profile.clone();
        let (engine_sender, engine_events) = mpsc::channel();

        Self {
            profiles,
            profile_name_input,
            profile_error: None,
            engine: None,
            engine_sender,
            engine_events,
            hotkeys: HotkeyService::new(),
            overlay: Overlay::new(),
            stats: EngineStats::default(),
            session_total_before: 0,
            total_clicks: 0,
            status: "Pronto".to_owned(),
            notice: None,
            storage_warning,
            shortcut_capture: None,
            confirm_clear_positions: false,
            confirm_delete_profile: false,
            coordinate_dialog: false,
            coordinate_input: Point::default(),
            focus_pending: false,
        }
    }

    fn process_engine_events(&mut self) {
        loop {
            match self.engine_events.try_recv() {
                Ok(EngineEvent::Progress(stats)) => {
                    self.total_clicks = self.session_total_before + stats.clicks_sent;
                    self.stats = stats;
                }
                Ok(EngineEvent::Finished { stats, error }) => {
                    self.total_clicks = self.session_total_before + stats.clicks_sent;
                    self.stats = stats;
                    if let Some(mut engine) = self.engine.take() {
                        engine.join();
                    }
                    if let Some(error) = error {
                        self.status = "Falha".to_owned();
                        self.notice = Some(error);
                    } else {
                        self.status = "Pronto".to_owned();
                    }
                }
                Err(TryRecvError::Empty | TryRecvError::Disconnected) => break,
            }
        }
    }

    fn process_hotkey_events(&mut self) -> bool {
        let mut changed = false;
        while let Ok(HotkeyEvent::KeyChanged {
            key_code,
            is_down,
            down_keys,
            previous_keys,
            cursor,
        }) = self.hotkeys.events.try_recv()
        {
            if let Some(capture) = &mut self.shortcut_capture {
                if is_down {
                    capture.keys.insert(normalize_key(key_code));
                }
                capture.down_keys = down_keys;
                capture.last_transition = Instant::now();
                continue;
            }

            let settings = self.profiles.active().settings.clone();
            if is_down {
                if settings.add_position_hotkey.matches_keys(&down_keys) {
                    if let Some(cursor) = cursor {
                        positions::append(
                            &mut self.profiles.active_mut().settings.positions,
                            cursor,
                        );
                        changed = true;
                        self.notice = Some(format!(
                            "Posição {} registrada · X {} · Y {}",
                            self.profiles.active().settings.positions.len(),
                            cursor.x,
                            cursor.y
                        ));
                        self.overlay
                            .set_points(self.profiles.active().settings.positions.clone());
                    } else {
                        self.notice =
                            Some("O Windows não conseguiu ler a posição do cursor.".to_owned());
                    }
                } else if settings.add_coordinate_hotkey.matches_keys(&down_keys) {
                    self.open_coordinate_dialog(cursor);
                } else if settings.toggle_overlay_hotkey.matches_keys(&down_keys) {
                    self.toggle_overlay();
                } else if settings.clear_positions_hotkey.matches_keys(&down_keys) {
                    if settings.positions.is_empty() {
                        self.notice = Some("Não há posições para limpar.".to_owned());
                    } else {
                        self.confirm_clear_positions = true;
                        self.focus_pending = true;
                    }
                } else if settings.start_hotkey.matches_keys(&down_keys) {
                    if settings.activation_mode == ActivationMode::Toggle {
                        self.toggle_engine();
                    } else if self.engine.is_none() {
                        self.start_engine(settings);
                    }
                }
            } else if settings.activation_mode == ActivationMode::Hold
                && settings.start_hotkey.matches_keys(&previous_keys)
                && !settings.start_hotkey.matches_keys(&down_keys)
            {
                self.stop_engine();
            }
        }

        let now = Instant::now();
        let capture_ready = self.shortcut_capture.as_ref().is_some_and(|capture| {
            !capture.keys.is_empty()
                && capture.down_keys.is_empty()
                && now.duration_since(capture.last_transition) >= CAPTURE_SETTLE_TIME
        });
        if capture_ready {
            changed |= self.commit_shortcut_capture();
        } else if self
            .shortcut_capture
            .as_ref()
            .is_some_and(|capture| now.duration_since(capture.started_at) >= CAPTURE_TIMEOUT)
        {
            self.shortcut_capture = None;
            self.notice = Some("Tempo de escuta encerrado. O atalho não foi alterado.".to_owned());
        }
        changed
    }

    fn begin_shortcut_capture(&mut self, target: ShortcutTarget) {
        self.shortcut_capture = Some(ShortcutCapture::new(target));
        self.notice = Some(
            "Escutando. Pressione a combinação e solte todas as teclas para salvar.".to_owned(),
        );
    }

    fn commit_shortcut_capture(&mut self) -> bool {
        let Some(capture) = self.shortcut_capture.take() else {
            return false;
        };
        let hotkey = Hotkey::from_keys(capture.keys);
        let settings = &mut self.profiles.active_mut().settings;
        if shortcut_conflicts(settings, capture.target, &hotkey) {
            self.notice = Some(format!(
                "{} coincide ou se sobrepõe a outro atalho. Clique no atalho e tente outra combinação.",
                hotkey.label()
            ));
            return false;
        }
        set_shortcut(settings, capture.target, hotkey.clone());
        self.notice = Some(format!("Atalho salvo: {}", hotkey.label()));
        true
    }

    fn start_engine(&mut self, settings: Settings) {
        if self.engine.is_some() {
            return;
        }
        if !self.hotkeys.is_available() {
            self.notice =
                Some("A execução real dos cliques está disponível somente no Windows.".to_owned());
            return;
        }
        match EngineSession::start(settings, self.engine_sender.clone()) {
            Ok(engine) => {
                self.engine = Some(engine);
                self.session_total_before = self.total_clicks;
                self.stats = EngineStats::default();
                self.status = "Executando".to_owned();
                self.notice = None;
            }
            Err(error) => self.notice = Some(error),
        }
    }

    fn stop_engine(&mut self) {
        if let Some(engine) = &self.engine {
            engine.stop();
            self.status = "Interrompendo…".to_owned();
        }
    }

    fn toggle_engine(&mut self) {
        if self.engine.is_some() {
            self.stop_engine();
        } else {
            self.start_engine(self.profiles.active().settings.clone());
        }
    }

    fn open_coordinate_dialog(&mut self, cursor: Option<Point>) {
        let position = cursor.or_else(input::cursor_position);
        self.coordinate_input = position.unwrap_or_default();
        self.coordinate_dialog = true;
        self.focus_pending = true;
        if position.is_none() {
            self.notice =
                Some("Cursor indisponível; informe as coordenadas manualmente.".to_owned());
        }
    }

    fn toggle_overlay(&mut self) {
        let points = self.profiles.active().settings.positions.clone();
        match self.overlay.toggle(points) {
            Ok(visible) => {
                self.notice = Some(if visible {
                    "Marcadores de posição visíveis.".to_owned()
                } else {
                    "Marcadores de posição ocultos.".to_owned()
                });
            }
            Err(error) => self.notice = Some(error),
        }
    }

    fn profile_section(&mut self, ui: &mut egui::Ui, dirty: &mut bool) {
        section_title(ui, "Perfil", "Configurações e pontos salvos juntos");
        let active = self.profiles.active_profile.clone();
        ui.add_enabled_ui(self.engine.is_none(), |ui| {
            egui::ComboBox::from_id_salt("active-profile")
                .selected_text(active)
                .width(ui.available_width())
                .show_ui(ui, |ui| {
                    for profile in &self.profiles.profiles {
                        if ui
                            .selectable_value(
                                &mut self.profiles.active_profile,
                                profile.name.clone(),
                                &profile.name,
                            )
                            .changed()
                        {
                            self.profile_name_input = self.profiles.active_profile.clone();
                            self.overlay
                                .set_points(self.profiles.active().settings.positions.clone());
                            *dirty = true;
                        }
                    }
                });
        });
        ui.add_enabled_ui(self.engine.is_none(), |ui| {
            ui.add(
                egui::TextEdit::singleline(&mut self.profile_name_input)
                    .hint_text("Nome do perfil")
                    .desired_width(ui.available_width()),
            );
        });
        ui.horizontal_wrapped(|ui| {
            if ui
                .add_enabled(self.engine.is_none(), egui::Button::new("Novo perfil"))
                .clicked()
            {
                match self.profiles.create(self.profile_name_input.clone(), false) {
                    Ok(()) => {
                        self.profile_name_input = self.profiles.active_profile.clone();
                        self.profile_error = None;
                        *dirty = true;
                    }
                    Err(error) => self.profile_error = Some(error),
                }
            }
            if ui
                .add_enabled(self.engine.is_none(), egui::Button::new("Duplicar"))
                .clicked()
            {
                match self.profiles.create(self.profile_name_input.clone(), true) {
                    Ok(()) => {
                        self.profile_name_input = self.profiles.active_profile.clone();
                        self.profile_error = None;
                        *dirty = true;
                    }
                    Err(error) => self.profile_error = Some(error),
                }
            }
        });
        ui.horizontal_wrapped(|ui| {
            if ui
                .add_enabled(self.engine.is_none(), egui::Button::new("Renomear"))
                .clicked()
            {
                match self.profiles.rename_active(self.profile_name_input.clone()) {
                    Ok(()) => {
                        self.profile_name_input = self.profiles.active_profile.clone();
                        self.profile_error = None;
                        *dirty = true;
                    }
                    Err(error) => self.profile_error = Some(error),
                }
            }
            if ui
                .add_enabled(
                    self.engine.is_none() && self.profiles.profiles.len() > 1,
                    egui::Button::new("Excluir"),
                )
                .clicked()
            {
                self.confirm_delete_profile = true;
            }
        });
        if let Some(error) = &self.profile_error {
            ui.colored_label(RED, error);
        }
    }

    fn execution_section(&mut self, ui: &mut egui::Ui, settings: &mut Settings) {
        section_title(ui, "Execução", "Controle disponível em segundo plano");
        let running = self.engine.is_some();
        let (label, color) = if running {
            ("Parar", RED)
        } else {
            ("Iniciar", TEAL_DARK)
        };
        let start_button = egui::Button::new(RichText::new(label).strong().size(16.0))
            .fill(color)
            .min_size(Vec2::new(ui.available_width(), 48.0));
        if ui.add(start_button).clicked() {
            if running {
                self.stop_engine();
            } else {
                self.start_engine(settings.clone());
            }
        }
        ui.add_space(7.0);
        ui.horizontal(|ui| {
            if ui
                .add(
                    egui::Button::new(RichText::new("Parada de emergência").strong())
                        .fill(Color32::from_rgb(94, 39, 47)),
                )
                .clicked()
            {
                self.stop_engine();
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                status_badge(ui, &self.status, running);
            });
        });

        ui.add_enabled_ui(!running, |ui| {
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.label("Ativação");
                egui::ComboBox::from_id_salt("activation-mode")
                    .selected_text(match settings.activation_mode {
                        ActivationMode::Toggle => "Alternar",
                        ActivationMode::Hold => "Enquanto pressionado",
                    })
                    .show_ui(ui, |ui| {
                        ui.selectable_value(
                            &mut settings.activation_mode,
                            ActivationMode::Toggle,
                            "Alternar",
                        );
                        ui.selectable_value(
                            &mut settings.activation_mode,
                            ActivationMode::Hold,
                            "Enquanto pressionado",
                        );
                    });
            });
            self.shortcut_row(
                ui,
                "Iniciar / parar",
                ShortcutTarget::StartStop,
                &settings.start_hotkey,
            );
        });
        if !self.hotkeys.is_available() {
            ui.label(
                RichText::new("Atalhos globais e cliques reais exigem Windows.")
                    .small()
                    .color(AMBER),
            );
        }
    }

    fn speed_section(&mut self, ui: &mut egui::Ui, settings: &mut Settings) {
        section_title(ui, "Ritmo", "Uma agenda sincroniza CPS e intervalo");
        ui.add_enabled_ui(self.engine.is_none(), |ui| {
            ui.horizontal(|ui| {
                ui.label("CPS");
                let mut cps = settings.cps;
                if ui
                    .add(
                        egui::DragValue::new(&mut cps)
                            .range(0.1..=100_000.0)
                            .speed(0.2)
                            .suffix(" /s"),
                    )
                    .changed()
                {
                    settings.set_cps(cps);
                }
            });
            ui.horizontal(|ui| {
                ui.label("Intervalo");
                let mut interval = settings.interval_ms;
                if ui
                    .add(
                        egui::DragValue::new(&mut interval)
                            .range(0.01..=600_000.0)
                            .speed(0.1)
                            .suffix(" ms"),
                    )
                    .changed()
                {
                    settings.set_interval_ms(interval);
                }
            });
            ui.horizontal_wrapped(|ui| {
                ui.label("Pressionamento");
                ui.add(
                    egui::DragValue::new(&mut settings.press_duration_ms)
                        .range(0.0..=60_000.0)
                        .speed(0.1)
                        .suffix(" ms"),
                );
            });
            ui.horizontal(|ui| {
                ui.label("Botão");
                egui::ComboBox::from_id_salt("mouse-button")
                    .selected_text(button_label(settings.mouse_button))
                    .show_ui(ui, |ui| {
                        ui.selectable_value(
                            &mut settings.mouse_button,
                            MouseButton::Left,
                            "Esquerdo",
                        );
                        ui.selectable_value(
                            &mut settings.mouse_button,
                            MouseButton::Right,
                            "Direito",
                        );
                        ui.selectable_value(
                            &mut settings.mouse_button,
                            MouseButton::Middle,
                            "Central",
                        );
                    });
            });
            ui.horizontal(|ui| {
                ui.label("Clique");
                egui::ComboBox::from_id_salt("click-kind")
                    .selected_text(match settings.click_kind {
                        ClickKind::Single => "Simples",
                        ClickKind::Double => "Duplo",
                    })
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut settings.click_kind, ClickKind::Single, "Simples");
                        ui.selectable_value(&mut settings.click_kind, ClickKind::Double, "Duplo");
                    });
            });
        });
        if let Some(warning) = settings.scheduler_warning() {
            ui.add_space(4.0);
            ui.label(RichText::new(warning).small().color(AMBER));
        }
    }

    fn destination_section(&mut self, ui: &mut egui::Ui, settings: &mut Settings) {
        section_title(
            ui,
            "Destino e repetição",
            "Escolha onde os cliques acontecem",
        );
        ui.horizontal_wrapped(|ui| {
            destination_tab(
                ui,
                &mut settings.destination_mode,
                DestinationMode::CurrentCursor,
                "Cursor atual",
            );
            destination_tab(
                ui,
                &mut settings.destination_mode,
                DestinationMode::FixedPosition,
                "Posição fixa",
            );
            destination_tab(
                ui,
                &mut settings.destination_mode,
                DestinationMode::PositionSequence,
                "Sequência",
            );
        });
        ui.add_space(10.0);
        match settings.destination_mode {
            DestinationMode::CurrentCursor => {
                ui.label(
                    RichText::new("O ponteiro permanece sob controle do outro aplicativo.")
                        .color(MUTED),
                );
            }
            DestinationMode::FixedPosition => {
                ui.horizontal(|ui| {
                    ui.label("X");
                    ui.add(egui::DragValue::new(&mut settings.fixed_position.x));
                    ui.label("Y");
                    ui.add(egui::DragValue::new(&mut settings.fixed_position.y));
                    if ui.button("Usar cursor").clicked() {
                        if let Some(position) = input::cursor_position() {
                            settings.fixed_position = position;
                        } else {
                            self.notice =
                                Some("O Windows não conseguiu ler a posição do cursor.".to_owned());
                        }
                    }
                });
            }
            DestinationMode::PositionSequence => {
                ui.horizontal_wrapped(|ui| {
                    ui.label("Cliques por ponto");
                    ui.add(
                        egui::DragValue::new(&mut settings.clicks_per_position)
                            .range(1..=1_000_000),
                    );
                    ui.label("Pausa");
                    ui.add(
                        egui::DragValue::new(&mut settings.interval_between_positions_ms)
                            .range(0.0..=600_000.0)
                            .speed(1.0)
                            .suffix(" ms"),
                    );
                });
            }
        }
        ui.separator();
        ui.horizontal_wrapped(|ui| {
            ui.checkbox(&mut settings.repeat_continuously, "Repetir continuamente");
            if !settings.repeat_continuously {
                ui.label("Ciclos");
                ui.add(egui::DragValue::new(&mut settings.cycles).range(1..=1_000_000));
            }
            let mut limit_enabled = settings.max_clicks.is_some();
            if ui.checkbox(&mut limit_enabled, "Limitar cliques").changed() {
                settings.max_clicks = limit_enabled.then_some(1);
            }
            if let Some(max_clicks) = &mut settings.max_clicks {
                ui.add(egui::DragValue::new(max_clicks).range(1..=u64::MAX));
            }
        });
    }

    fn positions_section(&mut self, ui: &mut egui::Ui, settings: &mut Settings) {
        section_title(
            ui,
            "Pontos de clique",
            "Capture do cursor ou informe coordenadas exatas",
        );
        ui.horizontal_wrapped(|ui| {
            ui.label(
                RichText::new(format!("{} pontos", settings.positions.len()))
                    .strong()
                    .color(TEAL),
            );
            ui.add_space(4.0);
            if ui
                .button(if self.overlay.is_visible() {
                    "Ocultar marcadores"
                } else {
                    "Mostrar marcadores"
                })
                .clicked()
            {
                self.toggle_overlay();
            }
            if ui.button("Adicionar coordenada").clicked() {
                self.open_coordinate_dialog(None);
            }
            if ui
                .add_enabled(
                    !settings.positions.is_empty(),
                    egui::Button::new("Limpar lista"),
                )
                .clicked()
            {
                self.confirm_clear_positions = true;
                self.focus_pending = true;
            }
        });

        if settings.positions.is_empty() {
            empty_state(
                ui,
                "Nenhum ponto registrado",
                "Pressione o atalho de captura no local desejado ou adicione as coordenadas manualmente.",
            );
        } else {
            egui::Frame::new()
                .fill(Color32::from_rgb(16, 22, 28))
                .stroke(Stroke::new(1.0_f32, BORDER))
                .corner_radius(egui::CornerRadius::same(8))
                .inner_margin(10.0)
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("#").small().color(MUTED));
                        ui.add_sized(
                            [56.0, 18.0],
                            egui::Label::new(RichText::new("X").small().color(MUTED)),
                        );
                        ui.add_sized(
                            [56.0, 18.0],
                            egui::Label::new(RichText::new("Y").small().color(MUTED)),
                        );
                        ui.label(RichText::new("ORDEM").small().color(MUTED));
                    });
                    ui.separator();
                    egui::ScrollArea::vertical()
                        .max_height(240.0)
                        .show(ui, |ui| {
                            let mut remove_index = None;
                            for index in 0..settings.positions.len() {
                                ui.horizontal(|ui| {
                                    ui.label(
                                        RichText::new(format!("{:02}", index + 1))
                                            .monospace()
                                            .color(TEAL),
                                    );
                                    ui.add_sized(
                                        [58.0, 22.0],
                                        egui::DragValue::new(&mut settings.positions[index].x),
                                    );
                                    ui.add_sized(
                                        [58.0, 22.0],
                                        egui::DragValue::new(&mut settings.positions[index].y),
                                    );
                                    if ui.add_enabled(index > 0, egui::Button::new("↑")).clicked()
                                    {
                                        positions::move_up(&mut settings.positions, index);
                                    }
                                    if ui
                                        .add_enabled(
                                            index + 1 < settings.positions.len(),
                                            egui::Button::new("↓"),
                                        )
                                        .clicked()
                                    {
                                        positions::move_down(&mut settings.positions, index);
                                    }
                                    if ui.button("Remover").clicked() {
                                        remove_index = Some(index);
                                    }
                                });
                            }
                            if let Some(index) = remove_index {
                                positions::remove(&mut settings.positions, index);
                            }
                        });
                });
        }
    }

    fn position_shortcuts_section(&mut self, ui: &mut egui::Ui, settings: &Settings) {
        section_title(
            ui,
            "Atalhos globais",
            "Clique em um atalho para gravar uma combinação",
        );
        self.shortcut_row(
            ui,
            "Capturar ponto no cursor",
            ShortcutTarget::AddPosition,
            &settings.add_position_hotkey,
        );
        self.shortcut_row(
            ui,
            "Mostrar / ocultar marcadores",
            ShortcutTarget::ToggleOverlay,
            &settings.toggle_overlay_hotkey,
        );
        self.shortcut_row(
            ui,
            "Adicionar coordenada",
            ShortcutTarget::AddCoordinate,
            &settings.add_coordinate_hotkey,
        );
        self.shortcut_row(
            ui,
            "Limpar pontos",
            ShortcutTarget::ClearPositions,
            &settings.clear_positions_hotkey,
        );
        if let Some((preview, has_keys)) = self
            .shortcut_capture
            .as_ref()
            .map(|capture| (capture.preview(), !capture.keys.is_empty()))
        {
            let mut cancel_capture = false;
            ui.add_space(5.0);
            egui::Frame::new()
                .fill(Color32::from_rgb(21, 47, 50))
                .stroke(Stroke::new(1.0_f32, TEAL_DARK))
                .corner_radius(egui::CornerRadius::same(8))
                .inner_margin(10.0)
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("● ESCUTANDO").strong().color(TEAL));
                        ui.label(if has_keys {
                            preview
                        } else {
                            "Pressione as teclas desejadas…".to_owned()
                        });
                        if ui.button("Cancelar").clicked() {
                            cancel_capture = true;
                        }
                    });
                    ui.label(
                        RichText::new("Solte todas as teclas para salvar a combinação.")
                            .small()
                            .color(MUTED),
                    );
                });
            if cancel_capture {
                self.shortcut_capture = None;
                self.notice = Some("Gravação cancelada.".to_owned());
            }
        }
    }

    fn shortcut_row(
        &mut self,
        ui: &mut egui::Ui,
        label: &str,
        target: ShortcutTarget,
        hotkey: &Hotkey,
    ) {
        ui.horizontal(|ui| {
            ui.label(label);
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let listening = self
                    .shortcut_capture
                    .as_ref()
                    .is_some_and(|capture| capture.target == target);
                let text = if listening {
                    self.shortcut_capture
                        .as_ref()
                        .map(ShortcutCapture::preview)
                        .unwrap_or_default()
                } else if hotkey.is_configured() {
                    hotkey.label()
                } else {
                    "Definir".to_owned()
                };
                let button = egui::Button::new(
                    RichText::new(if listening && text == "Não configurado" {
                        "Pressione…".to_owned()
                    } else {
                        text
                    })
                    .strong()
                    .color(if listening { TEAL } else { TEXT }),
                )
                .fill(if listening {
                    Color32::from_rgb(21, 47, 50)
                } else {
                    SURFACE_RAISED
                });
                if ui.add(button).clicked() {
                    self.begin_shortcut_capture(target);
                }
            });
        });
    }

    fn performance_section(&self, ui: &mut egui::Ui, settings: &Settings) {
        let elapsed = self.stats.elapsed.as_secs_f64();
        let cps = if elapsed > 0.0 {
            self.stats.clicks_sent as f64 / elapsed
        } else {
            0.0
        };
        let event_count = self
            .stats
            .successful_operations
            .saturating_add(self.stats.failed_events);
        let failure_rate = if event_count > 0 {
            self.stats.failed_events as f64 * 100.0 / event_count as f64
        } else {
            0.0
        };
        ui.horizontal_wrapped(|ui| {
            metric(ui, "CONFIGURADO", format!("{:.2} CPS", settings.cps));
            metric(ui, "ENVIADO", format!("{cps:.2} CPS"));
            metric(ui, "NESTA EXECUÇÃO", self.stats.clicks_sent.to_string());
            metric(ui, "TOTAL DA SESSÃO", self.total_clicks.to_string());
            metric(ui, "DURAÇÃO", format_duration(self.stats.elapsed));
            metric(
                ui,
                "FALHAS",
                format!("{} · {failure_rate:.2}%", self.stats.failed_events),
            );
        });
        ui.add_space(4.0);
        ui.label(RichText::new("Contagem baseada em eventos aceitos por SendInput; o aplicativo receptor pode processar um total diferente.").small().color(MUTED));
    }

    fn save_if_changed(&mut self) {
        match self.profiles.save() {
            Ok(()) => self.storage_warning = None,
            Err(error) => {
                self.storage_warning = Some(format!("Não foi possível salvar os perfis: {error}"))
            }
        }
    }

    fn modal_windows(&mut self, ctx: &egui::Context, settings: &mut Settings, dirty: &mut bool) {
        if self.coordinate_dialog {
            egui::Window::new("Adicionar coordenada")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
                .show(ctx, |ui| {
                    ui.label("O ponto começa na posição atual do cursor. Ajuste os valores antes de salvar.");
                    ui.horizontal(|ui| {
                        ui.label("X");
                        ui.add(egui::DragValue::new(&mut self.coordinate_input.x));
                        ui.label("Y");
                        ui.add(egui::DragValue::new(&mut self.coordinate_input.y));
                    });
                    ui.horizontal(|ui| {
                        if ui.button("Cancelar").clicked() {
                            self.coordinate_dialog = false;
                        }
                        if ui.button(RichText::new("Adicionar ponto").strong()).clicked() {
                            positions::append(&mut settings.positions, self.coordinate_input);
                            self.coordinate_dialog = false;
                            *dirty = true;
                            self.notice = Some(format!("Ponto salvo · X {} · Y {}", self.coordinate_input.x, self.coordinate_input.y));
                        }
                    });
                });
        }
        if self.confirm_clear_positions {
            egui::Window::new("Limpar pontos")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
                .show(ctx, |ui| {
                    ui.label(format!(
                        "Remover os {} pontos deste perfil?",
                        settings.positions.len()
                    ));
                    ui.horizontal(|ui| {
                        if ui.button("Cancelar").clicked() {
                            self.confirm_clear_positions = false;
                        }
                        if ui
                            .add(
                                egui::Button::new("Limpar pontos")
                                    .fill(Color32::from_rgb(94, 39, 47)),
                            )
                            .clicked()
                        {
                            settings.positions.clear();
                            self.overlay.set_points(Vec::new());
                            self.confirm_clear_positions = false;
                            *dirty = true;
                            self.notice = Some("Lista de pontos limpa.".to_owned());
                        }
                    });
                });
        }
        if self.confirm_delete_profile {
            egui::Window::new("Excluir perfil")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
                .show(ctx, |ui| {
                    ui.label(format!(
                        "Excluir o perfil ‘{}’?",
                        self.profiles.active_profile
                    ));
                    ui.horizontal(|ui| {
                        if ui.button("Cancelar").clicked() {
                            self.confirm_delete_profile = false;
                        }
                        if ui
                            .add(
                                egui::Button::new("Excluir perfil")
                                    .fill(Color32::from_rgb(94, 39, 47)),
                            )
                            .clicked()
                        {
                            if self.profiles.delete_active().is_ok() {
                                self.profile_name_input = self.profiles.active_profile.clone();
                                self.overlay
                                    .set_points(self.profiles.active().settings.positions.clone());
                                *dirty = true;
                            }
                            self.confirm_delete_profile = false;
                        }
                    });
                });
        }
    }
}

impl eframe::App for AutoClickerApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.process_engine_events();
        let mut dirty = self.process_hotkey_events();
        if self.focus_pending {
            ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
            self.focus_pending = false;
        }
        ctx.request_repaint_after(Duration::from_millis(100));

        egui::TopBottomPanel::top("app-header")
            .exact_height(62.0)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    draw_mark(ui);
                    ui.vertical(|ui| {
                        ui.heading(RichText::new("Neo AutoClicker").size(20.0).strong());
                        ui.label(
                            RichText::new("Cliques, posições e atalhos em um só lugar.")
                                .small()
                                .color(MUTED),
                        );
                    });
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        status_badge(ui, &self.status, self.engine.is_some());
                    });
                });
            });

        egui::TopBottomPanel::bottom("performance-panel")
            .resizable(false)
            .show(ctx, |ui| {
                egui::Frame::new()
                    .fill(SURFACE)
                    .corner_radius(egui::CornerRadius::same(10))
                    .inner_margin(egui::Margin::symmetric(18, 12))
                    .show(ui, |ui| {
                        self.performance_section(ui, &self.profiles.active().settings)
                    });
            });

        let mut settings = Settings::default();
        let mut original = Settings::default();
        egui::SidePanel::left("control-rail")
            .resizable(false)
            .exact_width(310.0)
            .show(ctx, |ui| {
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        egui::Frame::new()
                            .fill(SURFACE)
                            .stroke(Stroke::new(1.0_f32, BORDER))
                            .corner_radius(egui::CornerRadius::same(10))
                            .inner_margin(14.0)
                            .show(ui, |ui| {
                                self.profile_section(ui, &mut dirty);
                            });
                        ui.add_space(10.0);
                        settings = self.profiles.active().settings.clone();
                        original = settings.clone();
                        egui::Frame::new()
                            .fill(SURFACE)
                            .stroke(Stroke::new(1.0_f32, BORDER))
                            .corner_radius(egui::CornerRadius::same(10))
                            .inner_margin(14.0)
                            .show(ui, |ui| {
                                self.execution_section(ui, &mut settings);
                            });
                        ui.add_space(10.0);
                        egui::Frame::new()
                            .fill(SURFACE)
                            .stroke(Stroke::new(1.0_f32, BORDER))
                            .corner_radius(egui::CornerRadius::same(10))
                            .inner_margin(14.0)
                            .show(ui, |ui| {
                                self.speed_section(ui, &mut settings);
                            });
                    });
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    section_title(
                        ui,
                        "Configuração do clique",
                        "Defina o destino, o ciclo e os pontos salvos no perfil",
                    );
                    egui::Frame::new()
                        .fill(SURFACE)
                        .stroke(Stroke::new(1.0_f32, BORDER))
                        .corner_radius(egui::CornerRadius::same(10))
                        .inner_margin(16.0)
                        .show(ui, |ui| {
                            self.destination_section(ui, &mut settings);
                        });
                    ui.add_space(12.0);
                    egui::Frame::new()
                        .fill(SURFACE)
                        .stroke(Stroke::new(1.0_f32, BORDER))
                        .corner_radius(egui::CornerRadius::same(10))
                        .inner_margin(16.0)
                        .show(ui, |ui| {
                            self.positions_section(ui, &mut settings);
                        });
                    ui.add_space(12.0);
                    egui::Frame::new()
                        .fill(SURFACE)
                        .stroke(Stroke::new(1.0_f32, BORDER))
                        .corner_radius(egui::CornerRadius::same(10))
                        .inner_margin(16.0)
                        .show(ui, |ui| {
                            self.position_shortcuts_section(ui, &settings);
                        });
                    if let Some(notice) = &self.notice {
                        ui.add_space(10.0);
                        ui.label(RichText::new(notice).color(TEAL));
                    }
                    if let Some(warning) = &self.storage_warning {
                        ui.add_space(6.0);
                        ui.label(RichText::new(warning).color(AMBER));
                    }
                });
        });

        let profile_before_modal = self.profiles.active_profile.clone();
        self.modal_windows(ctx, &mut settings, &mut dirty);
        let profile_changed_in_modal = self.profiles.active_profile != profile_before_modal;
        if settings != original {
            dirty = true;
        }
        if dirty {
            if profile_changed_in_modal {
                self.overlay
                    .set_points(self.profiles.active().settings.positions.clone());
            } else {
                self.profiles.active_mut().settings = settings.clone();
                self.overlay.set_points(settings.positions.clone());
            }
            self.save_if_changed();
        }
    }
}

fn shortcut_conflicts(settings: &Settings, target: ShortcutTarget, candidate: &Hotkey) -> bool {
    if !candidate.is_configured() {
        return false;
    }
    let all = [
        (ShortcutTarget::StartStop, &settings.start_hotkey),
        (ShortcutTarget::AddPosition, &settings.add_position_hotkey),
        (
            ShortcutTarget::ToggleOverlay,
            &settings.toggle_overlay_hotkey,
        ),
        (
            ShortcutTarget::AddCoordinate,
            &settings.add_coordinate_hotkey,
        ),
        (
            ShortcutTarget::ClearPositions,
            &settings.clear_positions_hotkey,
        ),
    ];
    all.iter()
        .any(|(other_target, other)| *other_target != target && candidate.conflicts_with(other))
}

fn set_shortcut(settings: &mut Settings, target: ShortcutTarget, hotkey: Hotkey) {
    match target {
        ShortcutTarget::StartStop => settings.start_hotkey = hotkey,
        ShortcutTarget::AddPosition => settings.add_position_hotkey = hotkey,
        ShortcutTarget::ToggleOverlay => settings.toggle_overlay_hotkey = hotkey,
        ShortcutTarget::AddCoordinate => settings.add_coordinate_hotkey = hotkey,
        ShortcutTarget::ClearPositions => settings.clear_positions_hotkey = hotkey,
    }
}

fn section_title(ui: &mut egui::Ui, title: &str, subtitle: &str) {
    ui.horizontal_wrapped(|ui| {
        ui.label(RichText::new(title).strong().size(17.0));
        ui.label(RichText::new(subtitle).small().color(MUTED));
    });
    ui.add_space(9.0);
}

fn metric(ui: &mut egui::Ui, label: &str, value: String) {
    ui.vertical(|ui| {
        ui.label(RichText::new(label).small().color(MUTED));
        ui.label(RichText::new(value).strong().size(15.0));
    });
    ui.add_space(12.0);
}

fn status_badge(ui: &mut egui::Ui, status: &str, running: bool) {
    let color = if running {
        TEAL
    } else if status == "Falha" {
        RED
    } else {
        MUTED
    };
    egui::Frame::new()
        .fill(if running {
            Color32::from_rgb(20, 52, 53)
        } else {
            SURFACE_RAISED
        })
        .stroke(Stroke::new(
            1.0_f32,
            if running { TEAL_DARK } else { BORDER },
        ))
        .corner_radius(egui::CornerRadius::same(18))
        .inner_margin(egui::Margin::symmetric(10, 5))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                let (dot, _) = ui.allocate_exact_size(Vec2::splat(9.0), egui::Sense::hover());
                if running {
                    let phase = ui.input(|input| input.time * 3.1);
                    let pulse = (phase.sin() * 0.5 + 0.5) as f32;
                    ui.painter().circle_stroke(
                        dot.center(),
                        3.2 + pulse * 1.8,
                        Stroke::new(1.0_f32, TEAL.gamma_multiply(0.32 + pulse * 0.25)),
                    );
                }
                ui.painter().circle_filled(dot.center(), 2.8, color);
                ui.label(
                    RichText::new(status.to_uppercase())
                        .small()
                        .strong()
                        .color(color),
                );
            });
        });
}

fn destination_tab(
    ui: &mut egui::Ui,
    current: &mut DestinationMode,
    value: DestinationMode,
    label: &str,
) {
    let selected = *current == value;
    let response = ui.add(
        egui::Button::new(
            RichText::new(label)
                .strong()
                .color(if selected { TEAL } else { TEXT }),
        )
        .fill(if selected {
            Color32::from_rgb(20, 52, 53)
        } else {
            SURFACE_RAISED
        }),
    );
    if response.clicked() {
        *current = value;
    }
}

fn empty_state(ui: &mut egui::Ui, title: &str, description: &str) {
    egui::Frame::new()
        .fill(Color32::from_rgb(16, 22, 28))
        .stroke(Stroke::new(1.0_f32, BORDER))
        .corner_radius(egui::CornerRadius::same(10))
        .inner_margin(16.0)
        .show(ui, |ui| {
            ui.label(RichText::new(title).strong());
            ui.label(RichText::new(description).color(MUTED));
        });
}

fn draw_mark(ui: &mut egui::Ui) {
    let (rect, _) = ui.allocate_exact_size(Vec2::splat(34.0), egui::Sense::hover());
    let painter = ui.painter();
    painter.rect_filled(
        rect,
        egui::CornerRadius::same(9),
        Color32::from_rgb(17, 48, 51),
    );
    painter.rect_stroke(
        rect,
        egui::CornerRadius::same(9),
        Stroke::new(1.0_f32, TEAL_DARK),
        egui::StrokeKind::Inside,
    );
    let left = rect.left_center();
    let right = rect.right_center();
    painter.line_segment(
        [left + Vec2::new(8.0, 0.0), right - Vec2::new(8.0, 0.0)],
        Stroke::new(1.0_f32, TEAL),
    );
    painter.circle_filled(rect.center(), 4.0, TEAL);
    painter.circle_filled(rect.center() + Vec2::new(0.0, 9.0), 1.8, AMBER);
}

fn button_label(button: MouseButton) -> &'static str {
    match button {
        MouseButton::Left => "Esquerdo",
        MouseButton::Right => "Direito",
        MouseButton::Middle => "Central",
    }
}

fn format_duration(duration: Duration) -> String {
    let seconds = duration.as_secs();
    format!(
        "{:02}:{:02}:{:02}",
        seconds / 3600,
        (seconds / 60) % 60,
        seconds % 60
    )
}
