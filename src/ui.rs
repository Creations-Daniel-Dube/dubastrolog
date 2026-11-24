use std::collections::HashMap;
use std::ops::RangeInclusive;

use eframe::egui::{self, Color32, Frame, RichText, Stroke};

use crate::attachments::{self, is_image, open_file};
use crate::model::{
    Attachment, Camera, CameraKind, Instrument, InstrumentKind, Journal, ObserveMode, ProfileKind,
    Session, Site, SmartTelescope, Target, TargetInstrumentKind, TargetKind, Telescope, display_name,
    is_iso_date, profile_usages,
};
use crate::query::{self, nights_in_year, photos_of};
use crate::store;

const SMART_PRESETS: &[&str] = &["SeeStar S30", "SeeStar S50", "SeeStar S30 Pro"];

#[derive(Clone, Copy, PartialEq, Eq)]
enum View {
    Journal,
    Search,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ProfileTab {
    Sites,
    Telescopes,
    Cameras,
    Dslr,
    Smart,
}

struct Catalog {
    sites: Vec<(String, String)>,
    telescopes: Vec<(String, String)>,
    cameras: Vec<(String, String)>,
    smarts: Vec<(String, String)>,
}

impl Catalog {
    fn from(journal: &Journal) -> Self {
        Self {
            sites: names(&journal.sites, |site| &site.id, |site| &site.name),
            telescopes: names(&journal.telescopes, |item| &item.id, |item| &item.name),
            cameras: names(&journal.cameras, |item| &item.id, |item| &item.name),
            smarts: names(&journal.smart_scopes, |item| &item.id, |item| &item.name),
        }
    }

    fn find<'a>(items: &'a [(String, String)], id: &Option<String>) -> Option<&'a str> {
        id.as_ref()
            .and_then(|id| items.iter().find(|(item, _)| item == id))
            .map(|(_, name)| name.as_str())
    }
}

fn names<T>(
    items: &[T],
    id: impl Fn(&T) -> &String,
    name: impl Fn(&T) -> &String,
) -> Vec<(String, String)> {
    items
        .iter()
        .map(|item| (id(item).clone(), name(item).clone()))
        .collect()
}

fn camera_names(cameras: &[Camera], kind: CameraKind) -> Vec<(String, String)> {
    cameras
        .iter()
        .filter(|camera| camera.kind == kind)
        .map(|camera| (camera.id.clone(), camera.name.clone()))
        .collect()
}

struct SessionRow {
    id: String,
    sort_date: String,
    date: String,
    site: String,
    targets: usize,
    first_target: Option<String>,
}

struct AttachmentRow {
    id: String,
    stored_name: String,
    original_name: String,
}

pub struct App {
    journal: Journal,
    saved_text: String,
    can_save: bool,
    view: View,
    selected_session: Option<String>,
    selected_target: Option<String>,
    session_filter: String,
    profiles_open: bool,
    profile_tab: ProfileTab,
    selected_site: Option<String>,
    selected_telescope: Option<String>,
    selected_camera: Option<String>,
    selected_smart: Option<String>,
    pending_delete: Option<String>,
    notice: Option<String>,
    thumbs: HashMap<String, Option<egui::TextureHandle>>,
    object_query: String,
    year_query: String,
}

impl App {
    fn new(cc: &eframe::CreationContext) -> Self {
        cc.egui_ctx.set_visuals(theme());
        install_font(&cc.egui_ctx);
        let (journal, notice) = store::load_journal();
        let saved_text = serde_json::to_string_pretty(&journal).unwrap_or_default();
        let selected = journal.sessions.iter().max_by(|left, right| {
            left.date
                .cmp(&right.date)
                .then(left.start_time.cmp(&right.start_time))
                .then(left.id.cmp(&right.id))
        });
        Self {
            can_save: notice.is_none(),
            selected_session: selected.map(|session| session.id.clone()),
            selected_target: selected
                .and_then(|session| session.targets.first().map(|target| target.id.clone())),
            selected_site: journal.sites.first().map(|site| site.id.clone()),
            selected_telescope: journal.telescopes.first().map(|item| item.id.clone()),
            selected_camera: journal.cameras.first().map(|item| item.id.clone()),
            selected_smart: journal.smart_scopes.first().map(|item| item.id.clone()),
            notice,
            journal,
            saved_text,
            view: View::Journal,
            session_filter: String::new(),
            profiles_open: false,
            profile_tab: ProfileTab::Sites,
            pending_delete: None,
            thumbs: HashMap::new(),
            object_query: String::new(),
            year_query: String::new(),
        }
    }

    fn persist(&mut self) {
        if !self.can_save {
            return;
        }
        let Ok(text) = serde_json::to_string_pretty(&self.journal) else {
            return;
        };
        if text == self.saved_text {
            return;
        }
        match store::save_journal(&self.journal) {
            Ok(()) => self.saved_text = text,
            Err(err) => {
                let message = format!("Enregistrement impossible : {err}");
                if self.notice.as_ref() != Some(&message) {
                    self.notice = Some(message);
                }
                self.saved_text = text;
            }
        }
    }

    fn session_rows(&self) -> Vec<SessionRow> {
        let filter = self.session_filter.trim().to_lowercase();
        let mut rows = Vec::new();
        for session in &self.journal.sessions {
            let site = session
                .site_id
                .as_ref()
                .and_then(|id| self.journal.sites.iter().find(|site| &site.id == id))
                .map(|site| display_name(&site.name).to_string())
                .unwrap_or_else(|| "Sans site".to_string());
            if !filter.is_empty() {
                let mut hay = format!(
                    "{} {} {} {}",
                    session.date, site, session.notes, session.start_time
                )
                .to_lowercase();
                for target in &session.targets {
                    hay.push(' ');
                    hay.push_str(&target.name.to_lowercase());
                }
                if !hay.contains(&filter) {
                    continue;
                }
            }
            rows.push(SessionRow {
                id: session.id.clone(),
                sort_date: session.date.clone(),
                date: if session.date.is_empty() {
                    "Sans date".to_string()
                } else {
                    session.date.clone()
                },
                site,
                targets: session.targets.len(),
                first_target: session.targets.first().map(|target| target.id.clone()),
            });
        }
        rows.sort_by(|left, right| {
            right
                .sort_date
                .cmp(&left.sort_date)
                .then(right.id.cmp(&left.id))
        });
        rows
    }

