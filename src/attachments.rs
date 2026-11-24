use std::path::Path;

use image::ImageDecoder;

use crate::model::{Attachment, new_id};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FileKind {
    Png,
    Jpeg,
    Pdf,
    Txt,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AttachError {
    Refused(&'static str),
    Io(String),
}

impl AttachError {
    pub fn message(&self) -> String {
        match self {
            Self::Refused(text) => (*text).to_string(),
            Self::Io(text) => format!("Impossible de copier le fichier ({text})."),
        }
    }
}

const FITS: &str = "Les fichiers FITS (.fit, .fits) ne sont pas acceptés. Seules les astrophotos PNG 8 bits sont admises.";
const TIFF: &str = "Les fichiers TIFF (.tif, .tiff) ne sont pas acceptés. Seules les astrophotos PNG 8 bits sont admises.";
const XISF: &str =
    "Les fichiers XISF ne sont pas acceptés. Seules les astrophotos PNG 8 bits sont admises.";
const PNG_DEPTH: &str = "Ce PNG n'est pas en 8 bits. Seuls les PNG 8 bits sont acceptés.";
const UNSUPPORTED: &str =
    "Type de fichier non accepté. Formats permis : PNG 8 bits, JPEG, PDF et TXT.";
const UNREADABLE: &str = "Ce PNG n'a pas pu être lu.";

pub fn validate(path: &Path) -> Result<FileKind, AttachError> {
    let extension = extension_of(path);
    match extension.as_str() {
        "fit" | "fits" => Err(AttachError::Refused(FITS)),
        "tif" | "tiff" => Err(AttachError::Refused(TIFF)),
        "xisf" => Err(AttachError::Refused(XISF)),
        "jpg" | "jpeg" => Ok(FileKind::Jpeg),
        "pdf" => Ok(FileKind::Pdf),
        "txt" => Ok(FileKind::Txt),
        "png" => {
            if png_is_8bit(path)? {
                Ok(FileKind::Png)
            } else {
                Err(AttachError::Refused(PNG_DEPTH))
            }
        }
        _ => Err(AttachError::Refused(UNSUPPORTED)),
    }
}

pub fn import_attachment(src: &Path, media_dir: &Path) -> Result<Attachment, AttachError> {
    let kind = validate(src)?;
    let id = new_id();
    let ext = match kind {
        FileKind::Png => "png",
        FileKind::Jpeg => "jpg",
        FileKind::Pdf => "pdf",
        FileKind::Txt => "txt",
    };
    std::fs::create_dir_all(media_dir).map_err(|err| AttachError::Io(err.to_string()))?;
    let stored_name = format!("{id}.{ext}");
    std::fs::copy(src, media_dir.join(&stored_name))
        .map_err(|err| AttachError::Io(err.to_string()))?;
    let original_name = src
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| stored_name.clone());
    Ok(Attachment {
        id,
        stored_name,
        original_name,
        caption: String::new(),
    })
}

pub fn is_image(stored_name: &str) -> bool {
    matches!(extension_str(stored_name).as_str(), "png" | "jpg" | "jpeg")
}

pub fn open_file(path: &Path) {
    let _ = open::that(path);
}

fn png_is_8bit(path: &Path) -> Result<bool, AttachError> {
    let reader = image::ImageReader::open(path)
        .map_err(|_| AttachError::Refused(UNREADABLE))?
        .with_guessed_format()
        .map_err(|_| AttachError::Refused(UNREADABLE))?;
    let decoder = reader
        .into_decoder()
        .map_err(|_| AttachError::Refused(UNREADABLE))?;
    Ok(matches!(
        decoder.color_type(),
        image::ColorType::L8
            | image::ColorType::La8
            | image::ColorType::Rgb8
            | image::ColorType::Rgba8
    ))
}

fn extension_of(path: &Path) -> String {
    path.extension()
        .map(|ext| ext.to_string_lossy().to_ascii_lowercase())
        .unwrap_or_default()
}

fn extension_str(name: &str) -> String {
    Path::new(name)
        .extension()
        .map(|ext| ext.to_string_lossy().to_ascii_lowercase())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{ExtendedColorType, ImageEncoder, codecs::png::PngEncoder};
    use std::fs::File;

    fn temp_dir() -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("dubastrolog-files-{}", new_id()));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn refuses_fits_tiff_xisf_and_unknown_extensions() {
        assert!(matches!(
            validate(Path::new("lumiere.fit")),
            Err(AttachError::Refused(_))
        ));
        assert!(matches!(
            validate(Path::new("lumiere.FITS")),
            Err(AttachError::Refused(_))
        ));
        assert!(matches!(
            validate(Path::new("lumiere.tiff")),
            Err(AttachError::Refused(_))
        ));
        assert!(matches!(
            validate(Path::new("lumiere.TIF")),
            Err(AttachError::Refused(_))
        ));
        assert!(matches!(
            validate(Path::new("lumiere.xisf")),
            Err(AttachError::Refused(_))
        ));
        assert!(matches!(
            validate(Path::new("notes.docx")),
            Err(AttachError::Refused(_))
        ));
        assert_eq!(validate(Path::new("carte.pdf")).unwrap(), FileKind::Pdf);
        assert_eq!(validate(Path::new("notes.TXT")).unwrap(), FileKind::Txt);
        assert_eq!(validate(Path::new("aperçu.JPEG")).unwrap(), FileKind::Jpeg);
    }

    #[test]
    fn accepts_png8_and_refuses_png16() {
        let dir = temp_dir();
        let png8 = dir.join("huit.png");
        image::RgbImage::from_pixel(2, 2, image::Rgb([10, 20, 30]))
            .save(&png8)
            .unwrap();
        assert_eq!(validate(&png8).unwrap(), FileKind::Png);

        let png16 = dir.join("seize.png");
        let file = File::create(&png16).unwrap();
        PngEncoder::new(file)
            .write_image(&[0, 10], 1, 1, ExtendedColorType::L16)
            .unwrap();
        assert_eq!(validate(&png16), Err(AttachError::Refused(PNG_DEPTH)));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn copies_an_accepted_png_into_the_media_folder() {
        let dir = temp_dir();
        let src = dir.join("m42.png");
        image::RgbImage::from_pixel(2, 2, image::Rgb([1, 2, 3]))
            .save(&src)
            .unwrap();
        let media = dir.join("media");
        let stored = import_attachment(&src, &media).unwrap();
        assert!(media.join(&stored.stored_name).is_file());
        assert_eq!(stored.original_name, "m42.png");
        assert!(is_image(&stored.stored_name));
        assert!(!is_image("note.pdf"));

        let fits = dir.join("brute.fits");
        std::fs::write(&fits, b"not a fits").unwrap();
        assert!(import_attachment(&fits, &media).is_err());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
