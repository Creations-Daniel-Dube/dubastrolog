# dubastrolog

**Version : 0.1.100**

---

## Français

### À quoi sert ce logiciel ?

**dubastrolog** est un journal personnel pour l’astronomie amateur. Il sert à consigner, soir après soir, ce que vous avez observé ou photographié sous le ciel : lieu, conditions, instrument, objets visés et images gardées en souvenir.

### Fonctions disponibles

- **Soirées d’observation** — Créer et consulter des soirées (date, heures de début et de fin, site, instrument principal, notes).
- **Météo de la soirée** — Noter température, humidité, vent, nuages, seeing, transparence, luminosité du ciel, phase lunaire et remarques.
- **Objets observés** — Pour chaque soirée, ajouter des objets (ciel profond, planètes, Lune, Soleil, comètes, étoiles, etc.), indiquer s’il s’agit d’observation visuelle, photo ou les deux, et consigner filtres, poses, gain ou ISO, nombre d’images, oculaire et commentaires.
- **Instruments par objet** — Utiliser l’instrument de la soirée ou en choisir un autre (télescope, caméra, télescope intelligent) pour un objet précis.
- **Photos et documents** — Joindre des images ou fichiers à un objet, avec légende et aperçu ; ouvrir un fichier joint dans une application externe.
- **Liste des soirées** — Parcourir l’historique et filtrer la liste par texte (date, site, notes, noms d’objets).
- **Recherche** — Retrouver les photos liées à un nom d’objet, et lister les soirées d’une année donnée.
- **Profils réutilisables** — Gérer des sites d’observation, télescopes, caméras astro, appareils reflex, objectifs (fixe ou zoom) et télescopes intelligents, pour les réutiliser d’une soirée à l’autre ; supprimer un profil inutilisé (avec confirmation).
- **Soirée au reflex** — Choisir boîtier et objectif pour une soirée ; pour un zoom, indiquer la focale utilisée sur chaque objet photographié.
- **Suppression** — Supprimer une soirée (avec confirmation) ainsi que les pièces jointes qui lui sont associées.

### Données et distribution

- **Hors de l’exécutable** — Profils, soirées et pièces jointes ne sont pas inclus dans `dubastrolog.exe`. Chaque utilisateur a son propre journal sur son poste.
- **Emplacement (Windows)** — Fichier `journal.json` et dossier `media/` sous `%APPDATA%\dubastrolog` (barre d’état en bas de la fenêtre : chemin complet du journal).
- **Première installation** — Si `journal.json` n’existe pas encore, l’application démarre avec des profils et des soirées vides. Les données sont créées lors du premier enregistrement.
- **Mise à jour** — Distribuer uniquement le nouveau `.exe` : le journal existant est conservé tel quel.
- **À ne pas inclure dans un zip** — Ne joignez pas de copie de `journal.json`, du dossier `media/` ni d’un dossier `dubastrolog/` de développement.

**Développeur** — Tester une « première installation » sur votre propre compte Windows ne suffit pas : le programme relira votre journal habituel dans AppData. Utilisez un autre compte Windows, une autre machine, ou renommez temporairement `%APPDATA%\dubastrolog` avant le test.

---

## English

### What is this software for?

**dubastrolog** is a personal logbook for amateur astronomy. It helps you record, night after night, what you observed or imaged under the sky: location, conditions, equipment, targets, and pictures you want to keep.

### Available features

- **Observation nights** — Create and browse nights (date, start and end times, site, main instrument, notes).
- **Night weather** — Record temperature, humidity, wind, clouds, seeing, transparency, sky brightness, Moon illumination, and comments.
- **Observed targets** — For each night, add targets (deep sky, planets, Moon, Sun, comets, stars, and more), note visual, photographic, or both modes, and log filters, exposure, gain or ISO, frame count, eyepiece, and notes.
- **Per-target equipment** — Use the night’s default setup or pick a different telescope, camera, or smart telescope for a specific target.
- **Photos and documents** — Attach images or files to a target, with captions and previews; open an attachment in an external app.
- **Night list** — Browse your history and filter nights by text (date, site, notes, target names).
- **Search** — Find photos tied to a target name, and list all nights in a given year.
- **Reusable profiles** — Manage observing sites, telescopes, astro cameras, DSLR bodies, lenses (prime or zoom), and smart telescopes for reuse across nights; remove an unused profile (with confirmation).
- **DSLR nights** — Pick body and lens for a night; for a zoom lens, record the focal length used on each photographed target.
- **Deletion** — Remove a night (with confirmation) and its associated attachments.

### Data and distribution

- **Not inside the executable** — Profiles, nights, and attachments are not bundled in `dubastrolog.exe`. Each user keeps a separate journal on their machine.
- **Location (Windows)** — `journal.json` and the `media/` folder live under `%APPDATA%\dubastrolog` (status bar at the bottom of the window shows the full journal path).
- **First install** — If `journal.json` does not exist yet, the app starts with empty profiles and nights. Data is created on first save.
- **Upgrade** — Ship only the new `.exe`; the existing journal is preserved.
- **Do not zip with the app** — Do not attach a copy of `journal.json`, a `media/` folder, or a development `dubastrolog/` directory.

**Developer note** — Testing a “fresh install” on your own Windows account is not enough: the app will reload your usual AppData journal. Use another Windows account, another machine, or temporarily rename `%APPDATA%\dubastrolog` before testing.