    fn sessions_panel(&mut self, ui: &mut egui::Ui) {
        ui.add_space(6.0);
        if ui
            .add_sized(
                [ui.available_width(), 28.0],
                egui::Button::new("Nouvelle soirée"),
            )
            .clicked()
        {
            self.add_session();
        }
        if ui
            .add_sized(
                [ui.available_width(), 28.0],
                egui::Button::new("Recherche").selected(self.view == View::Search),
            )
            .clicked()
        {
            self.view = View::Search;
        }
        if ui
            .add_sized([ui.available_width(), 28.0], egui::Button::new("Profils"))
            .clicked()
        {
            self.profiles_open = true;
        }
        ui.add_space(8.0);
        ui.add(
            egui::TextEdit::singleline(&mut self.session_filter)
                .hint_text("Filtrer les soirées")
                .desired_width(f32::INFINITY),
        );
        ui.add_space(8.0);
        let rows = self.session_rows();
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                if self.journal.sessions.is_empty() {
                    ui.label("Aucune soirée pour le moment.");
                    return;
                }
                if rows.is_empty() {
                    ui.label("Aucune soirée ne correspond.");
                    return;
                }
                for row in &rows {
                    let selected = self.view == View::Journal
                        && self.selected_session.as_ref() == Some(&row.id);
                    let text = format!(
                        "{}\n{} · {}",
                        row.date,
                        row.site,
                        objects_phrase(row.targets)
                    );
                    ui.push_id(&row.id, |ui| {
                        let response = ui.add_sized(
                            [ui.available_width(), 46.0],
                            egui::Button::new(text).selected(selected),
                        );
                        if response.clicked() {
                            self.view = View::Journal;
                            self.selected_session = Some(row.id.clone());
                            self.selected_target = row.first_target.clone();
                        }
                        let session_id = row.id.clone();
                        let first_target = row.first_target.clone();
                        response.context_menu(|ui| {
                            if ui.button("Supprimer cette soirée").clicked() {
                                self.view = View::Journal;
                                self.selected_session = Some(session_id.clone());
                                self.selected_target = first_target.clone();
                                self.pending_delete = Some(session_id);
                                ui.close();
                            }
                        });
                    });
                }
            });
    }

    fn add_session(&mut self) {
        let session = Session::create();
        self.selected_session = Some(session.id.clone());
        self.selected_target = None;
        self.view = View::Journal;
        self.journal.sessions.push(session);
    }

    fn session_page(&mut self, ui: &mut egui::Ui) {
        if self.selected_session.is_none() {
            ui.label("Aucune soirée sélectionnée. Créez une soirée pour noter la météo, les objets et les photos.");
            return;
        }
        let mut delete = false;
        group(ui, "Soirée", |ui| delete = self.evening(ui));
        group(ui, "Météo", |ui| self.weather(ui));
        group(ui, "Objets", |ui| self.objects(ui));
        if delete {
            self.pending_delete = self.selected_session.clone();
        }
    }

    fn evening(&mut self, ui: &mut egui::Ui) -> bool {
        let catalog = Catalog::from(&self.journal);
        let Some(session_id) = self.selected_session.clone() else {
            return false;
        };
        let Some(session) = self
            .journal
            .sessions
            .iter_mut()
            .find(|session| session.id == session_id)
        else {
            return false;
        };
        labeled(ui, "Date", &mut session.date, "2026-11-23");
        if !session.date.is_empty() && !is_iso_date(&session.date) {
            ui.label("Utilisez le format AAAA-MM-JJ pour que la recherche par année retrouve cette soirée.");
        }
        ui.horizontal(|ui| {
            labeled(ui, "Début", &mut session.start_time, "20:30");
            labeled(ui, "Fin", &mut session.end_time, "23:45");
        });
        combo_option(
            ui,
            "session-site",
            "Site",
            &mut session.site_id,
            &catalog.sites,
            "Aucun",
        );
        ui.horizontal(|ui| {
            ui.selectable_value(
                &mut session.instrument.kind,
                InstrumentKind::Telescope,
                "Télescope",
            );
            ui.selectable_value(
                &mut session.instrument.kind,
                InstrumentKind::Smart,
                "Télescope intelligent",
            );
        });
        match session.instrument.kind {
            InstrumentKind::Telescope => {
                combo_option(
                    ui,
                    "session-scope",
                    "Télescope",
                    &mut session.instrument.telescope_id,
                    &catalog.telescopes,
                    "Aucun",
                );
                combo_option(
                    ui,
                    "session-camera",
                    "Caméra",
                    &mut session.instrument.camera_id,
                    &catalog.cameras,
                    "Aucune",
                );
            }
            InstrumentKind::Smart => {
                combo_option(
                    ui,
                    "session-smart",
                    "Télescope intelligent",
                    &mut session.instrument.smart_id,
                    &catalog.smarts,
                    "Aucun",
                );
            }
        }
        ui.label("Notes");
        ui.add(
            egui::TextEdit::multiline(&mut session.notes)
                .desired_rows(3)
                .desired_width(f32::INFINITY),
        );
        ui.button("Supprimer cette soirée").clicked()
    }

    fn weather(&mut self, ui: &mut egui::Ui) {
        let Some(session_id) = self.selected_session.clone() else {
            return;
        };
        let Some(session) = self
            .journal
            .sessions
            .iter_mut()
            .find(|session| session.id == session_id)
        else {
            return;
        };
        let weather = &mut session.weather;
        egui::Grid::new("weather")
            .num_columns(2)
            .spacing([18.0, 8.0])
            .show(ui, |ui| {
                opt_f32(
                    ui,
                    "Température (°C)",
                    &mut weather.temperature_c,
                    0.0,
                    -80.0..=50.0,
                    0.1,
                );
                opt_f32(
                    ui,
                    "Humidité (%)",
                    &mut weather.humidity_pct,
                    50.0,
                    0.0..=100.0,
                    1.0,
                );
                ui.end_row();
                opt_f32(
                    ui,
                    "Pression (hPa)",
                    &mut weather.pressure_hpa,
                    1013.0,
                    500.0..=1100.0,
                    0.5,
                );
                opt_f32(
                    ui,
                    "Vent (km/h)",
                    &mut weather.wind_kmh,
                    0.0,
                    0.0..=300.0,
                    1.0,
                );
                ui.end_row();
                opt_u8(
                    ui,
                    "Couverture nuageuse (0 à 10)",
                    &mut weather.cloud_cover,
                    0,
                    0..=10,
                );
                opt_u8(ui, "Seeing (1 à 5)", &mut weather.seeing, 3, 1..=5);
                ui.end_row();
                opt_f32(
                    ui,
                    "Seeing (secondes d'arc)",
                    &mut weather.seeing_arcsec,
                    2.0,
                    0.0..=10.0,
                    0.1,
                );
                opt_u8(
                    ui,
                    "Transparence (1 à 5)",
                    &mut weather.transparency,
                    3,
                    1..=5,
                );
                ui.end_row();
                opt_f32(ui, "SQM", &mut weather.sqm, 21.0, 0.0..=30.0, 0.1);
                opt_f32(
                    ui,
                    "Lune (% d'éclairement)",
                    &mut weather.moon_illumination_pct,
                    0.0,
                    0.0..=100.0,
                    1.0,
                );
                ui.end_row();
            });
        labeled(ui, "Direction du vent", &mut weather.wind_dir, "NO");
        ui.label("Notes");
        ui.add(
            egui::TextEdit::multiline(&mut weather.notes)
                .desired_rows(2)
                .desired_width(f32::INFINITY),
        );
    }

    fn objects(&mut self, ui: &mut egui::Ui) {
        let Some(session_id) = self.selected_session.clone() else {
            return;
        };
        let names: Vec<(String, String)> = self
            .journal
            .sessions
            .iter()
            .find(|session| session.id == session_id)
            .map(|session| {
                session
                    .targets
                    .iter()
                    .map(|target| (target.id.clone(), target.name.clone()))
                    .collect()
            })
            .unwrap_or_default();
        if names.is_empty() {
            ui.label("Aucun objet pour cette soirée.");
        }
        let mut pick = None;
        ui.horizontal_wrapped(|ui| {
            for (id, name) in &names {
                if ui
                    .selectable_label(
                        self.selected_target.as_ref() == Some(id),
                        display_name(name),
                    )
                    .clicked()
                {
                    pick = Some(id.clone());
                }
            }
        });
        if let Some(id) = pick {
            self.selected_target = Some(id);
        }
        let mut add = false;
        let mut remove = false;
        ui.horizontal(|ui| {
            if ui.button("Ajouter un objet").clicked() {
                add = true;
            }
            if ui
                .add_enabled(
                    self.selected_target.is_some(),
                    egui::Button::new("Supprimer l'objet"),
                )
                .clicked()
            {
                remove = true;
            }
        });
        if add {
            let target = Target::create();
            self.selected_target = Some(target.id.clone());
            if let Some(session) = self
                .journal
                .sessions
                .iter_mut()
                .find(|session| session.id == session_id)
            {
                session.targets.push(target);
            }
        }
        if remove {
            if let Some(target_id) = self.selected_target.clone() {
                self.remove_target(&session_id, &target_id);
            }
        }
        self.target_form(ui, &session_id);
    }

    fn target_form(&mut self, ui: &mut egui::Ui, session_id: &str) {
        let Some(target_id) = self.selected_target.clone() else {
            return;
        };
        let exists = self.journal.sessions.iter().any(|session| {
            session.id == session_id && session.targets.iter().any(|target| target.id == target_id)
        });
        if !exists {
            return;
        }
        let catalog = Catalog::from(&self.journal);
        let inherited = self
            .journal
            .sessions
            .iter()
            .find(|session| session.id == session_id)
            .map(|session| describe_instrument(&session.instrument, &catalog))
            .unwrap_or_default();
        if let Some(target) = self.target_mut(session_id, &target_id) {
            labeled(ui, "Nom", &mut target.name, "M42");
            enum_combo(
                ui,
                "target-kind",
                "Type",
                &mut target.kind,
                &TargetKind::all().map(|kind| (kind, kind.label())),
            );
            enum_combo(
                ui,
                "target-mode",
                "Mode",
                &mut target.mode,
                &ObserveMode::all().map(|mode| (mode, mode.label())),
            );
            labeled(ui, "Heure", &mut target.time, "21:10");
            ui.horizontal_wrapped(|ui| {
                ui.selectable_value(
                    &mut target.instrument_kind,
                    TargetInstrumentKind::Session,
                    "Celui de la soirée",
                );
                ui.selectable_value(
                    &mut target.instrument_kind,
                    TargetInstrumentKind::Telescope,
                    "Autre télescope",
                );
                ui.selectable_value(
                    &mut target.instrument_kind,
                    TargetInstrumentKind::Smart,
                    "Télescope intelligent",
                );
            });
            match target.instrument_kind {
                TargetInstrumentKind::Session => {
                    ui.label(format!("Utilise l'instrument de la soirée — {inherited}"));
                }
                TargetInstrumentKind::Telescope => {
                    combo_option(
                        ui,
                        "target-scope",
                        "Télescope",
                        &mut target.telescope_id,
                        &catalog.telescopes,
                        "Aucun",
                    );
                    combo_option(
                        ui,
                        "target-camera",
                        "Caméra",
                        &mut target.camera_id,
                        &catalog.cameras,
                        "Aucune",
                    );
                }
                TargetInstrumentKind::Smart => {
                    combo_option(
                        ui,
                        "target-smart",
                        "Télescope intelligent",
                        &mut target.smart_id,
                        &catalog.smarts,
                        "Aucun",
                    );
                }
            }
            labeled(ui, "Filtre", &mut target.filter, "Dual band");
            labeled(ui, "Gain ou ISO", &mut target.gain_iso, "gain 200");
            labeled(ui, "Oculaire", &mut target.eyepiece, "25 mm");
            ui.horizontal(|ui| {
                opt_f32(
                    ui,
                    "Pose (s)",
                    &mut target.exposure_s,
                    30.0,
                    0.0..=36_000.0,
                    1.0,
                );
                opt_u32(
                    ui,
                    "Nombre d'images",
                    &mut target.frame_count,
                    10,
                    0..=1_000_000,
                );
            });
            ui.label("Notes");
            ui.add(
                egui::TextEdit::multiline(&mut target.notes)
                    .desired_rows(3)
                    .desired_width(f32::INFINITY),
            );
        }
        self.attachment_strip(ui, session_id, &target_id);
    }

    fn attachment_strip(&mut self, ui: &mut egui::Ui, session_id: &str, target_id: &str) {
        let rows = self.attachment_rows(session_id, target_id);
        ui.add_space(6.0);
        let mut import = false;
        if ui.button("Ajouter une photo ou un document").clicked() {
            import = true;
        }
        let mut open = None;
        let mut remove = None;
        for row in &rows {
            ui.add_space(6.0);
            let stored = row.stored_name.clone();
            let attachment_id = row.id.clone();
            let original = row.original_name.clone();
            ui.push_id(&attachment_id, |ui| {
                group(ui, &original, |ui| {
                    if is_image(&stored) {
                        self.thumb(ui, &stored);
                    } else {
                        ui.label("Document");
                    }
                    if let Some(attachment) =
                        self.attachment_mut(session_id, target_id, &attachment_id)
                    {
                        labeled(
                            ui,
                            "Légende",
                            &mut attachment.caption,
                            "Coeur de la nébuleuse",
                        );
                    }
                    ui.horizontal(|ui| {
                        if ui.button("Ouvrir").clicked() {
                            open = Some(stored.clone());
                        }
                        if ui.button("Retirer").clicked() {
                            remove = Some((attachment_id.clone(), stored.clone()));
                        }
                    });
                });
            });
        }
        if let Some(stored) = open {
            open_file(&store::attachment_path(&stored));
        }
        if let Some((attachment_id, stored)) = remove {
            self.remove_attachment(session_id, target_id, &attachment_id, &stored);
        }
        if import {
            self.import_attachments(session_id, target_id);
        }
    }

    fn attachment_mut(
        &mut self,
        session_id: &str,
        target_id: &str,
        attachment_id: &str,
    ) -> Option<&mut Attachment> {
        self.target_mut(session_id, target_id)?
            .attachments
            .iter_mut()
            .find(|attachment| attachment.id == attachment_id)
    }

    fn attachment_rows(&self, session_id: &str, target_id: &str) -> Vec<AttachmentRow> {
        self.journal
            .sessions
            .iter()
            .find(|session| session.id == session_id)
            .and_then(|session| session.targets.iter().find(|target| target.id == target_id))
            .map(|target| {
                target
                    .attachments
                    .iter()
                    .map(|attachment| AttachmentRow {
                        id: attachment.id.clone(),
                        stored_name: attachment.stored_name.clone(),
                        original_name: attachment.original_name.clone(),
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    fn thumb(&mut self, ui: &mut egui::Ui, stored: &str) {
        if !self.thumbs.contains_key(stored) {
            let image = load_thumbnail(&store::attachment_path(stored));
            let handle = image.map(|image| {
                ui.ctx()
                    .load_texture(stored, image, egui::TextureOptions::LINEAR)
            });
            self.thumbs.insert(stored.to_string(), handle);
        }
        match self.thumbs.get(stored).and_then(|handle| handle.as_ref()) {
            Some(handle) => {
                ui.add(egui::Image::from_texture(handle).max_size(egui::vec2(240.0, 160.0)));
            }
            None => {
                ui.label("Aperçu indisponible");
            }
        }
    }

    fn import_attachments(&mut self, session_id: &str, target_id: &str) {
        let Some(paths) = rfd::FileDialog::new()
            .set_title("Ajouter une astrophoto ou un document")
            .pick_files()
        else {
            return;
        };
        let mut errors = Vec::new();
        for path in paths {
            match attachments::import_attachment(&path, &store::media_dir()) {
                Ok(attachment) => {
                    if let Some(target) = self.target_mut(session_id, target_id) {
                        target.attachments.push(attachment);
                    }
                }
                Err(err) => {
                    let name = path
                        .file_name()
                        .map(|name| name.to_string_lossy().into_owned())
                        .unwrap_or_else(|| path.display().to_string());
                    errors.push(format!("{name} : {}", err.message()));
                }
            }
        }
        if !errors.is_empty() {
            self.notice = Some(errors.join("\n"));
        }
    }

    fn remove_attachment(
        &mut self,
        session_id: &str,
        target_id: &str,
        attachment_id: &str,
        stored: &str,
    ) {
        if let Some(target) = self.target_mut(session_id, target_id) {
            target
                .attachments
                .retain(|attachment| attachment.id != attachment_id);
        }
        self.thumbs.remove(stored);
        store::remove_stored(stored);
    }

    fn remove_target(&mut self, session_id: &str, target_id: &str) {
        let stored = self.attachment_rows(session_id, target_id);
        if let Some(session) = self
            .journal
            .sessions
            .iter_mut()
            .find(|session| session.id == session_id)
        {
            session.targets.retain(|target| target.id != target_id);
            self.selected_target = session.targets.first().map(|target| target.id.clone());
        }
        for row in stored {
            self.thumbs.remove(&row.stored_name);
            store::remove_stored(&row.stored_name);
        }
    }

    fn target_mut(&mut self, session_id: &str, target_id: &str) -> Option<&mut Target> {
        self.journal
            .sessions
            .iter_mut()
            .find(|session| session.id == session_id)?
            .targets
            .iter_mut()
            .find(|target| target.id == target_id)
    }

    fn search(&mut self, ui: &mut egui::Ui) {
        group(ui, "Photos d'un objet", |ui| self.photo_query(ui));
        group(ui, "Soirées d'une année", |ui| self.year_query(ui));
    }

    fn photo_query(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label("Nom");
            ui.add(
                egui::TextEdit::singleline(&mut self.object_query)
                    .hint_text("M42")
                    .desired_width(220.0),
            );
        });
        ui.label("Les PNG et les JPEG dont le nom contient cette recherche sont listés ici.");
        if self.object_query.trim().is_empty() {
            ui.label("Saisissez un nom, par exemple M42.");
            return;
        }
        let photos = photos_of(&self.journal, &self.object_query);
        ui.label(RichText::new(photos_phrase(photos.len())).strong());
        if photos.is_empty() {
            ui.label("Aucune photo pour ce nom.");
            return;
        }
        let mut open_session = None;
        for photo in &photos {
            let title = if photo.date.is_empty() {
                photo.target_name.clone()
            } else {
                format!("{}  {}", photo.date, photo.target_name)
            };
            ui.add_space(6.0);
            let mut open = false;
            ui.push_id(&photo.stored_name, |ui| {
                group(ui, &title, |ui| {
                    self.thumb(ui, &photo.stored_name);
                    if !photo.caption.is_empty() {
                        ui.label(&photo.caption);
                    }
                    ui.label(&photo.original_name);
                    if ui.button("Ouvrir la soirée").clicked() {
                        open = true;
                    }
                });
            });
            if open {
                open_session = Some((photo.session_id.clone(), photo.target_id.clone()));
            }
        }
        if let Some((session_id, target_id)) = open_session {
            self.view = View::Journal;
            self.selected_session = Some(session_id);
            self.selected_target = Some(target_id);
        }
    }

    fn year_query(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label("Année");
            ui.add(
                egui::TextEdit::singleline(&mut self.year_query)
                    .hint_text("2026")
                    .desired_width(80.0),
            );
        });
        if self.year_query.trim().is_empty() {
            ui.label("Saisissez une année, par exemple 2026.");
            return;
        }
        let Some(year) = query::parse_year(&self.year_query) else {
            ui.label("Indiquez une année sur quatre chiffres, par exemple 2026.");
            return;
        };
        let nights = nights_in_year(&self.journal, year);
        ui.label(RichText::new(format!("{} en {year}", evenings_phrase(nights.len()))).strong());
        if nights.is_empty() {
            ui.label(format!("Aucune soirée en {year}."));
            return;
        }
        let mut open_session = None;
        for night in &nights {
            ui.push_id(&night.session_id, |ui| {
                if ui
                    .button(format!("{}    {}", night.date, night.site_name))
                    .clicked()
                {
                    open_session = Some(night.session_id.clone());
                }
            });
        }
        if let Some(session_id) = open_session {
            let first_target = self
                .journal
                .sessions
                .iter()
                .find(|session| session.id == session_id)
                .and_then(|session| session.targets.first().map(|target| target.id.clone()));
            self.view = View::Journal;
            self.selected_session = Some(session_id);
            self.selected_target = first_target;
        }
    }

    fn profiles_window(&mut self, ctx: &egui::Context) {
        if !self.profiles_open {
            return;
        }
        let mut open = self.profiles_open;
        egui::Window::new("Profils")
            .open(&mut open)
            .default_size([780.0, 540.0])
            .show(ctx, |ui| self.profiles_body(ui));
        self.profiles_open = open;
    }

    fn profiles_body(&mut self, ui: &mut egui::Ui) {
        ui.horizontal_wrapped(|ui| {
            if ui
                .selectable_label(self.profile_tab == ProfileTab::Sites, "Sites")
                .clicked()
            {
                self.profile_tab = ProfileTab::Sites;
            }
            if ui
                .selectable_label(self.profile_tab == ProfileTab::Telescopes, "Télescopes")
                .clicked()
            {
                self.profile_tab = ProfileTab::Telescopes;
            }
            if ui
                .selectable_label(self.profile_tab == ProfileTab::Cameras, "Caméras astro")
                .clicked()
            {
                self.profile_tab = ProfileTab::Cameras;
            }
            if ui
                .selectable_label(self.profile_tab == ProfileTab::Dslr, "Reflex")
                .clicked()
            {
                self.profile_tab = ProfileTab::Dslr;
            }
            if ui
                .selectable_label(
                    self.profile_tab == ProfileTab::Smart,
                    "Télescopes intelligents",
                )
                .clicked()
            {
                self.profile_tab = ProfileTab::Smart;
            }
        });
        ui.separator();
        let mut add = false;
        let mut delete = false;
        ui.horizontal(|ui| {
            if ui.button("Ajouter").clicked() {
                add = true;
            }
            if ui.button("Supprimer").clicked() {
                delete = true;
            }
        });
        if add {
            self.add_profile();
        }
        if delete {
            self.delete_profile();
        }
        let items = self.profile_items();
        ui.columns(2, |columns| {
            let (left, right) = columns.split_at_mut(1);
            self.profile_list(&mut left[0], &items);
            self.profile_form(&mut right[0]);
        });
    }

    fn profile_items(&self) -> Vec<(String, String)> {
        match self.profile_tab {
            ProfileTab::Sites => names(&self.journal.sites, |site| &site.id, |site| &site.name),
            ProfileTab::Telescopes => {
                names(&self.journal.telescopes, |item| &item.id, |item| &item.name)
            }
            ProfileTab::Cameras => camera_names(&self.journal.cameras, CameraKind::Astro),
            ProfileTab::Dslr => camera_names(&self.journal.cameras, CameraKind::Dslr),
            ProfileTab::Smart => names(
                &self.journal.smart_scopes,
                |item| &item.id,
                |item| &item.name,
            ),
        }
    }

    fn profile_list(&mut self, ui: &mut egui::Ui, items: &[(String, String)]) {
        let selected = self.selected_profile().cloned();
        egui::ScrollArea::vertical()
            .id_salt(("profiles", self.profile_tab as u8))
            .max_height(360.0)
            .show(ui, |ui| {
                if items.is_empty() {
                    ui.label("Aucun profil.");
                    return;
                }
                let mut pick = None;
                for (id, name) in items {
                    ui.push_id(id, |ui| {
                        if ui
                            .selectable_label(selected.as_ref() == Some(id), display_name(name))
                            .clicked()
                        {
                            pick = Some(id.clone());
                        }
                    });
                }
                if let Some(id) = pick {
                    self.set_selected_profile(id);
                }
            });
    }

    fn selected_profile(&self) -> Option<&String> {
        match self.profile_tab {
            ProfileTab::Sites => self.selected_site.as_ref(),
            ProfileTab::Telescopes => self.selected_telescope.as_ref(),
            ProfileTab::Cameras | ProfileTab::Dslr => self.selected_camera.as_ref(),
            ProfileTab::Smart => self.selected_smart.as_ref(),
        }
    }

    fn set_selected_profile(&mut self, id: String) {
        match self.profile_tab {
            ProfileTab::Sites => self.selected_site = Some(id),
            ProfileTab::Telescopes => self.selected_telescope = Some(id),
            ProfileTab::Cameras | ProfileTab::Dslr => self.selected_camera = Some(id),
            ProfileTab::Smart => self.selected_smart = Some(id),
        }
    }

    fn add_profile(&mut self) {
        match self.profile_tab {
            ProfileTab::Sites => {
                let site = Site::create();
                self.selected_site = Some(site.id.clone());
                self.journal.sites.push(site);
            }
            ProfileTab::Telescopes => {
                let telescope = Telescope::create();
                self.selected_telescope = Some(telescope.id.clone());
                self.journal.telescopes.push(telescope);
            }
            ProfileTab::Cameras => {
                let camera = Camera::create();
                self.selected_camera = Some(camera.id.clone());
                self.journal.cameras.push(camera);
            }
            ProfileTab::Dslr => {
                let camera = Camera::create_dslr();
                self.selected_camera = Some(camera.id.clone());
                self.journal.cameras.push(camera);
            }
            ProfileTab::Smart => {
                let scope = SmartTelescope::create();
                self.selected_smart = Some(scope.id.clone());
                self.journal.smart_scopes.push(scope);
            }
        }
    }

    fn delete_profile(&mut self) {
        let Some(id) = self.selected_profile().cloned() else {
            return;
        };
        let kind = match self.profile_tab {
            ProfileTab::Sites => ProfileKind::Site,
            ProfileTab::Telescopes => ProfileKind::Telescope,
            ProfileTab::Cameras | ProfileTab::Dslr => ProfileKind::Camera,
            ProfileTab::Smart => ProfileKind::Smart,
        };
        let usages = profile_usages(&self.journal, kind, &id);
        if !usages.is_empty() {
            let name = self.profile_name(&id);
            self.notice = Some(usage_message(&name, &usages));
            return;
        }
        match self.profile_tab {
            ProfileTab::Sites => {
                self.journal.sites.retain(|site| site.id != id);
                self.selected_site = self.journal.sites.first().map(|site| site.id.clone());
            }
            ProfileTab::Telescopes => {
                self.journal.telescopes.retain(|item| item.id != id);
                self.selected_telescope =
                    self.journal.telescopes.first().map(|item| item.id.clone());
            }
            ProfileTab::Cameras => {
                self.journal.cameras.retain(|item| item.id != id);
                self.selected_camera = self
                    .journal
                    .cameras
                    .iter()
                    .find(|item| item.kind == CameraKind::Astro)
                    .map(|item| item.id.clone());
            }
            ProfileTab::Dslr => {
                self.journal.cameras.retain(|item| item.id != id);
                self.selected_camera = self
                    .journal
                    .cameras
                    .iter()
                    .find(|item| item.kind == CameraKind::Dslr)
                    .map(|item| item.id.clone());
            }
            ProfileTab::Smart => {
                self.journal.smart_scopes.retain(|item| item.id != id);
                self.selected_smart = self
                    .journal
                    .smart_scopes
                    .first()
                    .map(|item| item.id.clone());
            }
        }
    }

    fn profile_name(&self, id: &str) -> String {
        let name = match self.profile_tab {
            ProfileTab::Sites => self
                .journal
                .sites
                .iter()
                .find(|site| site.id == id)
                .map(|site| site.name.as_str()),
            ProfileTab::Telescopes => self
                .journal
                .telescopes
                .iter()
                .find(|item| item.id == id)
                .map(|item| item.name.as_str()),
            ProfileTab::Cameras | ProfileTab::Dslr => self
                .journal
                .cameras
                .iter()
                .find(|item| item.id == id)
                .map(|item| item.name.as_str()),
            ProfileTab::Smart => self
                .journal
                .smart_scopes
                .iter()
                .find(|item| item.id == id)
                .map(|item| item.name.as_str()),
        };
        name.unwrap_or("").to_string()
    }

    fn profile_form(&mut self, ui: &mut egui::Ui) {
        match self.profile_tab {
            ProfileTab::Sites => self.site_form(ui),
            ProfileTab::Telescopes => self.telescope_form(ui),
            ProfileTab::Cameras => self.astro_camera_form(ui),
            ProfileTab::Dslr => self.dslr_form(ui),
            ProfileTab::Smart => self.smart_form(ui),
        }
    }

    fn site_form(&mut self, ui: &mut egui::Ui) {
        let Some(id) = self.selected_site.clone() else {
            ui.label("Sélectionnez un site ou ajoutez-en un.");
            return;
        };
        let Some(site) = self.journal.sites.iter_mut().find(|site| site.id == id) else {
            ui.label("Sélectionnez un site ou ajoutez-en un.");
            return;
        };
        labeled(ui, "Nom", &mut site.name, "Cour");
        labeled(ui, "Latitude", &mut site.latitude, "45.5 N");
        labeled(ui, "Longitude", &mut site.longitude, "73.5 O");
        opt_f32(
            ui,
            "Altitude (m)",
            &mut site.altitude_m,
            0.0,
            -500.0..=9000.0,
            1.0,
        );
        opt_u8(ui, "Bortle (1 à 9)", &mut site.bortle, 4, 1..=9);
        notes_field(ui, &mut site.notes);
    }

    fn telescope_form(&mut self, ui: &mut egui::Ui) {
        let Some(id) = self.selected_telescope.clone() else {
            ui.label("Sélectionnez un télescope ou ajoutez-en un.");
            return;
        };
        let Some(telescope) = self
            .journal
            .telescopes
            .iter_mut()
            .find(|item| item.id == id)
        else {
            ui.label("Sélectionnez un télescope ou ajoutez-en un.");
            return;
        };
        labeled(ui, "Nom", &mut telescope.name, "80ED");
        labeled(ui, "Type", &mut telescope.kind, "Lunette");
        opt_f32(
            ui,
            "Ouverture (mm)",
            &mut telescope.aperture_mm,
            80.0,
            1.0..=2_000.0,
            1.0,
        );
        opt_f32(
            ui,
            "Focale (mm)",
            &mut telescope.focal_mm,
            400.0,
            1.0..=20_000.0,
            1.0,
        );
        notes_field(ui, &mut telescope.notes);
    }

    fn astro_camera_form(&mut self, ui: &mut egui::Ui) {
        let Some(id) = self.selected_camera.clone() else {
            ui.label("Sélectionnez une caméra astro ou ajoutez-en une.");
            return;
        };
        let Some(camera) = self
            .journal
            .cameras
            .iter_mut()
            .find(|item| item.id == id && item.kind == CameraKind::Astro)
        else {
            ui.label("Sélectionnez une caméra astro ou ajoutez-en une.");
            return;
        };
        labeled(ui, "Nom", &mut camera.name, "ASI585");
        labeled(ui, "Capteur", &mut camera.sensor, "IMX585");
        opt_f32(ui, "Pixel (µm)", &mut camera.pixel_um, 3.8, 0.1..=50.0, 0.1);
        ui.checkbox(&mut camera.cooled, "Refroidie");
        notes_field(ui, &mut camera.notes);
    }

    fn dslr_form(&mut self, ui: &mut egui::Ui) {
        let Some(id) = self.selected_camera.clone() else {
            ui.label("Sélectionnez un reflex ou ajoutez-en un.");
            return;
        };
        let Some(camera) = self
            .journal
            .cameras
            .iter_mut()
            .find(|item| item.id == id && item.kind == CameraKind::Dslr)
        else {
            ui.label("Sélectionnez un reflex ou ajoutez-en un.");
            return;
        };
        labeled(ui, "Nom", &mut camera.name, "Mon R6");
        labeled(ui, "Modèle", &mut camera.model, "Canon EOS R6");
        labeled(ui, "Monture", &mut camera.mount, "RF");
        opt_f32(ui, "Facteur de crop", &mut camera.crop_factor, 1.0, 1.0..=3.0, 0.01);
        notes_field(ui, &mut camera.notes);
    }

    fn smart_form(&mut self, ui: &mut egui::Ui) {
        let Some(id) = self.selected_smart.clone() else {
            ui.label("Sélectionnez un télescope intelligent ou ajoutez-en un.");
            return;
        };
        let Some(scope) = self
            .journal
            .smart_scopes
            .iter_mut()
            .find(|item| item.id == id)
        else {
            ui.label("Sélectionnez un télescope intelligent ou ajoutez-en un.");
            return;
        };
        labeled(ui, "Nom", &mut scope.name, "SeeStar cour");
        smart_model(ui, &mut scope.model, &id);
        opt_f32(
            ui,
            "Ouverture (mm)",
            &mut scope.aperture_mm,
            30.0,
            1.0..=2_000.0,
            1.0,
        );
        opt_f32(
            ui,
            "Focale (mm)",
            &mut scope.focal_mm,
            150.0,
            1.0..=20_000.0,
            1.0,
        );
        notes_field(ui, &mut scope.notes);
    }

    fn confirm_delete(&mut self, ctx: &egui::Context) {
        let Some(id) = self.pending_delete.clone() else {
            return;
        };
        let date = self
            .journal
            .sessions
            .iter()
            .find(|session| session.id == id)
            .map(|session| {
                if session.date.is_empty() {
                    "sans date".to_string()
                } else {
                    session.date.clone()
                }
            })
            .unwrap_or_else(|| "sans date".to_string());
        let mut open = true;
        let mut confirm = false;
        let mut dismiss = false;
        egui::Window::new("Supprimer la soirée")
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
            .show(ctx, |ui| {
                ui.label(format!("Supprimer la soirée du {date} ?"));
                ui.label(
                    "Les objets et les fichiers joints de cette soirée seront retirés du journal.",
                );
                ui.horizontal(|ui| {
                    if ui.button("Annuler").clicked() {
                        dismiss = true;
                    }
                    if ui.button("Supprimer").clicked() {
                        confirm = true;
                    }
                });
            });
        if confirm {
            self.pending_delete = None;
            self.delete_session(&id);
        } else if dismiss || !open {
            self.pending_delete = None;
        }
    }

    fn delete_session(&mut self, id: &str) {
        let Some(index) = self
            .journal
            .sessions
            .iter()
            .position(|session| session.id == id)
        else {
            return;
        };
        let session = self.journal.sessions.remove(index);
        for target in session.targets {
            for attachment in target.attachments {
                self.thumbs.remove(&attachment.stored_name);
                store::remove_stored(&attachment.stored_name);
            }
        }
        if self.selected_session.as_deref() == Some(id) {
            self.selected_session = None;
            self.selected_target = None;
        }
    }

    fn notice_window(&mut self, ctx: &egui::Context) {
        let Some(message) = self.notice.clone() else {
            return;
        };
        let mut open = true;
        let mut dismiss = false;
        egui::Window::new("Journal")
            .open(&mut open)
            .collapsible(false)
            .resizable(true)
            .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
            .show(ctx, |ui| {
                ui.set_max_width(480.0);
                ui.label(message);
                if ui.button("OK").clicked() {
                    dismiss = true;
                }
            });
        if dismiss || !open {
            self.notice = None;
        }
    }

    fn status(&self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label(format!("Journal : {}", store::data_dir().display()));
            if !self.can_save {
                ui.label("Enregistrement bloqué");
            }
        });
    }
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        egui::Panel::left("sessions")
            .exact_size(340.0)
            .resizable(false)
            .show(ui, |ui| self.sessions_panel(ui));
        egui::Panel::bottom("status").show(ui, |ui| self.status(ui));
        egui::CentralPanel::default().show(ui, |ui| {
            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    ui.set_min_width(ui.available_width());
                    if self.view == View::Search {
                        self.search(ui);
                    } else {
                        self.session_page(ui);
                    }
                });
        });
        self.profiles_window(&ctx);
        self.confirm_delete(&ctx);
        self.notice_window(&ctx);
        self.persist();
    }

    fn on_exit(&mut self) {
        self.persist();
    }
}

