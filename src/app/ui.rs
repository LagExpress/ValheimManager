use super::*;
use components::{badge, card, field, path, primary};

#[cfg(test)]
mod tests;

fn tr(language: Language, en: &'static str, es: &'static str) -> &'static str {
    if language == Language::Spanish {
        es
    } else {
        en
    }
}

impl eframe::App for ModpackApp {
    fn clear_color(&self, _: &egui::Visuals) -> [f32; 4] {
        [0.0; 4]
    }

    fn on_exit(&mut self) {
        self.effects.shutdown();
    }

    fn ui(&mut self, ui: &mut Ui, frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        if !self.initial_size_set {
            if let Some(monitor) = ctx.input(|i| i.viewport().monitor_size) {
                ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(egui::vec2(
                    1100.0_f32.min(monitor.x - 48.0).max(640.0),
                    720.0_f32.min(monitor.y - 80.0).max(480.0),
                )));
            }
            self.initial_size_set = true;
        }
        let native = self.effects.update(frame, self.theme, ui.max_rect().size());
        if self.applied_theme != Some((self.theme, native)) {
            Palette::new(self.theme, native).apply(&ctx, self.theme);
            self.applied_theme = Some((self.theme, native));
        }
        self.poll_sync_events();
        self.poll_build_events();
        if let Some(receiver) = &self.open_receiver {
            if let Ok(result) = receiver.try_recv() {
                self.status_message = Some(match result {
                    Ok(()) => (
                        tr(
                            self.language,
                            "Output folder opened.",
                            "Carpeta de salida abierta.",
                        )
                        .into(),
                        false,
                    ),
                    Err(error) => (error, true),
                });
                self.open_receiver = None;
            }
        }
        self.render(ui);
    }
}

impl ModpackApp {
    fn render(&mut self, ui: &mut Ui) {
        ui.ctx().data_mut(|data| {
            data.insert_temp(
                egui::Id::unique("spanish_ui"),
                self.language == Language::Spanish,
            )
        });
        ui.ctx()
            .data_mut(|data| data.insert_temp(egui::Id::unique("glass_ui"), self.theme.glass()));
        let p = Palette::new(self.theme, self.effects.active());
        components::backdrop(ui, p.background, p.accent, self.theme.glass());
        egui::Panel::top("header")
            .frame(
                egui::Frame::new()
                    .fill(p.surface)
                    .inner_margin(egui::Margin::symmetric(16, 12)),
            )
            .show(ui, |ui| self.header(ui));
        egui::Panel::bottom("status")
            .frame(
                egui::Frame::new()
                    .fill(p.surface)
                    .inner_margin(egui::Margin::symmetric(16, 7)),
            )
            .show(ui, |ui| {
                let (message, color) = if let Some((message, error)) = &self.status_message {
                    (message.clone(), if *error { p.danger } else { p.success })
                } else {
                    (
                        self.language.status_summary(
                            self.mod_manager.label(),
                            &self.selected_profile,
                            self.mods.iter().filter(|m| m.selected).count(),
                            self.configs.iter().filter(|c| c.selected).count(),
                        ),
                        p.muted,
                    )
                };
                ui.add(egui::Label::new(RichText::new(&message).small().color(color)).truncate())
                    .on_hover_text(message);
            });
        egui::CentralPanel::default()
            .frame(
                egui::Frame::new().inner_margin(if self.active_tab == ActiveTab::ServerSync {
                    egui::Margin {
                        left: 0,
                        right: 0,
                        top: 16,
                        bottom: 16,
                    }
                } else {
                    egui::Margin::same(16)
                }),
            )
            .show(ui, |ui| {
                if self.active_tab == ActiveTab::ServerSync {
                    self.ui_server_sync(ui);
                    return;
                }
                let width = ui.available_width().min(components::MAX_CONTENT_WIDTH);
                let available = ui.available_rect_before_wrap();
                let rect = egui::Rect::from_center_size(
                    available.center(),
                    egui::vec2(width, available.height()),
                );
                ui.scope_builder(
                    egui::UiBuilder::new()
                        .max_rect(rect)
                        .layout(egui::Layout::top_down(egui::Align::Min)),
                    |ui| match self.active_tab {
                        ActiveTab::ProfileMetadata => self.ui_profile_metadata(ui),
                        ActiveTab::Mods => self.ui_mods_selection(ui),
                        ActiveTab::Configs => self.ui_configs_selection(ui),
                        ActiveTab::Build => self.ui_build_export(ui),
                        ActiveTab::ServerSync => self.ui_server_sync(ui),
                        ActiveTab::Help => self.ui_help(ui),
                    },
                );
            });
        self.build_version_dialog(ui.ctx());
    }

    fn saved_configurations(&mut self, ui: &mut Ui) {
        let language = self.language;
        card(
            ui,
            tr(
                language,
                "Saved modpack configurations",
                "Configuraciones de modpack guardadas",
            ),
            |ui| {
                let active = self.modpack_library.active_id;
                let label = active
                    .and_then(|id| self.modpack_library.configuration(id))
                    .map(|c| format!("{} · {}", c.name, c.version))
                    .unwrap_or_else(|| {
                        tr(
                            language,
                            "Unsaved configuration",
                            "Configuración sin guardar",
                        )
                        .to_owned()
                    });
                let mut load = None;
                egui::ComboBox::from_id_salt("saved_modpacks")
                    .width(ui.available_width().min(480.0))
                    .truncate()
                    .selected_text(label)
                    .show_ui(ui, |ui| {
                        for configuration in &self.modpack_library.configurations {
                            if ui
                                .selectable_label(
                                    active == Some(configuration.id),
                                    format!("{} · {}", configuration.name, configuration.version),
                                )
                                .clicked()
                            {
                                load = Some(configuration.id);
                            }
                        }
                    });
                if let Some(id) = load {
                    if let Err(error) = self.load_modpack_configuration(id) {
                        self.status_message = Some((error, true));
                    }
                }
                ui.horizontal_wrapped(|ui| {
                    let save = primary(
                        ui,
                        tr(language, "Save configuration", "Guardar configuración"),
                    )
                    .clicked();
                    let save_new = ui
                        .add_enabled(
                            active.is_some(),
                            egui::Button::new(tr(language, "Save as new", "Guardar como nueva")),
                        )
                        .clicked();
                    if save || save_new {
                        self.status_message =
                            Some(match self.save_modpack_configuration(save_new) {
                                Ok(_) => (
                                    tr(
                                        language,
                                        "Modpack configuration saved.",
                                        "Configuración del modpack guardada.",
                                    )
                                    .into(),
                                    false,
                                ),
                                Err(error) => (error, true),
                            });
                    }
                });
                ui.label(RichText::new(tr(language,
                "Restores the last saved modpack at startup. Builds save your metadata, profile, output folder, and selections automatically.",
                "Restaura el último modpack guardado al iniciar. Cada compilación guarda los datos, el perfil, la carpeta de salida y las selecciones.")).small().weak());
                if let Some(error) = &self.modpack_library_error {
                    ui.colored_label(Palette::for_ui(ui).danger, error);
                }
            },
        );
    }

    fn increment_preference(&mut self, ui: &mut Ui) {
        let previous = self.always_increment_version;
        if ui
            .add_enabled(
                !self.building,
                egui::Checkbox::new(
                    &mut self.always_increment_version,
                    tr(
                        self.language,
                        "Always increment the patch version when building a ZIP",
                        "Incrementar siempre la versión de parche al crear un ZIP",
                    ),
                ),
            )
            .changed()
        {
            if let Err(error) = self.save_modpack_configuration(false) {
                self.always_increment_version = previous;
                self.status_message = Some((error, true));
            }
        }
    }

