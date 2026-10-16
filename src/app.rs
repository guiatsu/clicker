use std::{
    sync::mpsc::{self, Receiver, Sender, TryRecvError},
    time::{Duration, Instant},
};

use eframe::egui::{self, Color32, RichText, Stroke};

use crate::{
    engine::{EngineEvent, EngineSession, EngineStats},
    hotkeys::{HotkeyEvent, HotkeyService},
    overlay::Overlay,
    positions,
    profiles::{ProfileStore, load_or_default},
    settings::{
        ActivationMode, ClickKind, DestinationMode, Hotkey, MouseButton, Point, Settings, SpeedMode,
    },
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ShortcutTarget {
    StartStop,
    AddPosition,
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
    shortcut_capture: Option<ShortcutTarget>,
    shortcut_capture_deadline: Option<Instant>,
    confirm_clear_positions: bool,
    confirm_delete_profile: bool,
}

impl AutoClickerApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        cc.egui_ctx.set_visuals(egui::Visuals::dark());
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
            status: "Parado".to_owned(),
            notice: None,
            storage_warning,
            shortcut_capture: None,
            shortcut_capture_deadline: None,
            confirm_clear_positions: false,
            confirm_delete_profile: false,
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
                        self.status = "Erro no motor".to_owned();
                        self.notice = Some(error);
                    } else {
                        self.status = "Parado".to_owned();
                    }
                }
                Err(TryRecvError::Empty | TryRecvError::Disconnected) => break,
            }
        }
    }

    fn process_hotkey_events(&mut self) -> bool {
        let mut changed = false;
        loop {
            match self.hotkeys.events.try_recv() {
                Ok(HotkeyEvent::KeyDown { hotkey, cursor }) => {
                    if let Some(target) = self.shortcut_capture.take() {
                        self.shortcut_capture_deadline = None;
                        let settings = &mut self.profiles.active_mut().settings;
                        let other = match target {
                            ShortcutTarget::StartStop => settings.add_position_hotkey,
                            ShortcutTarget::AddPosition => settings.start_hotkey,
                        };
                        if hotkey == other {
                            self.notice =
                                Some("Esse atalho já está associado à outra ação.".to_owned());
                            self.shortcut_capture = Some(target);
                            self.shortcut_capture_deadline =
                                Some(Instant::now() + Duration::from_secs(10));
                        } else {
                            match target {
                                ShortcutTarget::StartStop => settings.start_hotkey = hotkey,
                                ShortcutTarget::AddPosition => {
                                    settings.add_position_hotkey = hotkey
                                }
                            }
                            changed = true;
                            self.notice = Some(format!("Atalho definido: {}", hotkey.label()));
                        }
                        continue;
                    }

                    let settings = self.profiles.active().settings.clone();
                    if hotkey == settings.add_position_hotkey {
                        if let Some(cursor) = cursor {
                            positions::append(
                                &mut self.profiles.active_mut().settings.positions,
                                cursor,
                            );
                            changed = true;
                            let count = self.profiles.active().settings.positions.len();
                            self.notice = Some(format!(
                                "Posição {count} registrada: X={}, Y={}",
                                cursor.x, cursor.y
                            ));
                            self.overlay
                                .set_points(self.profiles.active().settings.positions.clone());
                        } else {
                            self.notice =
                                Some("O Windows não conseguiu ler a posição do cursor.".to_owned());
                        }
                    }
                    if hotkey == settings.start_hotkey {
                        if settings.activation_mode == ActivationMode::Toggle {
                            self.toggle_engine();
                        } else if self.engine.is_none() {
                            self.start_engine(settings);
                        }
                    }
                }
                Ok(HotkeyEvent::KeyUp { hotkey }) => {
                    let settings = self.profiles.active().settings.clone();
                    if settings.activation_mode == ActivationMode::Hold
                        && hotkey == settings.start_hotkey
                    {
                        self.stop_engine();
                    }
                }
                Err(TryRecvError::Empty | TryRecvError::Disconnected) => break,
            }
        }
        changed
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

    fn profile_section(&mut self, ui: &mut egui::Ui, dirty: &mut bool) {
        section_title(
            ui,
            "Perfil ativo",
            "As configurações e posições ficam salvas por perfil.",
        );
        ui.horizontal(|ui| {
            ui.label("Perfil:");
            let active = self.profiles.active_profile.clone();
            ui.add_enabled_ui(self.engine.is_none(), |ui| {
                egui::ComboBox::from_id_salt("active-profile")
                    .selected_text(active)
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
            ui.add_enabled(
                self.engine.is_none(),
                egui::TextEdit::singleline(&mut self.profile_name_input)
                    .hint_text("Nome do perfil")
                    .desired_width(160.0),
            );
        });
        ui.horizontal_wrapped(|ui| {
            if ui
                .add_enabled(self.engine.is_none(), egui::Button::new("Criar perfil"))
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
            ui.colored_label(Color32::LIGHT_RED, error);
        }
    }

    fn control_section(&mut self, ui: &mut egui::Ui, settings: &mut Settings) {
        section_title(
            ui,
            "Execução",
            "Atalhos funcionam enquanto outro aplicativo está em primeiro plano.",
        );
        ui.horizontal_wrapped(|ui| {
            let is_running = self.engine.is_some();
            let button = if is_running { "Parar" } else { "Iniciar" };
            let color = if is_running {
                Color32::from_rgb(182, 76, 77)
            } else {
                Color32::from_rgb(41, 143, 111)
            };
            if ui
                .add(
                    egui::Button::new(RichText::new(button).strong())
                        .fill(color)
                        .min_size([128.0, 42.0].into()),
                )
                .clicked()
            {
                if self.engine.is_some() {
                    self.stop_engine();
                } else {
                    self.start_engine(settings.clone());
                }
            }
            if ui
                .add(
                    egui::Button::new(RichText::new("PARADA DE EMERGÊNCIA").strong())
                        .fill(Color32::from_rgb(153, 43, 53)),
                )
                .clicked()
            {
                self.stop_engine();
            }
            ui.separator();
            ui.label("Estado:");
            let status_color = if self.engine.is_some() {
                Color32::LIGHT_GREEN
            } else {
                Color32::LIGHT_GRAY
            };
            ui.colored_label(status_color, RichText::new(&self.status).strong());
        });

        ui.horizontal_wrapped(|ui| {
            ui.label("Ativação:");
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
            ui.label("Atalho iniciar/parar:");
            hotkey_editor(ui, "start-stop", &mut settings.start_hotkey);
            if ui
                .button(
                    if self.shortcut_capture == Some(ShortcutTarget::StartStop) {
                        "Pressione uma tecla…"
                    } else {
                        "Capturar"
                    },
                )
                .clicked()
            {
                self.shortcut_capture = Some(ShortcutTarget::StartStop);
                self.shortcut_capture_deadline = Some(Instant::now() + Duration::from_secs(10));
                self.notice =
                    Some("Pressione a combinação desejada em até 10 segundos.".to_owned());
            }
        });

        if settings.start_hotkey.is_configured()
            && settings.start_hotkey == settings.add_position_hotkey
        {
            ui.colored_label(
                Color32::LIGHT_RED,
                "Os atalhos de execução e captura não podem ser iguais.",
            );
        }
        if !self.hotkeys.is_available() {
            ui.colored_label(Color32::YELLOW, "Recursos de mouse e atalhos globais exigem Windows; no WSL esta janela serve para configurar perfis.");
        }
    }

    fn speed_section(&mut self, ui: &mut egui::Ui, settings: &mut Settings) {
        section_title(
            ui,
            "Velocidade",
            "CPS e intervalo são sincronizados. O modo sem limite remove a espera entre cliques.",
        );
        ui.horizontal_wrapped(|ui| {
            ui.label("Modo:");
            egui::ComboBox::from_id_salt("speed-mode")
                .selected_text(match settings.speed_mode {
                    SpeedMode::Normal => "Normal",
                    SpeedMode::HighPrecision => "Alta precisão",
                    SpeedMode::Unlimited => "Sem limite intencional",
                })
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut settings.speed_mode, SpeedMode::Normal, "Normal");
                    ui.selectable_value(
                        &mut settings.speed_mode,
                        SpeedMode::HighPrecision,
                        "Alta precisão",
                    );
                    ui.selectable_value(
                        &mut settings.speed_mode,
                        SpeedMode::Unlimited,
                        "Sem limite intencional",
                    );
                });
        });
        ui.add_enabled_ui(self.engine.is_none(), |ui| {
            ui.horizontal(|ui| {
                ui.label("CPS:");
                let mut cps = settings.cps;
                if ui
                    .add(
                        egui::DragValue::new(&mut cps)
                            .range(0.1..=100_000.0)
                            .speed(0.2)
                            .suffix(" cliques/s"),
                    )
                    .changed()
                {
                    settings.set_cps(cps);
                }
                ui.label("Intervalo:");
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
            ui.horizontal(|ui| {
                ui.label("Pressionamento:");
                ui.add(
                    egui::DragValue::new(&mut settings.press_duration_ms)
                        .range(0.0..=60_000.0)
                        .speed(0.1)
                        .suffix(" ms"),
                );
                ui.label("Botão:");
                egui::ComboBox::from_id_salt("mouse-button")
                    .selected_text(match settings.mouse_button {
                        MouseButton::Left => "Esquerdo",
                        MouseButton::Right => "Direito",
                        MouseButton::Middle => "Central",
                    })
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
                ui.label("Clique:");
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
        if settings.speed_mode == SpeedMode::Unlimited {
            ui.colored_label(Color32::LIGHT_BLUE, "Sem limite remove o intervalo artificial. A taxa final depende do scheduler, do Windows e do aplicativo receptor.");
        } else if let Some(warning) = settings.scheduler_warning() {
            ui.colored_label(Color32::YELLOW, warning);
        }
    }

    fn destination_section(&mut self, ui: &mut egui::Ui, settings: &mut Settings) {
        section_title(
            ui,
            "Destino e repetição",
            "A posição atual não move o cursor; destinos fixos e sequenciais movem o cursor do Windows.",
        );
        ui.horizontal_wrapped(|ui| {
            ui.selectable_value(
                &mut settings.destination_mode,
                DestinationMode::CurrentCursor,
                "Cursor atual",
            );
            ui.selectable_value(
                &mut settings.destination_mode,
                DestinationMode::FixedPosition,
                "Posição fixa",
            );
            ui.selectable_value(
                &mut settings.destination_mode,
                DestinationMode::PositionSequence,
                "Sequência de posições",
            );
        });
        if settings.destination_mode == DestinationMode::FixedPosition {
            ui.horizontal(|ui| {
                ui.label("X:");
                ui.add(egui::DragValue::new(&mut settings.fixed_position.x));
                ui.label("Y:");
                ui.add(egui::DragValue::new(&mut settings.fixed_position.y));
            });
        }
        if settings.destination_mode == DestinationMode::PositionSequence {
            ui.horizontal_wrapped(|ui| {
                ui.label("Cliques por posição:");
                ui.add(
                    egui::DragValue::new(&mut settings.clicks_per_position).range(1..=1_000_000),
                );
                ui.label("Pausa entre posições:");
                ui.add(
                    egui::DragValue::new(&mut settings.interval_between_positions_ms)
                        .range(0.0..=600_000.0)
                        .speed(1.0)
                        .suffix(" ms"),
                );
            });
        }
        ui.horizontal_wrapped(|ui| {
            ui.checkbox(&mut settings.repeat_continuously, "Repetir continuamente");
            if !settings.repeat_continuously {
                ui.label("Ciclos:");
                ui.add(egui::DragValue::new(&mut settings.cycles).range(1..=1_000_000));
            }
            let mut limit_enabled = settings.max_clicks.is_some();
            if ui
                .checkbox(&mut limit_enabled, "Limitar cliques totais")
                .changed()
            {
                settings.max_clicks = limit_enabled.then_some(1);
            }
            if let Some(max_clicks) = &mut settings.max_clicks {
                ui.add(egui::DragValue::new(max_clicks).range(1..=u64::MAX));
            } else if !settings.repeat_continuously {
                ui.label("(Cursor atual/fixo executa uma vez)");
            }
        });
    }

    fn positions_section(&mut self, ui: &mut egui::Ui, settings: &mut Settings) {
        section_title(
            ui,
            "Posições de clique",
            "Pressione o atalho global no ponto desejado; o cursor não será movido.",
        );
        ui.horizontal_wrapped(|ui| {
            ui.strong(format!(
                "Posições registradas: {}",
                settings.positions.len()
            ));
            ui.label("Atalho:");
            hotkey_editor(ui, "add-position", &mut settings.add_position_hotkey);
            if ui
                .button(
                    if self.shortcut_capture == Some(ShortcutTarget::AddPosition) {
                        "Pressione uma tecla…"
                    } else {
                        "Capturar"
                    },
                )
                .clicked()
            {
                self.shortcut_capture = Some(ShortcutTarget::AddPosition);
                self.shortcut_capture_deadline = Some(Instant::now() + Duration::from_secs(10));
                self.notice =
                    Some("Pressione a combinação desejada em até 10 segundos.".to_owned());
            }
            let show_label = if self.overlay.is_visible() {
                "Ocultar posições"
            } else {
                "Mostrar posições na tela"
            };
            if ui.button(show_label).clicked() {
                match self.overlay.toggle(settings.positions.clone()) {
                    Ok(visible) => {
                        self.notice = Some(
                            if visible {
                                "Marcadores exibidos."
                            } else {
                                "Marcadores ocultos."
                            }
                            .to_owned(),
                        )
                    }
                    Err(error) => self.notice = Some(error),
                }
            }
            if ui.button("Adicionar coordenada").clicked() {
                positions::append(&mut settings.positions, Point::default());
            }
            if ui.button("Limpar posições").clicked() && !settings.positions.is_empty() {
                if settings.positions.len() > 1 {
                    self.confirm_clear_positions = true;
                } else {
                    settings.positions.clear();
                }
            }
        });

        if settings.start_hotkey.is_configured()
            && settings.start_hotkey == settings.add_position_hotkey
        {
            ui.colored_label(
                Color32::LIGHT_RED,
                "Este atalho conflita com iniciar/parar; escolha combinações diferentes.",
            );
        }

        egui::Frame::group(ui.style()).show(ui, |ui| {
            egui::ScrollArea::vertical()
                .max_height(230.0)
                .show(ui, |ui| {
                    if settings.positions.is_empty() {
                        ui.label(RichText::new("Nenhuma posição registrada ainda.").weak());
                    }
                    let mut remove_index = None;
                    for index in 0..settings.positions.len() {
                        ui.horizontal(|ui| {
                            ui.label(format!("{}.", index + 1));
                            ui.label("X");
                            ui.add(egui::DragValue::new(&mut settings.positions[index].x));
                            ui.label("Y");
                            ui.add(egui::DragValue::new(&mut settings.positions[index].y));
                            if ui.add_enabled(index > 0, egui::Button::new("↑")).clicked() {
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
                                ui.ctx().request_repaint();
                            }
                        });
                    }
                    if let Some(index) = remove_index {
                        positions::remove(&mut settings.positions, index);
                    }
                });
        });
    }

    fn performance_section(&self, ui: &mut egui::Ui, settings: &Settings) {
        section_title(
            ui,
            "Desempenho",
            "Métricas contam eventos aceitos por SendInput, não o que outro aplicativo reconheceu.",
        );
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
        egui::Grid::new("performance-grid")
            .num_columns(4)
            .spacing([18.0, 6.0])
            .show(ui, |ui| {
                metric(
                    ui,
                    "Configurado",
                    if settings.speed_mode == SpeedMode::Unlimited {
                        "Sem limite".to_owned()
                    } else {
                        format!("{:.2} CPS", settings.cps)
                    },
                );
                metric(ui, "Enviado", format!("{cps:.2} CPS"));
                metric(
                    ui,
                    "Cliques nesta execução",
                    self.stats.clicks_sent.to_string(),
                );
                metric(ui, "Total nesta sessão", self.total_clicks.to_string());
                ui.end_row();
                metric(ui, "Duração", format_duration(self.stats.elapsed));
                metric(
                    ui,
                    "Falhas",
                    format!("{} ({failure_rate:.2}%)", self.stats.failed_events),
                );
                metric(
                    ui,
                    "Scheduler",
                    match settings.speed_mode {
                        SpeedMode::Normal => "Normal".to_owned(),
                        SpeedMode::HighPrecision => "Alta precisão".to_owned(),
                        SpeedMode::Unlimited => "Sem espera".to_owned(),
                    },
                );
                metric(ui, "CPU", "Não medido".to_owned());
                ui.end_row();
            });
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
        if self.confirm_clear_positions {
            egui::Window::new("Limpar posições")
                .collapsible(false)
                .resizable(false)
                .show(ctx, |ui| {
                    ui.label(format!("Remover as {} posições deste perfil? As outras configurações serão mantidas.", settings.positions.len()));
                    ui.horizontal(|ui| {
                        if ui.button("Cancelar").clicked() {
                            self.confirm_clear_positions = false;
                        }
                        if ui.button("Limpar").clicked() {
                            settings.positions.clear();
                            self.confirm_clear_positions = false;
                            *dirty = true;
                        }
                    });
                });
        }
        if self.confirm_delete_profile {
            egui::Window::new("Excluir perfil")
                .collapsible(false)
                .resizable(false)
                .show(ctx, |ui| {
                    ui.label(format!(
                        "Excluir o perfil ‘{}’?",
                        self.profiles.active_profile
                    ));
                    ui.horizontal(|ui| {
                        if ui.button("Cancelar").clicked() {
                            self.confirm_delete_profile = false;
                        }
                        if ui.button("Excluir perfil").clicked() {
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
        if self
            .shortcut_capture_deadline
            .is_some_and(|deadline| Instant::now() >= deadline)
        {
            self.shortcut_capture = None;
            self.shortcut_capture_deadline = None;
            self.notice = Some("Tempo para capturar o atalho esgotado.".to_owned());
        }
        ctx.request_repaint_after(Duration::from_millis(100));

        egui::TopBottomPanel::top("app-header").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading(
                    RichText::new("Rust AutoClicker").color(Color32::from_rgb(229, 185, 112)),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(
                        RichText::new(if self.engine.is_some() {
                            "● EXECUTANDO"
                        } else {
                            "● PRONTO"
                        })
                        .color(if self.engine.is_some() {
                            Color32::LIGHT_GREEN
                        } else {
                            Color32::LIGHT_GRAY
                        }),
                    );
                });
            });
        });

        egui::TopBottomPanel::top("profile-panel").show(ctx, |ui| {
            egui::Frame::group(ui.style())
                .stroke(Stroke::new(1.0_f32, Color32::from_gray(62)))
                .show(ui, |ui| self.profile_section(ui, &mut dirty));
        });

        let mut settings = self.profiles.active().settings.clone();
        let original = settings.clone();
        egui::CentralPanel::default().show(ctx, |ui| {
            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    egui::Frame::group(ui.style())
                        .show(ui, |ui| self.control_section(ui, &mut settings));
                    ui.add_space(8.0);
                    egui::Frame::group(ui.style()).show(ui, |ui| {
                        self.speed_section(ui, &mut settings);
                        ui.separator();
                        self.destination_section(ui, &mut settings);
                    });
                    ui.add_space(8.0);
                    egui::Frame::group(ui.style())
                        .show(ui, |ui| self.positions_section(ui, &mut settings));
                    ui.add_space(8.0);
                    egui::Frame::group(ui.style())
                        .show(ui, |ui| self.performance_section(ui, &settings));
                    if let Some(notice) = &self.notice {
                        ui.add_space(6.0);
                        ui.colored_label(Color32::LIGHT_BLUE, notice);
                    }
                    if let Some(warning) = &self.storage_warning {
                        ui.add_space(6.0);
                        ui.colored_label(Color32::YELLOW, warning);
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

fn section_title(ui: &mut egui::Ui, title: &str, subtitle: &str) {
    ui.horizontal(|ui| {
        ui.label(RichText::new(title).strong().size(17.0));
        ui.label(RichText::new(subtitle).weak().small());
    });
    ui.add_space(5.0);
}

fn metric(ui: &mut egui::Ui, label: &str, value: String) {
    ui.vertical(|ui| {
        ui.label(RichText::new(label).weak().small());
        ui.label(RichText::new(value).strong());
    });
}

fn format_duration(duration: Duration) -> String {
    let seconds = duration.as_secs();
    let millis = duration.subsec_millis();
    format!(
        "{:02}:{:02}:{:02}.{:03}",
        seconds / 3600,
        (seconds / 60) % 60,
        seconds % 60,
        millis
    )
}

fn hotkey_editor(ui: &mut egui::Ui, id: &str, hotkey: &mut Hotkey) {
    egui::ComboBox::from_id_salt(id)
        .selected_text(hotkey.label())
        .width(128.0)
        .show_ui(ui, |ui| {
            ui.selectable_value(hotkey, Hotkey::default(), "Não configurado");
            for code in (0x41..=0x5a)
                .chain(0x30..=0x39)
                .chain(0x70..=0x87)
                .chain([0x20, 0x1b, 0x09, 0x2d, 0x2e])
            {
                let candidate = Hotkey {
                    key_code: code,
                    modifiers: hotkey.modifiers,
                };
                ui.selectable_value(hotkey, candidate, candidate.label());
            }
        });
    ui.checkbox(&mut hotkey.modifiers.control, "Ctrl");
    ui.checkbox(&mut hotkey.modifiers.shift, "Shift");
    ui.checkbox(&mut hotkey.modifiers.alt, "Alt");
    ui.checkbox(&mut hotkey.modifiers.windows, "Win");
}