pub fn run() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1280.0, 820.0])
            .with_min_inner_size([980.0, 640.0])
            .with_title(format!("dubastrolog    v{}", env!("CARGO_PKG_VERSION")))
            .with_icon(app_icon()),
        centered: true,
        ..Default::default()
    };
    eframe::run_native(
        "dubastrolog",
        options,
        Box::new(|cc| Ok(Box::new(App::new(cc)))),
    )
}

fn app_icon() -> egui::IconData {
    let image = image::load_from_memory(include_bytes!("../images/dubastrolog.png"))
        .expect("images/dubastrolog.png");
    let image = if image.width() > 256 || image.height() > 256 {
        image.resize(256, 256, image::imageops::FilterType::Lanczos3)
    } else {
        image
    };
    let image = image.into_rgba8();
    let (width, height) = image.dimensions();
    egui::IconData {
        rgba: image.into_raw(),
        width,
        height,
    }
}

fn theme() -> egui::Visuals {
    let mut visuals = egui::Visuals::dark();
    let label = Color32::from_rgb(0xff, 0xe1, 0x4d);
    let control = Color32::from_rgb(0xff, 0x93, 0x20);
    visuals.window_fill = Color32::from_rgb(0x16, 0x05, 0x05);
    visuals.panel_fill = Color32::from_rgb(0x16, 0x05, 0x05);
    visuals.extreme_bg_color = Color32::from_rgb(0x0e, 0x03, 0x03);
    visuals.faint_bg_color = Color32::from_rgb(0x3d, 0x12, 0x10);
    visuals.widgets.noninteractive.bg_fill = Color32::from_rgb(0x3d, 0x12, 0x10);
    visuals.widgets.noninteractive.fg_stroke.color = label;
    visuals.widgets.inactive.bg_fill = Color32::from_rgb(0x62, 0x20, 0x18);
    visuals.widgets.inactive.fg_stroke.color = control;
    visuals.widgets.hovered.bg_fill = Color32::from_rgb(0x7a, 0x28, 0x20);
    visuals.widgets.hovered.fg_stroke.color = control;
    visuals.widgets.active.bg_fill = Color32::from_rgb(0x4a, 0x14, 0x14);
    visuals.widgets.active.fg_stroke.color = control;
    visuals.widgets.open.bg_fill = Color32::from_rgb(0x62, 0x20, 0x18);
    visuals.widgets.open.fg_stroke.color = control;
    visuals.selection.bg_fill = Color32::from_rgb(0xc4, 0x3a, 0x18);
    visuals.selection.stroke.color = control;
    visuals
}

