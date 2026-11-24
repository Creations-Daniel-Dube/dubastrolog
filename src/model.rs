use serde::{Deserialize, Serialize};

pub fn new_id() -> String {
    uuid::Uuid::new_v4().to_string()
}

pub fn today() -> String {
    chrono::Local::now().format("%Y-%m-%d").to_string()
}

pub fn is_iso_date(date: &str) -> bool {
    let bytes = date.as_bytes();
    if bytes.len() != 10 || bytes[4] != b'-' || bytes[7] != b'-' {
        return false;
    }
    if !bytes
        .iter()
        .enumerate()
        .all(|(index, byte)| index == 4 || index == 7 || byte.is_ascii_digit())
    {
        return false;
    }
    let Ok(month) = date[5..7].parse::<u32>() else {
        return false;
    };
    let Ok(day) = date[8..10].parse::<u32>() else {
        return false;
    };
    (1..=12).contains(&month) && (1..=31).contains(&day)
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Journal {
    pub sites: Vec<Site>,
    pub telescopes: Vec<Telescope>,
    pub cameras: Vec<Camera>,
    pub smart_scopes: Vec<SmartTelescope>,
    pub sessions: Vec<Session>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Site {
    pub id: String,
    pub name: String,
    pub latitude: String,
    pub longitude: String,
    pub altitude_m: Option<f32>,
    pub bortle: Option<u8>,
    pub notes: String,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Telescope {
    pub id: String,
    pub name: String,
    pub aperture_mm: Option<f32>,
    pub focal_mm: Option<f32>,
    pub kind: String,
    pub notes: String,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum CameraKind {
    #[default]
    #[serde(rename = "astro")]
    Astro,
    #[serde(rename = "dslr")]
    Dslr,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Camera {
    pub id: String,
    pub kind: CameraKind,
    pub name: String,
    pub sensor: String,
    pub pixel_um: Option<f32>,
    pub cooled: bool,
    pub model: String,
    pub mount: String,
    pub crop_factor: Option<f32>,
    pub notes: String,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct SmartTelescope {
    pub id: String,
    pub name: String,
    pub model: String,
    pub aperture_mm: Option<f32>,
    pub focal_mm: Option<f32>,
    pub notes: String,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum InstrumentKind {
    #[default]
    #[serde(rename = "telescope")]
    Telescope,
    #[serde(rename = "smart")]
    Smart,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Instrument {
    pub kind: InstrumentKind,
    pub telescope_id: Option<String>,
    pub camera_id: Option<String>,
    pub smart_id: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Weather {
    pub temperature_c: Option<f32>,
    pub humidity_pct: Option<f32>,
    pub pressure_hpa: Option<f32>,
    pub wind_kmh: Option<f32>,
    pub wind_dir: String,
    pub cloud_cover: Option<u8>,
    pub seeing: Option<u8>,
    pub seeing_arcsec: Option<f32>,
    pub transparency: Option<u8>,
    pub sqm: Option<f32>,
    pub moon_illumination_pct: Option<f32>,
    pub notes: String,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum TargetKind {
    #[default]
    #[serde(rename = "deep_sky")]
    DeepSky,
    #[serde(rename = "planet")]
    Planet,
    #[serde(rename = "moon")]
    Moon,
    #[serde(rename = "sun")]
    Sun,
    #[serde(rename = "comet")]
    Comet,
    #[serde(rename = "star")]
    Star,
    #[serde(rename = "double_star")]
    DoubleStar,
    #[serde(rename = "other")]
    Other,
}

impl TargetKind {
    pub fn label(self) -> &'static str {
        match self {
            Self::DeepSky => "Ciel profond",
            Self::Planet => "Planète",
            Self::Moon => "Lune",
            Self::Sun => "Soleil",
            Self::Comet => "Comète",
            Self::Star => "Étoile",
            Self::DoubleStar => "Étoile double",
            Self::Other => "Autre",
        }
    }

    pub fn all() -> [Self; 8] {
        [
            Self::DeepSky,
            Self::Planet,
            Self::Moon,
            Self::Sun,
            Self::Comet,
            Self::Star,
            Self::DoubleStar,
            Self::Other,
        ]
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum ObserveMode {
    #[serde(rename = "visual")]
    Visual,
    #[default]
    #[serde(rename = "photo")]
    Photo,
    #[serde(rename = "both")]
    Both,
}

impl ObserveMode {
    pub fn label(self) -> &'static str {
        match self {
            Self::Visual => "Visuel",
            Self::Photo => "Photo",
            Self::Both => "Visuel et photo",
        }
    }

    pub fn all() -> [Self; 3] {
        [Self::Visual, Self::Photo, Self::Both]
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum TargetInstrumentKind {
    #[default]
    #[serde(rename = "session")]
    Session,
    #[serde(rename = "telescope")]
    Telescope,
    #[serde(rename = "smart")]
    Smart,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Attachment {
    pub id: String,
    pub stored_name: String,
    pub original_name: String,
    pub caption: String,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Target {
    pub id: String,
    pub name: String,
    pub kind: TargetKind,
    pub mode: ObserveMode,
    pub time: String,
    pub instrument_kind: TargetInstrumentKind,
    pub telescope_id: Option<String>,
    pub camera_id: Option<String>,
    pub smart_id: Option<String>,
    pub filter: String,
    pub exposure_s: Option<f32>,
    pub gain_iso: String,
    pub frame_count: Option<u32>,
    pub eyepiece: String,
    pub notes: String,
    pub attachments: Vec<Attachment>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Session {
    pub id: String,
    pub date: String,
    pub start_time: String,
    pub end_time: String,
    pub site_id: Option<String>,
    pub instrument: Instrument,
    pub notes: String,
    pub weather: Weather,
    pub targets: Vec<Target>,
}

impl Site {
    pub fn create() -> Self {
        Self {
            id: new_id(),
            ..Self::default()
        }
    }
}

impl Telescope {
    pub fn create() -> Self {
        Self {
            id: new_id(),
            ..Self::default()
        }
    }
}

impl Camera {
    pub fn create() -> Self {
        Self {
            id: new_id(),
            kind: CameraKind::Astro,
            ..Self::default()
        }
    }

    pub fn create_dslr() -> Self {
        Self {
            id: new_id(),
            kind: CameraKind::Dslr,
            ..Self::default()
        }
    }
}

impl SmartTelescope {
    pub fn create() -> Self {
        Self {
            id: new_id(),
            model: "SeeStar S30".to_string(),
            ..Self::default()
        }
    }
}

impl Session {
    pub fn create() -> Self {
        Self {
            id: new_id(),
            date: today(),
            ..Self::default()
        }
    }
}

impl Target {
    pub fn create() -> Self {
        Self {
            id: new_id(),
            ..Self::default()
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProfileKind {
    Site,
    Telescope,
    Camera,
    Smart,
}

pub fn profile_usages(journal: &Journal, kind: ProfileKind, id: &str) -> Vec<String> {
    let mut lines = Vec::new();
    for session in &journal.sessions {
        let evening = if session.date.is_empty() {
            "Soirée sans date".to_string()
        } else {
            format!("Soirée du {}", session.date)
        };
        match kind {
            ProfileKind::Site => {
                if session.site_id.as_deref() == Some(id) {
                    lines.push(evening);
                }
            }
            ProfileKind::Telescope => {
                if session.instrument.kind == InstrumentKind::Telescope
                    && session.instrument.telescope_id.as_deref() == Some(id)
                {
                    lines.push(evening.clone());
                }
                for target in &session.targets {
                    if target.instrument_kind == TargetInstrumentKind::Telescope
                        && target.telescope_id.as_deref() == Some(id)
                    {
                        lines.push(format!("{evening}, objet {}", object_label(&target.name)));
                    }
                }
            }
            ProfileKind::Camera => {
                if session.instrument.kind == InstrumentKind::Telescope
                    && session.instrument.camera_id.as_deref() == Some(id)
                {
                    lines.push(evening.clone());
                }
                for target in &session.targets {
                    if target.instrument_kind == TargetInstrumentKind::Telescope
                        && target.camera_id.as_deref() == Some(id)
                    {
                        lines.push(format!("{evening}, objet {}", object_label(&target.name)));
                    }
                }
            }
            ProfileKind::Smart => {
                if session.instrument.kind == InstrumentKind::Smart
                    && session.instrument.smart_id.as_deref() == Some(id)
                {
                    lines.push(evening.clone());
                }
                for target in &session.targets {
                    if target.instrument_kind == TargetInstrumentKind::Smart
                        && target.smart_id.as_deref() == Some(id)
                    {
                        lines.push(format!("{evening}, objet {}", object_label(&target.name)));
                    }
                }
            }
        }
    }
    lines
}

pub fn object_label(name: &str) -> &str {
    if name.trim().is_empty() {
        "(sans nom)"
    } else {
        name
    }
}

pub fn display_name(name: &str) -> &str {
    if name.trim().is_empty() {
        "(sans nom)"
    } else {
        name
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_journal_reads_as_default() {
        let journal: Journal = serde_json::from_str("{}").unwrap();
        assert_eq!(journal, Journal::default());
    }

    #[test]
    fn legacy_camera_json_defaults_to_astro() {
        let journal: Journal = serde_json::from_str(
            r#"{"cameras":[{"id":"c1","name":"ASI585","sensor":"IMX585","cooled":true}]}"#,
        )
        .unwrap();
        assert_eq!(journal.cameras.len(), 1);
        assert_eq!(journal.cameras[0].kind, CameraKind::Astro);
    }

    #[test]
    fn journal_roundtrip_keeps_profiles_sessions_and_files() {
        let mut journal = Journal::default();
        journal.sites.push(Site {
            id: "site-1".to_string(),
            name: "Cour".to_string(),
            latitude: "45.5 N".to_string(),
            longitude: "73.5 O".to_string(),
            altitude_m: Some(30.0),
            bortle: Some(6),
            notes: "Maison".to_string(),
        });
        journal.telescopes.push(Telescope {
            id: "scope-1".to_string(),
            name: "80ED".to_string(),
            aperture_mm: Some(80.0),
            focal_mm: Some(560.0),
            kind: "Lunette".to_string(),
            notes: String::new(),
        });
        journal.cameras.push(Camera {
            id: "cam-1".to_string(),
            kind: CameraKind::Astro,
            name: "ASI585".to_string(),
            sensor: "IMX585".to_string(),
            pixel_um: Some(2.9),
            cooled: true,
            notes: String::new(),
            ..Camera::default()
        });
        journal.cameras.push(Camera {
            id: "cam-2".to_string(),
            kind: CameraKind::Dslr,
            name: "Reflex terrain".to_string(),
            model: "Canon EOS R6".to_string(),
            mount: "RF".to_string(),
            crop_factor: Some(1.0),
            notes: String::new(),
            ..Camera::default()
        });
        journal.smart_scopes.push(SmartTelescope {
            id: "smart-1".to_string(),
            name: "SeeStar cour".to_string(),
            model: "SeeStar S50".to_string(),
            aperture_mm: Some(50.0),
            focal_mm: Some(250.0),
            notes: String::new(),
        });
        journal.sessions.push(Session {
            id: "night-1".to_string(),
            date: "2026-11-23".to_string(),
            start_time: "20:15".to_string(),
            end_time: "23:40".to_string(),
            site_id: Some("site-1".to_string()),
            instrument: Instrument {
                kind: InstrumentKind::Smart,
                smart_id: Some("smart-1".to_string()),
                ..Instrument::default()
            },
            notes: "Ciel stable".to_string(),
            weather: Weather {
                temperature_c: Some(-4.5),
                humidity_pct: Some(72.0),
                seeing: Some(3),
                transparency: Some(4),
                moon_illumination_pct: Some(18.0),
                ..Weather::default()
            },
            targets: vec![Target {
                id: "obj-1".to_string(),
                name: "M42".to_string(),
                kind: TargetKind::DeepSky,
                mode: ObserveMode::Photo,
                filter: "Dual band".to_string(),
                exposure_s: Some(30.0),
                gain_iso: "gain 200".to_string(),
                frame_count: Some(40),
                attachments: vec![Attachment {
                    id: "file-1".to_string(),
                    stored_name: "file-1.png".to_string(),
                    original_name: "orion.png".to_string(),
                    caption: "Coeur".to_string(),
                }],
                ..Target::default()
            }],
        });

        let text = serde_json::to_string_pretty(&journal).unwrap();
        let restored: Journal = serde_json::from_str(&text).unwrap();
        assert_eq!(restored, journal);
    }

    #[test]
    fn used_profile_names_the_evening_and_the_object() {
        let journal = Journal {
            telescopes: vec![Telescope {
                id: "scope-1".to_string(),
                name: "80ED".to_string(),
                ..Telescope::default()
            }],
            sessions: vec![Session {
                date: "2026-01-02".to_string(),
                instrument: Instrument {
                    kind: InstrumentKind::Telescope,
                    telescope_id: Some("scope-1".to_string()),
                    ..Instrument::default()
                },
                targets: vec![Target {
                    name: "M31".to_string(),
                    instrument_kind: TargetInstrumentKind::Telescope,
                    telescope_id: Some("scope-1".to_string()),
                    ..Target::default()
                }],
                ..Session::default()
            }],
            ..Journal::default()
        };
        let usages = profile_usages(&journal, ProfileKind::Telescope, "scope-1");
        assert_eq!(
            usages,
            vec![
                "Soirée du 2026-01-02".to_string(),
                "Soirée du 2026-01-02, objet M31".to_string()
            ]
        );
    }

    #[test]
    fn iso_date_rejects_a_month_outside_the_calendar() {
        assert!(is_iso_date("2026-11-23"));
        assert!(!is_iso_date("2026-13-01"));
        assert!(!is_iso_date("23-11-2026"));
    }
}