    fn build_version_dialog(&mut self, ctx: &egui::Context) {
        let Some(prompt) = self.build_version_prompt.as_mut() else {
            return;
        };
        let language = self.language;
        let mut choice = None;
        let dialog_theme = if self.theme.dark() {
            ThemePreset::Dark
        } else {
            ThemePreset::Light
        };
        let style = ctx.style_of(if dialog_theme.dark() {
            egui::Theme::Dark
        } else {
            egui::Theme::Light
        });
        let frame = egui::Frame::popup(&style).fill(Palette::new(dialog_theme, false).surface);
        let response = egui::Modal::new(egui::Id::unique("build_version_choice")).frame(frame).show(ctx, |ui| {
            ui.set_width((ctx.content_rect().width() - 64.0).clamp(240.0, 460.0));
            ui.heading(tr(language, "Choose the ZIP version", "Elige la versión del ZIP"));
            ui.label(format!("{}: {}", tr(language, "Current version", "Versión actual"), self.pack_version));
            ui.label(tr(language, "Increment the patch version or rebuild the same version? The saved version advances only after a successful build.",
                "¿Incrementar la versión de parche o conservar la actual? La versión guardada solo aumenta si la compilación finaliza correctamente."));
            if let Err(error) = &prompt.next_version { ui.colored_label(Palette::for_ui(ui).danger, error); }
            ui.add_enabled(prompt.next_version.is_ok(), egui::Checkbox::new(&mut prompt.always_increment,
                tr(language, "Always increment when building a ZIP", "Incrementar siempre al crear un ZIP")));
            ui.horizontal_wrapped(|ui| {
                if let Ok(next) = &prompt.next_version {
                    if primary(ui, &format!("{} {next}", tr(language, "Increment to", "Incrementar a"))).clicked() { choice = Some(true); }
                }
                if ui.button(format!("{} {}", tr(language, "Keep", "Conservar"), self.pack_version)).clicked() { choice = Some(false); }
                if ui.button(tr(language, "Cancel", "Cancelar")).clicked() { ui.close(); }
            });
        });
        let always = prompt.always_increment;
        if let Some(increment) = choice {
            self.start_versioned_build(ctx, increment, always);
        } else if response.should_close() {
            self.build_version_prompt = None;
        }
    }