fn install_font(ctx: &egui::Context) {
    let candidates = [
        r"C:\Windows\Fonts\segoeuib.ttf",
        r"C:\Windows\Fonts\arialbd.ttf",
    ];
    let Some(bytes) = candidates.iter().find_map(|path| std::fs::read(path).ok()) else {
        return;
    };
    ctx.add_font(egui::FontInsert::new(
        "stack-bold",
        egui::FontData::from_owned(bytes),
        vec![egui::InsertFontFamily {
            family: egui::FontFamily::Proportional,
            priority: egui::FontPriority::Highest,
        }],
    ));
}

fn group(ui: &mut egui::Ui, title: &str, add: impl FnOnce(&mut egui::Ui)) {
    ui.add_space(6.0);
    ui.label(RichText::new(title).strong());
    Frame::NONE
        .fill(Color32::from_rgb(0x3d, 0x12, 0x10))
        .stroke(Stroke::new(1.0, Color32::from_rgb(0x8e, 0x32, 0x24)))
        .inner_margin(8.0)
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            add(ui);
        });
}

fn labeled(ui: &mut egui::Ui, label: &str, text: &mut String, hint: &str) {
    ui.horizontal(|ui| {
        ui.label(label);
        ui.add(
            egui::TextEdit::singleline(text)
                .hint_text(hint)
                .desired_width(200.0),
        );
    });
}

