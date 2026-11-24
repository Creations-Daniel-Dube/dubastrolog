use crate::attachments::is_image;
use crate::model::{Journal, is_iso_date, object_label};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PhotoHit {
    pub session_id: String,
    pub target_id: String,
    pub date: String,
    pub target_name: String,
    pub caption: String,
    pub original_name: String,
    pub stored_name: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NightHit {
    pub session_id: String,
    pub date: String,
    pub site_name: String,
}

pub fn normalize_name(name: &str) -> String {
    name.chars()
        .filter(|c| !c.is_whitespace())
        .flat_map(|c| c.to_lowercase())
        .collect()
}

pub fn name_matches(object_name: &str, query: &str) -> bool {
    let query = normalize_name(query);
    !query.is_empty() && normalize_name(object_name).contains(&query)
}

pub fn parse_year(text: &str) -> Option<i32> {
    let text = text.trim();
    if text.len() != 4 || !text.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    text.parse().ok()
}

pub fn session_year(date: &str) -> Option<i32> {
    if !is_iso_date(date) {
        return None;
    }
    date[..4].parse().ok()
}

pub fn photos_of(journal: &Journal, query: &str) -> Vec<PhotoHit> {
    if normalize_name(query).is_empty() {
        return Vec::new();
    }
    let mut hits = Vec::new();
    for session in &journal.sessions {
        for target in &session.targets {
            if !name_matches(&target.name, query) {
                continue;
            }
            for attachment in &target.attachments {
                if !is_image(&attachment.stored_name) {
                    continue;
                }
                hits.push(PhotoHit {
                    session_id: session.id.clone(),
                    target_id: target.id.clone(),
                    date: session.date.clone(),
                    target_name: object_label(&target.name).to_string(),
                    caption: attachment.caption.clone(),
                    original_name: attachment.original_name.clone(),
                    stored_name: attachment.stored_name.clone(),
                });
            }
        }
    }
    hits.sort_by(|left, right| {
        right
            .date
            .cmp(&left.date)
            .then(left.original_name.cmp(&right.original_name))
    });
    hits
}

pub fn nights_in_year(journal: &Journal, year: i32) -> Vec<NightHit> {
    let mut hits = Vec::new();
    for session in &journal.sessions {
        if session_year(&session.date) != Some(year) {
            continue;
        }
        let site_name = session
            .site_id
            .as_ref()
            .and_then(|id| journal.sites.iter().find(|site| &site.id == id))
            .map(|site| object_label(&site.name).to_string())
            .unwrap_or_else(|| "Sans site".to_string());
        hits.push(NightHit {
            session_id: session.id.clone(),
            date: session.date.clone(),
            site_name,
        });
    }
    hits.sort_by(|left, right| {
        right
            .date
            .cmp(&left.date)
            .then(right.session_id.cmp(&left.session_id))
    });
    hits
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Attachment, Instrument, InstrumentKind, Session, Site, Target};

    fn sample() -> Journal {
        Journal {
            sites: vec![Site {
                id: "cour".to_string(),
                name: "Cour".to_string(),
                ..Site::default()
            }],
            sessions: vec![
                Session {
                    id: "n-2026".to_string(),
                    date: "2026-02-10".to_string(),
                    site_id: Some("cour".to_string()),
                    instrument: Instrument {
                        kind: InstrumentKind::Smart,
                        ..Instrument::default()
                    },
                    targets: vec![
                        Target {
                            id: "m42".to_string(),
                            name: "M42 — Nébuleuse d’Orion".to_string(),
                            attachments: vec![
                                Attachment {
                                    stored_name: "a.png".to_string(),
                                    original_name: "coeur.png".to_string(),
                                    caption: "Coeur".to_string(),
                                    ..Attachment::default()
                                },
                                Attachment {
                                    stored_name: "note.pdf".to_string(),
                                    original_name: "note.pdf".to_string(),
                                    ..Attachment::default()
                                },
                            ],
                            ..Target::default()
                        },
                        Target {
                            id: "m31".to_string(),
                            name: "M31".to_string(),
                            attachments: vec![Attachment {
                                stored_name: "m31.png".to_string(),
                                original_name: "m31.png".to_string(),
                                ..Attachment::default()
                            }],
                            ..Target::default()
                        },
                    ],
                    ..Session::default()
                },
                Session {
                    id: "n-2025".to_string(),
                    date: "2025-12-01".to_string(),
                    targets: vec![Target {
                        id: "m42b".to_string(),
                        name: "m 42".to_string(),
                        attachments: vec![Attachment {
                            stored_name: "b.jpg".to_string(),
                            original_name: "large.jpg".to_string(),
                            ..Attachment::default()
                        }],
                        ..Target::default()
                    }],
                    ..Session::default()
                },
                Session {
                    id: "bad-date".to_string(),
                    date: "mars 2026".to_string(),
                    ..Session::default()
                },
            ],
            ..Journal::default()
        }
    }

    #[test]
    fn object_query_ignores_case_and_spaces_and_keeps_only_images() {
        assert!(name_matches("M42", "M42"));
        assert!(name_matches("m 42", "M42"));
        assert!(name_matches("M42 — Nébuleuse d’Orion", "m 42"));
        assert!(!name_matches("M31", "M42"));

        let hits = photos_of(&sample(), "M42");
        let names: Vec<&str> = hits.iter().map(|hit| hit.original_name.as_str()).collect();
        assert_eq!(names, vec!["coeur.png", "large.jpg"]);
        assert!(
            hits.iter()
                .all(|hit| hit.target_name.to_lowercase().contains("42")
                    || hit.target_name.to_lowercase().contains("m"))
        );
        assert_eq!(hits[0].date, "2026-02-10");
        assert_eq!(hits[1].date, "2025-12-01");
    }

    #[test]
    fn year_query_counts_only_dated_nights_of_that_year() {
        let journal = sample();
        let nights = nights_in_year(&journal, 2026);
        assert_eq!(nights.len(), 1);
        assert_eq!(nights[0].date, "2026-02-10");
        assert_eq!(nights[0].site_name, "Cour");
        assert!(nights_in_year(&journal, 2024).is_empty());
        assert_eq!(parse_year("2026"), Some(2026));
        assert_eq!(parse_year("202"), None);
        assert_eq!(parse_year("2026 "), Some(2026));
    }
}