    fn header(&mut self, ui: &mut Ui) {
        let p = Palette::for_ui(ui);
        let compact = ui.available_width() < 800.0;
        ui.horizontal(|ui| {
            components::icon(ui, Icon::Shield, 25.0, p.accent);
            ui.label(
                RichText::new(if compact {
                    "Valheim Studio"
                } else {
                    "Valheim Modpack Studio"
                })
                .size(18.0)
                .strong(),
            );
            if !compact {
                ui.label(
                    RichText::new(format!("v{}", env!("CARGO_PKG_VERSION")))
                        .small()
                        .color(p.muted),
                );
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui
                    .add_enabled(
                        !self.building,
                        egui::Button::new(
                            RichText::new(tr(self.language, "Build ZIP", "Crear ZIP"))
                                .color(p.on_accent)
                                .strong(),
                        )
                        .fill(p.accent),
                    )
                    .clicked()
                {
                    self.trigger_build(ui.ctx());
                }
                if self.building {
                    ui.spinner();
                }
                let appearance_label = tr(self.language, "Appearance", "Apariencia");
                ui.menu_button(appearance_label, |ui| {
                    ui.set_min_width(220.0);
                    ui.label(RichText::new(appearance_label).strong());
                    for theme in ThemePreset::ALL {
                        ui.horizontal(|ui| {
                            let palette = Palette::new(theme, false);
                            components::icon(
                                ui,
                                if theme.dark() { Icon::Moon } else { Icon::Sun },
                                17.0,
                                palette.accent,
                            );
                            if ui
                                .selectable_label(
                                    self.theme == theme,
                                    theme.label(self.language == Language::Spanish),
                                )
                                .clicked()
                            {
                                self.theme = theme;
                                self.preferences.theme = Some(theme);
                                if let Err(error) = self.preferences.save(&UiPreferences::path()) {
                                    self.status_message = Some((error, true));
                                }
                                self.applied_theme = None;
                                ui.ctx().request_repaint();
                            }
                        });
                    }
                    if ui
                        .small_button(tr(
                            self.language,
                            "Use system default",
                            "Usar tema del sistema",
                        ))
                        .clicked()
                    {
                        self.preferences.theme = None;
                        self.theme = self.preferences.resolve(ui.ctx());
                        if let Err(error) = self.preferences.save(&UiPreferences::path()) {
                            self.status_message = Some((error, true));
                        }
                        self.applied_theme = None;
                    }
                    if self.theme.glass() {
                        ui.label(
                            RichText::new(if self.effects.active() {
                                tr(
                                    self.language,
                                    "Desktop blur • controlled by your system",
                                    "Desenfoque del escritorio • controlado por el sistema",
                                )
                            } else {
                                tr(
                                    self.language,
                                    "Glass backdrop • native blur unavailable",
                                    "Fondo de cristal • desenfoque nativo no disponible",
                                )
                            })
                            .small()
                            .weak(),
                        );
                    }
                    ui.separator();
                    ui.horizontal(|ui| {
                        let english_changed = ui
                            .selectable_value(&mut self.language, Language::English, "EN")
                            .changed();
                        let spanish_changed = ui
                            .selectable_value(&mut self.language, Language::Spanish, "ES")
                            .changed();
                        if english_changed || spanish_changed {
                            self.sync.config.language = Some(self.language);
                            if let Err(error) = self.sync.config.save() {
                                self.status_message = Some((error, true));
                            }
                        }
                    });
                    ui.label(
                        RichText::new(format!(
                            "{} {}",
                            tr(self.language, "Version", "Versión"),
                            env!("CARGO_PKG_VERSION")
                        ))
                        .small()
                        .weak(),
                    );
                    ui.collapsing("Inter · SIL Open Font License", |ui| {
                        egui::ScrollArea::vertical()
                            .max_height(120.0)
                            .show(ui, |ui| {
                                ui.add(
                                    egui::Label::new(
                                        RichText::new(include_str!("../assets/Inter-LICENSE.txt"))
                                            .small(),
                                    )
                                    .wrap(),
                                );
                            });
                    });
                });
                if !compact {
                    ui.label(RichText::new(self.language.code()).small().weak());
                }
            });
        });
        ui.add_space(7.0);
        let tabs = [
            (
                ActiveTab::ProfileMetadata,
                self.language.tab_profile().to_owned(),
                Icon::Folder,
            ),
            (
                ActiveTab::Mods,
                format!(
                    "{} ({})",
                    self.language.tab_mods(),
                    self.mods.iter().filter(|m| m.selected).count()
                ),
                Icon::Layers,
            ),
            (
                ActiveTab::Configs,
                format!(
                    "{} ({})",
                    self.language.tab_configs(),
                    self.configs.iter().filter(|c| c.selected).count()
                ),
                Icon::Layers,
            ),
            (
                ActiveTab::Build,
                self.language.tab_build().to_owned(),
                Icon::Build,
            ),
            (
                ActiveTab::ServerSync,
                self.language.tab_server_sync().to_owned(),
                Icon::Server,
            ),
            (
                ActiveTab::Help,
                self.language.tab_help().to_owned(),
                Icon::Info,
            ),
        ];
        let needed: f32 = tabs
            .iter()
            .map(|(_, label, _)| {
                ui.painter()
                    .layout_no_wrap(label.clone(), egui::FontId::proportional(14.0), p.text)
                    .size()
                    .x
                    + 16.0 // vector icon
                    + ui.spacing().item_spacing.x
                    + 2.0 * ui.spacing().button_padding.x
            })
            .sum::<f32>()
            + (tabs.len() - 1) as f32 * ui.spacing().item_spacing.x;
        if needed <= ui.available_width() {
            let _right_edge = ui.max_rect().right();
            let _navigation = ui.horizontal(|ui| {
                for (tab, label, icon) in tabs {
                    ui.horizontal(|ui| {
                        components::icon(
                            ui,
                            icon,
                            16.0,
                            if self.active_tab == tab {
                                p.accent
                            } else {
                                p.muted
                            },
                        );
                        if ui.selectable_label(self.active_tab == tab, label).clicked() {
                            self.active_tab = tab;
                        }
                    });
                }
            });
            #[cfg(test)]
            assert!(
                _navigation.response.rect.right() <= _right_edge + 1.0,
                "Localized navigation overflowed its available width"
            );
        } else {
            let selected = tabs
                .iter()
                .find(|(tab, _, _)| *tab == self.active_tab)
                .map(|(_, label, _)| label.as_str())
                .unwrap_or("");
            egui::ComboBox::from_id_salt("compact_navigation")
                .width(ui.available_width().min(320.0))
                .selected_text(selected)
                .show_ui(ui, |ui| {
                    for (tab, label, _) in tabs {
                        ui.selectable_value(&mut self.active_tab, tab, label);
                    }
                });
        }
    }

    fn page_heading(&self, ui: &mut Ui, title: &str, description: &str) {
        ui.heading(title);
        if !description.is_empty() {
            if ui.available_height() < 360.0 {
                ui.label(RichText::new(description).small().weak());
            } else {
                ui.label(RichText::new(description).weak());
            }
        }
        ui.add_space(12.0);
    }

    fn ui_profile_metadata(&mut self, ui: &mut Ui) {
        self.page_heading(
            ui,
            self.language.tab_profile(),
            tr(
                self.language,
                "Choose your source profile and shape your next modpack.",
                "Elige tu perfil de origen y prepara tu próximo modpack.",
            ),
        );
        egui::ScrollArea::vertical()
            .id_salt("profile_scroll")
            .show(ui, |ui| {
                ui.add_enabled_ui(!self.building, |ui| {
                    self.saved_configurations(ui);
                    if ui.available_width() >= 1000.0 {
                        ui.columns(2, |columns| {
                            self.source_profile(&mut columns[0]);
                            self.package_metadata(&mut columns[1]);
                        });
                    } else {
                        self.source_profile(ui);
                        self.package_metadata(ui);
                    }
                });
                if primary(ui, self.language.btn_proceed_to_mods()).clicked() {
                    self.active_tab = ActiveTab::Mods;
                }
            });
    }

    fn source_profile(&mut self, ui: &mut Ui) {
        let language = self.language;
        card(ui, self.language.heading_mod_manager(), |ui| {
            ui.horizontal_wrapped(|ui| {
                for (manager, label) in [
                    (ModManager::R2Modman, "r2modman"),
                    (ModManager::Gale, "Gale"),
                    (
                        ModManager::Custom,
                        tr(language, "Custom folder", "Carpeta personalizada"),
                    ),
                ] {
                    if ui
                        .selectable_label(self.mod_manager == manager, label)
                        .clicked()
                    {
                        self.mod_manager = manager;
                        if manager != ModManager::Custom {
                            let roots = if manager == ModManager::Gale {
                                get_gale_roots()
                            } else {
                                get_r2modman_roots()
                            };
                            self.selected_root = roots.first().cloned();
                            self.refresh_profiles();
                        }
                    }
                }
            });
            let directory = self
                .selected_root
                .as_ref()
                .map(|p| p.display().to_string())
                .unwrap_or_else(|| self.language.not_found().into());
            path(ui, self.language.label_profiles_dir(), &directory);
            if ui.button(self.language.btn_browse()).clicked() {
                if let Some(folder) = rfd::FileDialog::new().pick_folder() {
                    self.selected_root = Some(folder);
                    self.mod_manager = ModManager::Custom;
                    self.refresh_profiles();
                }
            }
            ui.label(
                RichText::new(self.language.label_selected_profile())
                    .small()
                    .strong(),
            );
            let mut changed = false;
            egui::ComboBox::from_id_salt("profile_select")
                .width(ui.available_width().min(400.0))
                .selected_text(if self.selected_profile.is_empty() {
                    tr(language, "Choose a profile", "Elige un perfil")
                } else {
                    &self.selected_profile
                })
                .show_ui(ui, |ui| {
                    for profile in &self.available_profiles {
                        changed |= ui
                            .selectable_value(&mut self.selected_profile, profile.clone(), profile)
                            .changed();
                    }
                });
            if changed {
                self.load_selected_profile_data();
            }
            ui.horizontal_wrapped(|ui| {
                if ui.button(self.language.btn_reload()).clicked() {
                    self.refresh_profiles();
                }
                ui.label(
                    RichText::new(self.language.label_profile_stats(
                        self.mods.len(),
                        self.mods.iter().filter(|m| m.enabled).count(),
                    ))
                    .small()
                    .weak(),
                );
            });
        });
        card(ui, self.language.heading_output_dir(), |ui| {
            path(
                ui,
                tr(language, "Destination", "Destino"),
                &self.output_dir.display().to_string(),
            );
            ui.label(
                RichText::new(self.language.label_output_dir_desc())
                    .small()
                    .weak(),
            );
            if ui.button(self.language.btn_browse()).clicked() {
                if let Some(folder) = rfd::FileDialog::new().pick_folder() {
                    self.output_dir = folder;
                }
            }
        });
    }

    fn package_metadata(&mut self, ui: &mut Ui) {
        let language = self.language;
        card(ui, self.language.heading_metadata(), |ui| {
            if field(ui, self.language.label_package_name(), &mut self.pack_name).changed() {
                self.pack_name = self
                    .pack_name
                    .replace(|c: char| !c.is_alphanumeric() && c != '_', "_");
            }
            field(ui, self.language.label_version(), &mut self.pack_version);
            self.increment_preference(ui);
            ui.label(
                RichText::new(tr(
                    language,
                    "Semantic version, for example 1.0.0",
                    "Versión semántica, por ejemplo 1.0.0",
                ))
                .small()
                .weak(),
            );
            ui.label(
                RichText::new(self.language.label_description())
                    .small()
                    .strong(),
            );
            ui.add(
                egui::TextEdit::multiline(&mut self.pack_description)
                    .desired_rows(3)
                    .desired_width(f32::INFINITY),
            );
            field(ui, self.language.label_website(), &mut self.pack_website);
            ui.label(RichText::new(self.language.label_icon()).small().strong());
            if let Some(icon) = &self.custom_icon_path {
                ui.add(egui::Label::new(icon.display().to_string()).truncate());
                if ui.button(self.language.btn_remove()).clicked() {
                    self.custom_icon_path = None;
                }
            } else {
                ui.label(RichText::new(self.language.label_default_icon()).weak());
            }
            if ui
                .button(tr(language, "Choose PNG icon", "Elegir icono PNG"))
                .clicked()
            {
                if let Some(file) = rfd::FileDialog::new()
                    .add_filter("PNG", &["png"])
                    .pick_file()
                {
                    self.custom_icon_path = Some(file);
                }
            }
        });
    }

    fn update_mod_filter(&mut self) {
        let search = self.mod_search.to_lowercase();
        let key = (search.clone(), self.data_revision);
        if self.mod_filter.key.as_ref() != Some(&key) {
            self.mod_filter.indices = self
                .mods
                .iter()
                .enumerate()
                .filter(|(_, m)| {
                    search.is_empty()
                        || m.name.to_lowercase().contains(&search)
                        || m.display_name.to_lowercase().contains(&search)
                        || m.author_name.to_lowercase().contains(&search)
                })
                .map(|(i, _)| i)
                .collect();
            self.mod_filter.key = Some(key);
        }
    }

    fn search(ui: &mut Ui, query: &mut String, hint: &str) {
        ui.horizontal(|ui| {
            components::icon(ui, Icon::Search, 18.0, Palette::for_ui(ui).muted);
            let spanish = ui.ctx().data(|data| {
                data.get_temp::<bool>(egui::Id::unique("spanish_ui"))
                    .unwrap_or(false)
            });
            let label = if spanish { "Limpiar" } else { "Clear" };
            let reserve = ui
                .painter()
                .layout_no_wrap(
                    label.into(),
                    egui::FontId::proportional(14.0),
                    ui.visuals().text_color(),
                )
                .size()
                .x
                + ui.spacing().button_padding.x * 2.0
                + ui.spacing().item_spacing.x;
            let width = (ui.available_width() - reserve).max(80.0);
            ui.add_sized(
                [width, 32.0],
                egui::TextEdit::singleline(query).hint_text(hint),
            );
            if ui.button(label).clicked() {
                query.clear();
            }
        });
    }

    fn ui_mods_selection(&mut self, ui: &mut Ui) {
        let language = self.language;
        self.page_heading(
            ui,
            self.language.heading_mods(),
            tr(
                language,
                "Select dependencies and adjust versions before packaging.",
                "Selecciona dependencias y ajusta las versiones del paquete.",
            ),
        );
        let compact = ui.available_width() < components::TABLE_BREAKPOINT;
        ui.horizontal_wrapped(|ui| {
            if compact {
                ui.menu_button(tr(language, "Selection", "Selección"), |ui| {
                    self.mod_bulk_actions(ui)
                });
            } else {
                self.mod_bulk_actions(ui);
            }
            let next = if compact {
                tr(language, "Next: configurations", "Siguiente: configuración")
            } else {
                self.language.btn_proceed_to_configs()
            };
            if primary(ui, next).clicked() {
                self.active_tab = ActiveTab::Configs;
            }
        });
        Self::search(ui, &mut self.mod_search, self.language.hint_search_mods());
        ui.label(
            RichText::new(self.language.label_mods_selected(
                self.mods.iter().filter(|m| m.selected).count(),
                self.mods.len(),
            ))
            .small()
            .weak(),
        );
        self.update_mod_filter();
        let indices = &self.mod_filter.indices;
        if indices.is_empty() {
            ui.label(tr(
                language,
                "No mods match this search.",
                "No hay mods que coincidan.",
            ));
            return;
        }
        if ui.available_width() < components::TABLE_BREAKPOINT {
            egui::ScrollArea::vertical()
                .id_salt("compact_mods")
                .auto_shrink([false, false])
                .show_rows(ui, 76.0, indices.len(), |ui, rows| {
                    for row in rows {
                        let index = indices[row];
                        let item = &mut self.mods[index];
                        ui.push_id(index, |ui| {
                            ui.horizontal(|ui| {
                                ui.checkbox(&mut item.selected, "");
                                let width = (ui.available_width() - 128.0).max(50.0);
                                components::left_label(
                                    ui,
                                    egui::vec2(width, 32.0),
                                    RichText::new(&item.display_name).strong(),
                                )
                                .on_hover_text(&item.display_name);
                                ui.add_sized(
                                    [100.0, 32.0],
                                    egui::TextEdit::singleline(&mut item.current_version),
                                )
                                .on_hover_text(tr(
                                    language,
                                    "Dependency version",
                                    "Versión de dependencia",
                                ));
                            });
                            ui.horizontal(|ui| {
                                components::left_label(
                                    ui,
                                    egui::vec2((ui.available_width() - 100.0).max(40.0), 20.0),
                                    RichText::new(&item.author_name).small().weak(),
                                );
                                ui.menu_button(tr(language, "Details", "Detalles"), |ui| {
                                    ui.set_max_width(350.0);
                                    ui.label(&item.name);
                                    ui.label(&item.description);
                                    ui.label(format!(
                                        "{}: {}",
                                        tr(language, "Original", "Original"),
                                        item.original_version
                                    ));
                                });
                            });
                            ui.separator();
                        });
                    }
                });
        } else {
            egui_extras::TableBuilder::new(ui)
                .id_salt("mods_table")
                .striped(true)
                .column(egui_extras::Column::exact(36.0))
                .column(egui_extras::Column::remainder().at_least(160.0))
                .column(egui_extras::Column::initial(140.0))
                .column(egui_extras::Column::exact(110.0))
                .column(egui_extras::Column::initial(210.0))
                .header(30.0, |mut header| {
                    for title in [
                        "",
                        tr(language, "Mod", "Mod"),
                        tr(language, "Author", "Autor"),
                        tr(language, "Version", "Versión"),
                        "Thunderstore ID",
                    ] {
                        header.col(|ui| {
                            ui.label(RichText::new(title).strong());
                        });
                    }
                })
                .body(|body| {
                    body.rows(36.0, indices.len(), |mut row| {
                        let item = &mut self.mods[indices[row.index()]];
                        row.col(|ui| {
                            ui.checkbox(&mut item.selected, "");
                        });
                        row.col(|ui| {
                            ui.add(
                                egui::Label::new(RichText::new(&item.display_name).strong())
                                    .truncate(),
                            )
                            .on_hover_text(&item.description);
                        });
                        row.col(|ui| {
                            ui.add(egui::Label::new(&item.author_name).truncate());
                        });
                        row.col(|ui| {
                            ui.add(
                                egui::TextEdit::singleline(&mut item.current_version)
                                    .desired_width(96.0),
                            );
                        });
                        row.col(|ui| {
                            ui.add(
                                egui::Label::new(RichText::new(&item.name).monospace()).truncate(),
                            )
                            .on_hover_text(&item.name);
                        });
                    });
                });
        }
    }

    fn mod_bulk_actions(&mut self, ui: &mut Ui) {
        if ui.button(self.language.btn_select_all()).clicked() {
            for item in &mut self.mods {
                item.selected = true;
            }
        }
        if ui.button(self.language.btn_select_enabled()).clicked() {
            for item in &mut self.mods {
                item.selected = item.enabled;
            }
        }
        if ui.button(self.language.btn_deselect_all()).clicked() {
            for item in &mut self.mods {
                item.selected = false;
            }
        }
    }

    fn category(category: ConfigCategory, language: Language) -> &'static str {
        match category {
            ConfigCategory::Safe => tr(language, "Safe", "Normal"),
            ConfigCategory::Sensitive => tr(language, "Sensitive", "Sensible"),
            ConfigCategory::Cache => tr(language, "Cache / report", "Caché / informe"),
        }
    }

    fn category_color(category: ConfigCategory, p: Palette) -> Color32 {
        match category {
            ConfigCategory::Safe => p.success,
            ConfigCategory::Sensitive => p.danger,
            ConfigCategory::Cache => p.warning,
        }
    }

    fn ui_configs_selection(&mut self, ui: &mut Ui) {
        let language = self.language;
        let p = Palette::for_ui(ui);
        self.page_heading(ui,self.language.heading_configs(),tr(language,"Review configuration files before sharing. Sensitive files are excluded by default.","Revisa la configuración antes de compartir. Los archivos sensibles se excluyen por defecto."));
        let compact = ui.available_width() < components::TABLE_BREAKPOINT;
        ui.horizontal_wrapped(|ui| {
            if compact {
                ui.menu_button(tr(language, "Selection", "Selección"), |ui| {
                    self.config_bulk_actions(ui)
                });
            } else {
                self.config_bulk_actions(ui);
            }
            let next = if compact {
                tr(
                    language,
                    "Next: build & export",
                    "Siguiente: crear y exportar",
                )
            } else {
                self.language.btn_proceed_to_build()
            };
            if primary(ui, next).clicked() {
                self.active_tab = ActiveTab::Build;
            }
        });
        Self::search(
            ui,
            &mut self.config_search,
            self.language.hint_search_configs(),
        );
        ui.label(
            RichText::new(self.language.label_configs_selected(
                self.configs.iter().filter(|c| c.selected).count(),
                self.configs.len(),
            ))
            .small()
            .weak(),
        );
        let search = self.config_search.to_lowercase();
        let key = (search.clone(), self.data_revision);
        if self.config_filter.key.as_ref() != Some(&key) {
            self.config_filter.indices = self
                .configs
                .iter()
                .enumerate()
                .filter(|(_, c)| search.is_empty() || c.name.to_lowercase().contains(&search))
                .map(|(i, _)| i)
                .collect();
            self.config_filter.key = Some(key);
        }
        let indices = &self.config_filter.indices;
        if indices.is_empty() {
            ui.label(tr(
                language,
                "No configuration files match.",
                "No hay archivos que coincidan.",
            ));
            return;
        }
        if ui.available_width() < components::TABLE_BREAKPOINT {
            egui::ScrollArea::vertical()
                .id_salt("compact_configs")
                .auto_shrink([false, false])
                .show_rows(ui, 76.0, indices.len(), |ui, rows| {
                    for row in rows {
                        let index = indices[row];
                        let c = &mut self.configs[index];
                        ui.push_id(index, |ui| {
                            ui.horizontal(|ui| {
                                ui.checkbox(&mut c.selected, "");
                                ui.add(
                                    egui::Label::new(RichText::new(&c.name).strong()).truncate(),
                                )
                                .on_hover_text(&c.name);
                            });
                            ui.horizontal(|ui| {
                                badge(
                                    ui,
                                    Self::category(c.category, language),
                                    Self::category_color(c.category, p),
                                );
                                ui.menu_button(tr(language, "Details", "Detalles"), |ui| {
                                    ui.label(if c.is_dir {
                                        tr(language, "Directory", "Carpeta").to_owned()
                                    } else {
                                        format!("{:.1} KB", c.size as f64 / 1024.0)
                                    });
                                    path(
                                        ui,
                                        tr(language, "Source", "Origen"),
                                        &c.path.display().to_string(),
                                    );
                                });
                            });
                            ui.separator();
                        });
                    }
                });
        } else {
            egui_extras::TableBuilder::new(ui)
                .id_salt("configs_table")
                .striped(true)
                .column(egui_extras::Column::exact(36.0))
                .column(egui_extras::Column::initial(150.0))
                .column(egui_extras::Column::remainder().at_least(200.0))
                .column(egui_extras::Column::exact(100.0))
                .column(egui_extras::Column::exact(90.0))
                .header(30.0, |mut header| {
                    for title in [
                        "",
                        tr(language, "Category", "Categoría"),
                        tr(language, "Name", "Nombre"),
                        tr(language, "Type", "Tipo"),
                        tr(language, "Size", "Tamaño"),
                    ] {
                        header.col(|ui| {
                            ui.label(RichText::new(title).strong());
                        });
                    }
                })
                .body(|body| {
                    body.rows(36.0, indices.len(), |mut row| {
                        let c = &mut self.configs[indices[row.index()]];
                        row.col(|ui| {
                            ui.checkbox(&mut c.selected, "");
                        });
                        row.col(|ui| {
                            ui.colored_label(
                                Self::category_color(c.category, p),
                                Self::category(c.category, language),
                            );
                        });
                        row.col(|ui| {
                            ui.add(egui::Label::new(RichText::new(&c.name).monospace()).truncate())
                                .on_hover_text(c.path.display().to_string());
                        });
                        row.col(|ui| {
                            ui.label(if c.is_dir {
                                tr(language, "Directory", "Carpeta")
                            } else {
                                tr(language, "File", "Archivo")
                            });
                        });
                        row.col(|ui| {
                            ui.label(if c.is_dir {
                                "—".into()
                            } else {
                                format!("{:.1} KB", c.size as f64 / 1024.0)
                            });
                        });
                    });
                });
        }
    }

    fn config_bulk_actions(&mut self, ui: &mut Ui) {
        let language = self.language;
        if ui
            .button(tr(language, "Safe only", "Solo normales"))
            .clicked()
        {
            for c in &mut self.configs {
                c.selected = c.category == ConfigCategory::Safe;
            }
        }
        if ui
            .button(tr(
                language,
                "Include all (may include secrets)",
                "Incluir todos (puede incluir secretos)",
            ))
            .clicked()
        {
            for c in &mut self.configs {
                c.selected = true;
            }
        }
        if ui.button(self.language.btn_deselect_all()).clicked() {
            for c in &mut self.configs {
                c.selected = false;
            }
        }
    }

    fn open_output(&mut self, ctx: &egui::Context) {
        if self.open_receiver.is_some() {
            return;
        }
        let Some(folder) = self
            .last_build_result
            .as_ref()
            .and_then(|r| r.zip_path.parent())
            .map(|p| p.to_path_buf())
        else {
            return;
        };
        let (tx, rx) = mpsc::channel();
        self.open_receiver = Some(rx);
        let repaint = ctx.clone();
        std::thread::spawn(move || {
            let result = opener::open(folder).map_err(|e| e.to_string());
            let _ = tx.send(result);
            repaint.request_repaint();
        });
    }

    fn ui_build_export(&mut self, ui: &mut Ui) {
        let language = self.language;
        let p = Palette::for_ui(ui);
        self.page_heading(
            ui,
            self.language.heading_build(),
            tr(
                language,
                "Create a Thunderstore-ready archive from your selected mods and configurations.",
                "Crea un archivo para Thunderstore con los mods y configuraciones seleccionados.",
            ),
        );
        egui::ScrollArea::vertical().id_salt("build_scroll").show(ui,|ui|{
            card(ui,tr(language,"Package overview","Resumen del paquete"),|ui|{
                ui.label(RichText::new(&self.pack_name).size(20.0).strong());ui.label(format!("v{} · {}",self.pack_version,self.selected_profile));
                self.increment_preference(ui);
                ui.horizontal_wrapped(|ui|{badge(ui,&format!("{} mods",self.mods.iter().filter(|m|m.selected).count()),p.info);badge(ui,&format!("{} configs",self.configs.iter().filter(|c|c.selected).count()),p.accent);});
                path(ui,self.language.heading_output_dir(),&self.output_dir.display().to_string());
                if self.configs.iter().any(|c|c.selected&&c.category==ConfigCategory::Sensitive){ui.colored_label(p.warning,tr(language,"Sensitive files selected. Review the generated config folder before sharing.","Hay archivos sensibles seleccionados. Revisa la carpeta config antes de compartir."));}
                if let Some((fraction,message))=&self.build_progress{ui.add(egui::ProgressBar::new(*fraction).text(message).show_percentage());}
                if ui.add_enabled(!self.building,egui::Button::new(RichText::new(self.language.btn_build_main()).color(p.on_accent).strong()).fill(p.accent)).clicked(){self.trigger_build(ui.ctx());}
            });
            if let Some(result)=&self.last_build_result{
                let archive=result.zip_path.display().to_string();let unpacked=result.build_dir.display().to_string();let size=result.zip_size_bytes;
                card(ui,tr(language,"Build complete","Paquete creado"),|ui|{
                    ui.horizontal(|ui|{components::icon(ui,Icon::Check,22.0,p.success);ui.colored_label(p.success,format!("{:.1} KB",size as f64/1024.0));});
                    path(ui,self.language.label_zip_archive(),&archive);path(ui,self.language.label_unpacked_folder(),&unpacked);
                    if ui.button(self.language.btn_open_folder()).clicked(){self.open_output(ui.ctx());}
                });
            }
        });
    }

    fn ui_server_sync(&mut self, ui: &mut Ui) {
        components::centered_page(ui, |ui| {
            self.page_heading(ui, self.language.sync_title(), self.language.sync_desc());
        });
        egui::ScrollArea::vertical()
            .id_salt("sync_scroll")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                components::centered_page(ui, |ui| {
                    if let Some(warning) = self.sync.warning.clone() {
                        ui.colored_label(Palette::for_ui(ui).warning, warning);
                        if ui.small_button(self.language.btn_dismiss()).clicked() {
                            self.sync.warning = None;
                        }
                    }
                    self.sync_actions(ui);
                    self.sync_ignore(ui);
                    ui.add_space(8.0);
                    if ui.available_width() >= components::SYNC_BREAKPOINT {
                        ui.columns(2, |columns| {
                            self.sync_source(&mut columns[0]);
                            self.sync_connection(&mut columns[0]);
                            self.sync_preview(&mut columns[1]);
                            self.sync_summary(&mut columns[1]);
                            self.sync_logs(&mut columns[1]);
                        });
                    } else {
                        self.sync_source(ui);
                        self.sync_connection(ui);
                        self.sync_preview(ui);
                        self.sync_summary(ui);
                        self.sync_logs(ui);
                    }
                });
            });
    }

    fn sync_source(&mut self, ui: &mut Ui) {
        let source = self.sync_source_bepinex();
        let p = Palette::for_ui(ui);
        card(ui, self.language.heading_source_profile(), |ui| {
            if let Some(source) = source {
                path(ui, "BepInEx", &source.display().to_string());
                let found = source.is_dir();
                ui.colored_label(
                    if found { p.success } else { p.danger },
                    if found {
                        self.language.source_folder_found()
                    } else {
                        self.language.source_folder_missing()
                    },
                );
            } else {
                ui.colored_label(p.warning, self.language.no_profile_selected());
            }
        });
    }

    fn sync_connection(&mut self, ui: &mut Ui) {
        let language = self.language;
        card(ui, self.language.heading_connection(), |ui| {
            field(ui, self.language.label_host(), &mut self.sync.config.host);
            ui.label(RichText::new(self.language.label_port()).small().strong());
            ui.add_sized(
                [ui.available_width(), 32.0],
                egui::DragValue::new(&mut self.sync.config.port).range(1..=65535),
            );
            field(
                ui,
                self.language.label_username(),
                &mut self.sync.config.username,
            );
            field(
                ui,
                tr(
                    language,
                    "Trusted server key (SHA-256)",
                    "Clave de servidor confiable (SHA-256)",
                ),
                &mut self.sync.config.host_key_sha256,
            );
            ui.label(RichText::new(tr(language,"Verify the fingerprint in the first connection log, then paste its 64 hex characters.","Verifica la huella del registro de conexión y pega sus 64 caracteres hexadecimales.")).small().weak());
            ui.label(
                RichText::new(self.language.label_password())
                    .small()
                    .strong(),
            );
            ui.add(
                egui::TextEdit::singleline(&mut self.sync.config.password)
                    .password(!self.sync.show_password)
                    .desired_width(f32::INFINITY),
            );
            ui.checkbox(
                &mut self.sync.show_password,
                self.language.label_show_password(),
            );
            field(
                ui,
                self.language.label_staging_dir(),
                &mut self.sync.config.staging_dir,
            );
            if ui.button(self.language.btn_browse()).clicked() {
                if let Some(folder) = rfd::FileDialog::new().pick_folder() {
                    self.sync.config.staging_dir = folder.display().to_string();
                    self.sync.reload_ignore();
                }
            }
            ui.checkbox(
                &mut self.sync.config.auto_refresh_staging,
                self.language.chk_auto_refresh(),
            );
            ui.label(
                RichText::new(self.language.desc_auto_refresh())
                    .small()
                    .weak(),
            );
            self.launcher_compatibility(ui);
            if ui.button(self.language.btn_save_settings()).clicked() {
                self.persist_sync();
            }
        });
    }

    fn launcher_compatibility(&mut self, ui: &mut Ui) {
        let language = self.language;
        ui.separator();
        ui.label(
            RichText::new(tr(
                language,
                "Hosting compatibility",
                "Compatibilidad del alojamiento",
            ))
            .small()
            .strong(),
        );
        let previous = self.sync.config.manage_bepinex_launcher;
        let changed = ui
            .add_enabled(
                !self.sync.running,
                egui::Checkbox::new(
                    &mut self.sync.config.manage_bepinex_launcher,
                    tr(
                        language,
                        "Enable custom BepInEx launcher checks and repair",
                        "Activar la revisión y reparación del iniciador BepInEx personalizado",
                    ),
                ),
            )
            .changed();
        ui.label(RichText::new(tr(language,
            "Leave off when your provider supplies BepInEx or restricts executable changes. Enable only when your host lets you replace valheim_server.x86_64 and manage its data symlink. Config and plugin sync remains available in either mode.",
            "Déjalo desactivado si el proveedor incluye BepInEx o restringe los cambios del ejecutable. Actívalo solo si permite reemplazar valheim_server.x86_64 y gestionar su enlace de datos. La sincronización de config y plugins sigue disponible en ambos modos.",
        )).small().weak());
        if changed {
            match self.sync.config.save() {
                Ok(()) => {
                    self.sync.preview = None;
                    self.sync.preview_revision = self.sync.preview_revision.wrapping_add(1);
                    self.status_message = Some((
                        tr(
                            language,
                            "Hosting compatibility preference saved.",
                            "Preferencia de compatibilidad guardada.",
                        )
                        .into(),
                        false,
                    ));
                }
                Err(error) => {
                    self.sync.config.manage_bepinex_launcher = previous;
                    self.sync_error(error);
                }
            }
        }
    }

    fn persist_sync(&mut self) {
        match self.sync.persist() {
            Ok(()) => {
                self.status_message = Some((
                    tr(
                        self.language,
                        "Sync settings saved.",
                        "Configuración guardada.",
                    )
                    .into(),
                    false,
                ))
            }
            Err(error) => self.sync_error(error),
        }
    }

    fn sync_actions(&mut self, ui: &mut Ui) {
        let language = self.language;
        card(ui, self.language.heading_actions(), |ui| {
            ui.add_enabled_ui(!self.sync.running, |ui| {
                ui.horizontal_wrapped(|ui| {
                    if ui.button(self.language.btn_action_preview()).clicked() {
                        self.start_sync(SyncMode::Preview, ui.ctx());
                    }
                    if primary(ui, self.language.btn_action_sync()).clicked() {
                        self.start_sync(SyncMode::SyncChanges, ui.ctx());
                    }
                    if ui.button(self.language.btn_action_upload_only()).clicked() {
                        self.start_sync(SyncMode::UploadOnly, ui.ctx());
                    }
                    for (mode, label) in [
                        (SyncMode::ForceAll, self.language.btn_action_force()),
                        (SyncMode::CheckWrapper, self.language.btn_action_wrapper()),
                        (SyncMode::RefreshStaging, self.language.btn_action_refresh()),
                        (SyncMode::TestConnection, self.language.btn_action_test()),
                    ] {
                        let enabled = mode != SyncMode::CheckWrapper || self.sync.config.manage_bepinex_launcher;
                        let response = ui.add_enabled(enabled, egui::Button::new(label));
                        let response = if enabled { response } else { response.on_disabled_hover_text(tr(language,
                                "Enable custom BepInEx launcher checks in Hosting compatibility to use this action.",
                                "Activa la revisión del iniciador BepInEx en Compatibilidad del alojamiento para usar esta acción.")) };
                        if response.clicked() {
                            self.start_sync(mode, ui.ctx());
                        }
                    }
                });
            });
            if self.sync.running {
                ui.horizontal(|ui| {
                    ui.spinner();
                    ui.label(tr(language, "Sync running…", "Sincronizando…"));
                    if matches!(
                        self.sync.active_mode,
                        Some(SyncMode::SyncChanges | SyncMode::ForceAll | SyncMode::UploadOnly)
                    ) && ui.button(tr(language, "Cancel", "Cancelar")).clicked()
                    {
                        if let Some(cancel) = &self.sync.cancel {
                            cancel.store(true, Ordering::Relaxed);
                        }
                    }
                });
            }
            if let Some(progress) = &self.sync.progress {
                ui.add(
                    egui::ProgressBar::new(progress.current as f32 / progress.total.max(1) as f32)
                        .text(&progress.label),
                );
            }
        });
    }

    fn sync_ignore(&mut self, ui: &mut Ui) {
        egui::CollapsingHeader::new(self.language.heading_ignore())
            .id_salt("advanced_ignore")
            .default_open(ui.available_width() >= components::SYNC_BREAKPOINT)
            .show(ui, |ui| {
                ui.label(self.language.desc_ignore());
                ui.add(
                    egui::TextEdit::multiline(&mut self.sync.ignore_text)
                        .desired_rows(6)
                        .desired_width(f32::INFINITY)
                        .code_editor(),
                );
                ui.horizontal_wrapped(|ui| {
                    if ui.button(self.language.btn_reset_defaults()).clicked() {
                        self.sync.ignore_text =
                            format!("{}\n", DEFAULT_IGNORED_PATTERNS.join("\n"));
                    }
                    if ui.button(self.language.btn_reload_staging()).clicked() {
                        self.sync.reload_ignore();
                    }
                    if ui.button(self.language.btn_save_ignore()).clicked() {
                        self.persist_sync();
                    }
                });
            });
    }

    fn sync_preview(&mut self, ui: &mut Ui) {
        let language = self.language;
        let p = Palette::for_ui(ui);
        card(ui, self.language.heading_preview(), |ui| {
            let Some(preview) = self.sync.preview.as_ref() else {
                ui.horizontal(|ui| {
                    components::icon(ui, Icon::Search, 26.0, p.muted);
                    ui.add(egui::Label::new(self.language.placeholder_no_preview()).wrap());
                });
                return;
            };
            ui.horizontal_wrapped(|ui| {
                badge(
                    ui,
                    &format!(
                        "{} {}",
                        preview.uploads.len(),
                        tr(language, "uploads", "subidas")
                    ),
                    p.success,
                );
                badge(
                    ui,
                    &format!(
                        "{} {}",
                        preview.deletions.len(),
                        tr(language, "deletions", "eliminaciones")
                    ),
                    p.danger,
                );
                badge(
                    ui,
                    &format!(
                        "{} / {} {}",
                        preview.local_count,
                        preview.remote_count,
                        tr(language, "local / remote", "locales / remotos")
                    ),
                    p.info,
                );
            });
            if preview.wrapper.status == WrapperStatus::Skipped {
                ui.colored_label(
                    p.muted,
                    tr(
                        language,
                        "Custom launcher checks disabled · provider-managed BepInEx",
                        "Revisión del iniciador desactivada · BepInEx gestionado por el proveedor",
                    ),
                );
            } else {
                let (status, color) = match preview.wrapper.status {
                    WrapperStatus::Intact => (self.language.wrapper_intact(), p.success),
                    WrapperStatus::ElfNeedsRename => {
                        (self.language.wrapper_rename_needed(), p.warning)
                    }
                    WrapperStatus::Missing => (self.language.wrapper_missing(), p.warning),
                    _ => (
                        tr(
                            language,
                            "Wrapper status unknown",
                            "Estado del iniciador desconocido",
                        ),
                        p.warning,
                    ),
                };
                ui.colored_label(color, status);
                ui.label(
                    RichText::new(if preview.wrapper.symlink_present {
                        self.language.symlink_ok()
                    } else {
                        self.language.symlink_missing()
                    })
                    .small()
                    .weak(),
                );
                for warning in &preview.wrapper.warnings {
                    ui.colored_label(p.warning, warning);
                }
            }
            ui.horizontal_wrapped(|ui| {
                for (tab, label) in [
                    (PreviewTab::All, tr(language, "All", "Todos")),
                    (PreviewTab::Uploads, tr(language, "Uploads", "Subidas")),
                    (
                        PreviewTab::Deletions,
                        tr(language, "Deletions", "Eliminaciones"),
                    ),
                ] {
                    ui.selectable_value(&mut self.sync.preview_tab, tab, label);
                }
            });
            Self::search(
                ui,
                &mut self.sync.preview_search,
                self.language.hint_search_changes(),
            );
            let query = self.sync.preview_search.to_lowercase();
            let key = (
                format!("{:?}:{query}", self.sync.preview_tab),
                self.sync.preview_revision,
            );
            if self.preview_filter.key.as_ref() != Some(&key) {
                let uploads = preview.uploads.len();
                self.preview_filter.indices = preview
                    .uploads
                    .iter()
                    .chain(preview.deletions.iter())
                    .enumerate()
                    .filter(|(i, path)| {
                        (self.sync.preview_tab == PreviewTab::All
                            || (self.sync.preview_tab == PreviewTab::Uploads) == (*i < uploads))
                            && (query.is_empty() || path.to_lowercase().contains(&query))
                    })
                    .map(|(i, _)| i)
                    .collect();
                self.preview_filter.key = Some(key);
            }
            if self.preview_filter.indices.is_empty() {
                ui.label(RichText::new(self.language.msg_no_changes_filter()).weak());
            }
            egui::ScrollArea::vertical()
                .id_salt("preview_rows")
                .max_height(220.0)
                .auto_shrink([false, true])
                .show_rows(ui, 30.0, self.preview_filter.indices.len(), |ui, rows| {
                    for row in rows {
                        let index = self.preview_filter.indices[row];
                        let upload = index < preview.uploads.len();
                        let value = if upload {
                            &preview.uploads[index]
                        } else {
                            &preview.deletions[index - preview.uploads.len()]
                        };
                        ui.horizontal(|ui| {
                            ui.colored_label(
                                if upload { p.success } else { p.danger },
                                if upload { "+" } else { "−" },
                            );
                            ui.add(egui::Label::new(RichText::new(value).monospace()).truncate())
                                .on_hover_text(value);
                        });
                    }
                });
        });
    }

    fn sync_summary(&mut self, ui: &mut Ui) {
        let Some(summary) = &self.sync.summary else {
            return;
        };
        let language = self.language;
        let p = Palette::for_ui(ui);
        card(
            ui,
            tr(language, "Execution summary", "Resultado de ejecución"),
            |ui| {
                ui.horizontal_wrapped(|ui| {
                    badge(
                        ui,
                        &format!(
                            "{} {}",
                            summary.uploaded,
                            tr(language, "uploaded", "subidos")
                        ),
                        p.success,
                    );
                    badge(
                        ui,
                        &format!(
                            "{} {}",
                            summary.deleted,
                            tr(language, "deleted", "eliminados")
                        ),
                        p.info,
                    );
                    badge(
                        ui,
                        &format!(
                            "{} {}",
                            summary.failed_uploads + summary.failed_deletes,
                            tr(language, "failed", "fallidos")
                        ),
                        if summary.failed_uploads + summary.failed_deletes > 0 {
                            p.danger
                        } else {
                            p.muted
                        },
                    );
                });
                ui.label(format!(
                    "{:.1} s · {} {} · {} {}",
                    summary.elapsed_secs,
                    summary.staged_copied,
                    tr(language, "staged", "copiados"),
                    summary.staged_pruned,
                    tr(language, "pruned", "depurados")
                ));
                if summary.cancelled {
                    ui.colored_label(
                        p.warning,
                        tr(
                            language,
                            "Operation cancelled; sync state was not updated.",
                            "Operación cancelada; no se actualizó el estado.",
                        ),
                    );
                }
            },
        );
    }

    fn sync_logs(&mut self, ui: &mut Ui) {
        let language = self.language;
        let p = Palette::for_ui(ui);
        card(ui, self.language.heading_logs(), |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.checkbox(
                    &mut self.sync.log_autoscroll,
                    tr(language, "Auto-scroll", "Desplazamiento automático"),
                );
                if ui.button(tr(language, "Copy", "Copiar")).clicked() {
                    ui.ctx().copy_text(
                        self.sync
                            .logs
                            .iter()
                            .map(|l| l.message.as_str())
                            .collect::<Vec<_>>()
                            .join("\n"),
                    );
                }
                if ui.button(tr(language, "Clear", "Limpiar")).clicked() {
                    self.sync.logs.clear();
                    self.sync.logs_revision = self.sync.logs_revision.wrapping_add(1);
                }
            });
            ui.horizontal_wrapped(|ui| {
                for (level, label) in [
                    (None, tr(language, "All", "Todos")),
                    (
                        Some(LogLevel::Success),
                        tr(language, "Success", "Correctos"),
                    ),
                    (Some(LogLevel::Info), "Info"),
                    (Some(LogLevel::Warning), tr(language, "Warnings", "Avisos")),
                    (Some(LogLevel::Error), tr(language, "Errors", "Errores")),
                ] {
                    ui.selectable_value(&mut self.sync.log_filter, level, label);
                }
            });
            Self::search(
                ui,
                &mut self.sync.log_search,
                self.language.hint_search_logs(),
            );
            let query = self.sync.log_search.to_lowercase();
            let key = (
                format!("{:?}:{query}", self.sync.log_filter),
                self.sync.logs_revision,
            );
            if self.logs_filter.key.as_ref() != Some(&key) {
                self.logs_filter.indices = self
                    .sync
                    .logs
                    .iter()
                    .enumerate()
                    .filter(|(_, l)| {
                        self.sync.log_filter.is_none_or(|level| level == l.level)
                            && (query.is_empty() || l.message.to_lowercase().contains(&query))
                    })
                    .map(|(i, _)| i)
                    .collect();
                self.logs_filter.key = Some(key);
            }
            if self.logs_filter.indices.is_empty() {
                ui.label(RichText::new(self.language.no_logs_matching()).weak());
            }
            egui::ScrollArea::vertical()
                .id_salt("log_rows")
                .max_height(260.0)
                .auto_shrink([false, true])
                .stick_to_bottom(self.sync.log_autoscroll)
                .show_viewport(ui, |ui, viewport| {
                    self.layout_log_messages(ui);
                    let total_height = self.log_layout.offsets.last().copied().unwrap_or(0.0);
                    let (content, _) = ui.allocate_exact_size(
                        egui::vec2(ui.available_width(), total_height),
                        egui::Sense::hover(),
                    );
                    let first = self
                        .log_layout
                        .offsets
                        .partition_point(|offset| *offset < viewport.top())
                        .saturating_sub(1);
                    let end = self
                        .log_layout
                        .offsets
                        .partition_point(|offset| *offset <= viewport.bottom())
                        .min(self.log_layout.galleys.len());
                    for row in first..end {
                        let log = &self.sync.logs[self.logs_filter.indices[row]];
                        let (tag, color) = match log.level {
                            LogLevel::Info => ("INFO", p.info),
                            LogLevel::Success => ("OK", p.success),
                            LogLevel::Warning => ("WARN", p.warning),
                            LogLevel::Error => ("ERROR", p.danger),
                        };
                        let top = content.top() + self.log_layout.offsets[row];
                        ui.put(
                            egui::Rect::from_min_size(
                                egui::pos2(content.left(), top + 4.0),
                                egui::vec2(52.0, 20.0),
                            ),
                            egui::Label::new(RichText::new(tag).small().color(color).strong()),
                        );
                        ui.painter().galley_with_override_text_color(
                            egui::pos2(content.left() + 60.0, top + 4.0),
                            self.log_layout.galleys[row].clone(),
                            p.text,
                        );
                    }
                });
        });
    }

    fn layout_log_messages(&mut self, ui: &Ui) {
        let width = (ui.available_width() - 60.0).max(40.0);
        let font = egui::TextStyle::Monospace.resolve(ui.style());
        let key = (
            format!("{:?}:{}", self.sync.log_filter, self.sync.log_search),
            self.sync.logs_revision,
            width.to_bits(),
            ui.ctx().pixels_per_point().to_bits(),
            font.clone(),
        );
        if self.log_layout.key.as_ref() == Some(&key)
            && self.log_layout.context.as_ref() == Some(ui.ctx())
        {
            return;
        }
        self.log_layout.galleys.clear();
        self.log_layout.offsets.clear();
        self.log_layout.offsets.push(0.0);
        for index in &self.logs_filter.indices {
            let galley = ui.painter().layout(
                self.sync.logs[*index].message.clone(),
                font.clone(),
                Color32::WHITE,
                width,
            );
            let height = galley.size().y.max(20.0) + 8.0;
            self.log_layout
                .offsets
                .push(self.log_layout.offsets.last().copied().unwrap() + height);
            self.log_layout.galleys.push(galley);
        }
        self.log_layout.key = Some(key);
        self.log_layout.context = Some(ui.ctx().clone());
    }

    fn ui_help(&mut self, ui: &mut Ui) {
        self.page_heading(
            ui,
            self.language.help_title(),
            tr(
                self.language,
                "From local profile to a shareable modpack or a synchronized server.",
                "Del perfil local a un modpack compartible o un servidor sincronizado.",
            ),
        );
        egui::ScrollArea::vertical().id_salt("help_scroll").show(ui,|ui|{
            for(title,points)in self.language.help_content(){card(ui,title,|ui|{for point in *points{ui.label(*point);}});}
            card(ui,tr(self.language,"Appearance and desktop glass","Apariencia y cristal del escritorio"),|ui|{
                ui.label(tr(self.language,"Choose Light, Dark, Glass Light, or Glass Dark from Appearance. Desktop blur follows compositor capabilities and system transparency settings; a readable glass backdrop is used when unavailable.","Elige Claro, Oscuro, Cristal claro o Cristal oscuro en Apariencia. El desenfoque depende del compositor y de la transparencia del sistema; se usa un fondo legible cuando no está disponible."));
            });
        });
    }
}