fn notes_field(ui: &mut egui::Ui, notes: &mut String) {
    ui.label("Notes");
    ui.add(
        egui::TextEdit::multiline(notes)
            .desired_rows(3)
            .desired_width(f32::INFINITY),
    );
}

fn combo_option(
    ui: &mut egui::Ui,
    salt: &str,
    label: &str,
    current: &mut Option<String>,
    items: &[(String, String)],
    none_label: &str,
) {
    let selected = items
        .iter()
        .find(|(id, _)| Some(id) == current.as_ref())
        .map(|(_, name)| display_name(name).to_string())
        .unwrap_or_else(|| none_label.to_string());
    ui.horizontal(|ui| {
        ui.label(label);
        egui::ComboBox::from_id_salt(salt)
            .selected_text(selected)
            .width(220.0)
            .show_ui(ui, |ui| {
                if ui.selectable_label(current.is_none(), none_label).clicked() {
                    *current = None;
                }
                for (id, name) in items {
                    if ui
                        .selectable_label(
                            current.as_deref() == Some(id.as_str()),
                            display_name(name),
                        )
                        .clicked()
                    {
                        *current = Some(id.clone());
                    }
                }
            });
    });
}

fn enum_combo<T: Copy + PartialEq>(
    ui: &mut egui::Ui,
    salt: &str,
    label: &str,
    current: &mut T,
    options: &[(T, &str)],
) {
    let selected = options
        .iter()
        .find(|(value, _)| value == current)
        .map(|(_, name)| *name)
        .unwrap_or("");
    ui.horizontal(|ui| {
        ui.label(label);
        egui::ComboBox::from_id_salt(salt)
            .selected_text(selected)
            .width(220.0)
            .show_ui(ui, |ui| {
                for (value, name) in options {
                    ui.selectable_value(current, *value, *name);
                }
            });
    });
}

fn smart_model(ui: &mut egui::Ui, model: &mut String, salt: &str) {
    let preset = SMART_PRESETS.contains(&model.as_str());
    let current = if preset {
        model.clone()
    } else {
        "Autre".to_string()
    };
    let mut choice = current.clone();
    ui.horizontal(|ui| {
        ui.label("Modèle");
        egui::ComboBox::from_id_salt(("smart-model", salt))
            .selected_text(if preset { model.as_str() } else { "Autre" })
            .width(220.0)
            .show_ui(ui, |ui| {
                for item in SMART_PRESETS {
                    ui.selectable_value(&mut choice, (*item).to_string(), *item);
                }
                ui.selectable_value(&mut choice, "Autre".to_string(), "Autre");
            });
    });
    if choice != current {
        if choice == "Autre" {
            model.clear();
        } else {
            *model = choice;
        }
    }
    if !SMART_PRESETS.contains(&model.as_str()) {
        labeled(ui, "Autre modèle", model, "Dwarf 3");
    }
}

fn opt_f32(
    ui: &mut egui::Ui,
    label: &str,
    value: &mut Option<f32>,
    initial: f32,
    range: RangeInclusive<f32>,
    speed: f64,
) {
    ui.horizontal(|ui| {
        let mut on = value.is_some();
        if ui.checkbox(&mut on, label).changed() {
            if on {
                if value.is_none() {
                    *value = Some(initial);
                }
            } else {
                *value = None;
            }
        }
        if let Some(number) = value.as_mut() {
            ui.add(
                egui::DragValue::new(number)
                    .range(range)
                    .speed(speed)
                    .fixed_decimals(1),
            );
        }
    });
}

fn opt_u8(
    ui: &mut egui::Ui,
    label: &str,
    value: &mut Option<u8>,
    initial: u8,
    range: RangeInclusive<u8>,
) {
    ui.horizontal(|ui| {
        let mut on = value.is_some();
        if ui.checkbox(&mut on, label).changed() {
            if on {
                if value.is_none() {
                    *value = Some(initial);
                }
            } else {
                *value = None;
            }
        }
        if let Some(number) = value.as_mut() {
            ui.add(egui::DragValue::new(number).range(range).speed(1.0));
        }
    });
}

fn opt_u32(
    ui: &mut egui::Ui,
    label: &str,
    value: &mut Option<u32>,
    initial: u32,
    range: RangeInclusive<u32>,
) {
    ui.horizontal(|ui| {
        let mut on = value.is_some();
        if ui.checkbox(&mut on, label).changed() {
            if on {
                if value.is_none() {
                    *value = Some(initial);
                }
            } else {
                *value = None;
            }
        }
        if let Some(number) = value.as_mut() {
            ui.add(egui::DragValue::new(number).range(range).speed(1.0));
        }
    });
}

fn objects_phrase(count: usize) -> String {
    if count == 1 {
        "1 objet".to_string()
    } else {
        format!("{count} objets")
    }
}

fn photos_phrase(count: usize) -> String {
    if count == 1 {
        "1 photo".to_string()
    } else {
        format!("{count} photos")
    }
}

fn evenings_phrase(count: usize) -> String {
    if count == 1 {
        "1 soirée".to_string()
    } else {
        format!("{count} soirées")
    }
}

fn usage_message(name: &str, usages: &[String]) -> String {
    let label = if name.trim().is_empty() {
        "(sans nom)"
    } else {
        name
    };
    let shown = usages
        .iter()
        .take(8)
        .cloned()
        .collect::<Vec<_>>()
        .join("\n");
    let extra = if usages.len() > 8 {
        format!("\n… et {} autre(s)", usages.len() - 8)
    } else {
        String::new()
    };
    format!(
        "« {label} » est encore utilisé :\n{shown}{extra}\nRetirez-le de ces soirées avant de le supprimer."
    )
}

fn describe_instrument(instrument: &Instrument, catalog: &Catalog) -> String {
    match instrument.kind {
        InstrumentKind::Smart => match Catalog::find(&catalog.smarts, &instrument.smart_id)
            .filter(|name| !name.trim().is_empty())
        {
            Some(name) => format!("Télescope intelligent : {name}"),
            None => "Télescope intelligent : aucun".to_string(),
        },
        InstrumentKind::Telescope => {
            let scope = Catalog::find(&catalog.telescopes, &instrument.telescope_id)
                .filter(|name| !name.trim().is_empty())
                .unwrap_or("aucun télescope");
            let camera = Catalog::find(&catalog.cameras, &instrument.camera_id)
                .filter(|name| !name.trim().is_empty())
                .unwrap_or("aucune caméra");
            format!("Télescope : {scope} · Caméra : {camera}")
        }
    }
}

fn load_thumbnail(path: &std::path::Path) -> Option<egui::ColorImage> {
    let image = image::open(path).ok()?.thumbnail(480, 480);
    let rgba = image.to_rgba8();
    let (width, height) = rgba.dimensions();
    Some(egui::ColorImage::from_rgba_unmultiplied(
        [width as usize, height as usize],
        rgba.as_raw(),
    ))
}
