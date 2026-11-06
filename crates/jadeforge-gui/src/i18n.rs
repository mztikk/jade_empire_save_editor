//! The interface language catalog.
//!
//! Static user-facing strings are wrapped in [`t`] at their call site. The
//! English catalog maps each source string to itself; the QA pseudolocale maps
//! it to a bracketed, accented variant that exposes clipping and untranslated
//! constants. A real translation adds one catalog row per language; until a
//! reviewed translation exists the source string is shown unchanged, which is
//! why no invented translation ships with the application.
//!
//! The active locale is thread local, exactly like the theme: the root installs
//! it before rendering and the headless suite can render several languages
//! without leaking one test's choice into another.

use std::cell::Cell;

use crate::Locale;

thread_local! {
    static CURRENT: Cell<Locale> = const { Cell::new(Locale::English) };
}

/// Installs the locale for the following render pass.
pub fn install(locale: Locale) {
    CURRENT.with(|current| current.set(locale));
}

/// The active locale.
#[must_use]
pub fn current() -> Locale {
    CURRENT.with(Cell::get)
}

/// Translates a static interface string.
///
/// Unknown strings are returned unchanged. That is the deliberate fallback for
/// dynamic values (a character name or a path) and for a locale that has not
/// translated a newly added key yet.
#[must_use]
pub fn t(text: &'static str) -> &'static str {
    let locale = current();
    if locale == Locale::English {
        return text;
    }
    CATALOG
        .iter()
        .find(|(source, _pseudo)| *source == text)
        .map_or(text, |(_, translated)| *translated)
}

/// Source string to QA pseudolocale translation.
///
/// Every `t("...")` in `src` must have a row here; the test in this module
/// parses the source tree and fails on a missing key.
const CATALOG: &[(&str, &str)] = &[
    (
        "A new style is stored with the same field layout as one the player already owns; the game fills in the remaining data from its own tables.",
        "⟦Á ñéw stylé ís stóréd wíth thé sámé fíéld láyóút ás óñé thé pláyér álréády ówñs; thé gámé fílls íñ thé rémáíñíñg dátá fróm íts ówñ táblés.⟧",
    ),
    (
        "A timestamped backup is created next to the savegame on every save and shown here.",
        "⟦Á tíméstámpéd báçkúp ís çréátéd ñéxt tó thé sávégámé óñ évéry sávé áñd shówñ héré.⟧",
    ),
    ("ADD AN ITEM", "⟦ÁDD ÁÑ ÍTÉM⟧"),
    ("APPEARANCE", "⟦ÁPPÉÁRÁÑÇÉ⟧"),
    ("About", "⟦Ábóút⟧"),
    ("About JadeForge", "⟦Ábóút JádéFórgé⟧"),
    (
        "About JadeForge, version, settings and backups",
        "⟦Ábóút JádéFórgé, vérsíóñ, séttíñgs áñd báçkúps⟧",
    ),
    ("Add a gem", "⟦Ádd á gém⟧"),
    ("Amulet", "⟦Ámúlét⟧"),
    ("At a glance", "⟦Át á gláñçé⟧"),
    (
        "Attribute points you can still spend; the field is created when the savegame does not have it.",
        "⟦Áttríbúté póíñts yóú çáñ stíll spéñd; thé fíéld ís çréátéd whéñ thé sávégámé dóés ñót hávé ít.⟧",
    ),
    ("Attributes", "⟦Áttríbútés⟧"),
    ("BACKUPS", "⟦BÁÇKÚPS⟧"),
    ("Back to editing", "⟦Báçk tó édítíñg⟧"),
    ("Backup", "⟦Báçkúp⟧"),
    ("Backups", "⟦Báçkúps⟧"),
    (
        "Browse the backups of the open savegame",
        "⟦Brówsé thé báçkúps óf thé ópéñ sávégámé⟧",
    ),
    ("Browse…", "⟦Brówsé…⟧"),
    (
        "Bundled English names are loaded, but new styles and upgrade details still need the game data. Levels, weapons and time spent come from the savegame.",
        "⟦Búñdléd Éñglísh ñámés áré lóádéd, bút ñéw stylés áñd úpgrádé détáíls stíll ñééd thé gámé dátá. Lévéls, wéápóñs áñd tímé spéñt çómé fróm thé sávégámé.⟧",
    ),
    ("CATEGORY", "⟦ÇÁTÉGÓRY⟧"),
    ("CHI", "⟦ÇHÍ⟧"),
    ("Cancel scan", "⟦Çáñçél sçáñ⟧"),
    ("Character", "⟦Çháráçtér⟧"),
    ("Choose save folder", "⟦Çhóósé sávé fóldér⟧"),
    ("Clear", "⟦Çléár⟧"),
    ("Clear the gem filter", "⟦Çléár thé gém fíltér⟧"),
    (
        "Clear the item database filter",
        "⟦Çléár thé ítém dátábásé fíltér⟧",
    ),
    ("Clear the savegame filter", "⟦Çléár thé sávégámé fíltér⟧"),
    ("Clear the style filter", "⟦Çléár thé stylé fíltér⟧"),
    ("Close", "⟦Çlósé⟧"),
    ("Close the About page", "⟦Çlósé thé Ábóút págé⟧"),
    ("Close the backup browser", "⟦Çlósé thé báçkúp brówsér⟧"),
    (
        "Companion levels follow the player through the game; raising them here only changes what the companion starts with.",
        "⟦Çómpáñíóñ lévéls fóllów thé pláyér thróúgh thé gámé; ráísíñg thém héré óñly çháñgés whát thé çómpáñíóñ stárts wíth.⟧",
    ),
    ("Companions", "⟦Çómpáñíóñs⟧"),
    ("Contents", "⟦Çóñtéñts⟧"),
    ("Copy", "⟦Çópy⟧"),
    ("Copy the status message", "⟦Çópy thé státús mésságé⟧"),
    ("Current sheet", "⟦Çúrréñt shéét⟧"),
    (
        "Current values may stay below the maximum; the game fills them on rest.",
        "⟦Çúrréñt válúés máy stáy bélów thé máxímúm; thé gámé fílls thém óñ rést.⟧",
    ),
    (
        "Every save writes a copy of the previous save.sac next to it, named save.sac.jadeforge-backup-<seconds>.<nanos>.<counter>. To undo a save, close JadeForge and copy the backup over save.sac. An earlier backup is never overwritten.",
        "⟦Évéry sávé wrítés á çópy óf thé prévíóús sávé.sáç ñéxt tó ít, ñáméd sávé.sáç.jádéfórgé-báçkúp-<séçóñds>.<ñáñós>.<çóúñtér>. Tó úñdó á sávé, çlósé JádéFórgé áñd çópy thé báçkúp óvér sávé.sáç. Áñ éárlíér báçkúp ís ñévér óvérwríttéñ.⟧",
    ),
    ("FOCUS", "⟦FÓÇÚS⟧"),
    ("Fighting styles", "⟦Fíghtíñg stylés⟧"),
    ("File", "⟦Fílé⟧"),
    (
        "Filled gold orbs are owned; an empty ring is available. Clicking tier N buys every missing tier up to N; clicking an owned tier removes it and everything above it.",
        "⟦Fílléd góld órbs áré ówñéd; áñ émpty ríñg ís áváíláblé. Çlíçkíñg tíér Ñ búys évéry míssíñg tíér úp tó Ñ; çlíçkíñg áñ ówñéd tíér rémóvés ít áñd évérythíñg ábóvé ít.⟧",
    ),
    ("GAME DATA", "⟦GÁMÉ DÁTÁ⟧"),
    ("GOLD", "⟦GÓLD⟧"),
    ("Game data", "⟦Gámé dátá⟧"),
    ("Gems", "⟦Géms⟧"),
    (
        "Gold is optional in the savegame: leave it empty to keep the field absent, or enter a value to create it when you save.",
        "⟦Góld ís óptíóñál íñ thé sávégámé: léávé ít émpty tó kéép thé fíéld ábséñt, ór éñtér á válúé tó çréáté ít whéñ yóú sávé.⟧",
    ),
    ("HEALTH", "⟦HÉÁLTH⟧"),
    ("Identity", "⟦Ídéñtíty⟧"),
    ("Inventory", "⟦Íñvéñtóry⟧"),
    ("Jade Empire save editor", "⟦Jádé Émpíré sávé édítór⟧"),
    (
        "Jade Empire savegame editor",
        "⟦Jádé Émpíré sávégámé édítór⟧",
    ),
    (
        "JadeForge is reading the save folder and the game data. The save list appears as soon as the scan finishes.",
        "⟦JádéFórgé ís réádíñg thé sávé fóldér áñd thé gámé dátá. Thé sávé líst áppéárs ás sóóñ ás thé sçáñ fíñíshés.⟧",
    ),
    (
        "JadeForge never writes a savegame without verifying it and keeping a backup.",
        "⟦JádéFórgé ñévér wrítés á sávégámé wíthóút vérífyíñg ít áñd kéépíñg á báçkúp.⟧",
    ),
    (
        "JadeForge reads and writes save.sac files directly. Every write is verified by re-reading the produced file and is written atomically, and the game installation is only ever read.",
        "⟦JádéFórgé réáds áñd wrítés sávé.sáç fílés díréçtly. Évéry wríté ís vérífíéd by ré-réádíñg thé pródúçéd fílé áñd ís wríttéñ átómíçálly, áñd thé gámé íñstállátíóñ ís óñly évér réád.⟧",
    ),
    (
        "JadeForge verifies every savegame by re-reading it before writing, writes it atomically and keeps a timestamped backup next to the file.",
        "⟦JádéFórgé vérífíés évéry sávégámé by ré-réádíñg ít béfóré wrítíñg, wrítés ít átómíçálly áñd kééps á tíméstámpéd báçkúp ñéxt tó thé fílé.⟧",
    ),
    ("KEYBOARD SHORTCUTS", "⟦KÉYBÓÁRD SHÓRTÇÚTS⟧"),
    ("LEARN A STYLE", "⟦LÉÁRÑ Á STYLÉ⟧"),
    ("Level", "⟦Lévél⟧"),
    ("Loading savegames…", "⟦Lóádíñg sávégámés…⟧"),
    (
        "Moving a gem keeps its effects; JadeForge only changes which list references it.",
        "⟦Móvíñg á gém kééps íts éfféçts; JádéFórgé óñly çháñgés whíçh líst référéñçés ít.⟧",
    ),
    ("Name", "⟦Ñámé⟧"),
    (
        "No backup has been written in this session; saving a change creates one and shows its exact path on the Overview tab.",
        "⟦Ñó báçkúp hás bééñ wríttéñ íñ thís séssíóñ; sávíñg á çháñgé çréátés óñé áñd shóws íts éxáçt páth óñ thé Óvérvíéw táb.⟧",
    ),
    (
        "No backups were found next to this savegame. The first successful save creates one.",
        "⟦Ñó báçkúps wéré fóúñd ñéxt tó thís sávégámé. Thé fírst súççéssfúl sávé çréátés óñé.⟧",
    ),
    (
        "No config directory is available in this environment. JADEFORGE_CONFIG_DIR can point at one for portable installs.",
        "⟦Ñó çóñfíg díréçtóry ís áváíláblé íñ thís éñvíróñméñt. JÁDÉFÓRGÉ_ÇÓÑFÍG_DÍR çáñ póíñt át óñé fór pórtáblé íñstálls.⟧",
    ),
    (
        "No gems carried in the inventory.",
        "⟦Ñó géms çárríéd íñ thé íñvéñtóry.⟧",
    ),
    (
        "No gems socketed into the amulet.",
        "⟦Ñó géms sóçkétéd íñtó thé ámúlét.⟧",
    ),
    (
        "No save folder has been chosen yet. Use Browse… or Ctrl+O to pick one.",
        "⟦Ñó sávé fóldér hás bééñ çhóséñ yét. Úsé Brówsé… ór Çtrl+Ó tó píçk óñé.⟧",
    ),
    ("No savegame selected", "⟦Ñó sávégámé séléçtéd⟧"),
    (
        "Nothing is written until the write button below is pressed. The file is verified by re-reading it, written atomically, and the previous file is kept as a backup. Invalid character fields are not applied; the other edits are.",
        "⟦Ñóthíñg ís wríttéñ úñtíl thé wríté búttóñ bélów ís présséd. Thé fílé ís vérífíéd by ré-réádíñg ít, wríttéñ átómíçálly, áñd thé prévíóús fílé ís képt ás á báçkúp. Íñválíd çháráçtér fíélds áré ñót ápplíéd; thé óthér édíts áré.⟧",
    ),
    ("Overview", "⟦Óvérvíéw⟧"),
    (
        "Path to the Jade Empire Save folder",
        "⟦Páth tó thé Jádé Émpíré Sávé fóldér⟧",
    ),
    (
        "Path to the Jade Empire installation (with dialog.tlk)",
        "⟦Páth tó thé Jádé Émpíré íñstállátíóñ (wíth díálóg.tlk)⟧",
    ),
    (
        "Reading the game data; existing items can already be edited and removed.",
        "⟦Réádíñg thé gámé dátá; éxístíñg ítéms çáñ álréády bé édítéd áñd rémóvéd.⟧",
    ),
    (
        "Reading the game data; learned styles can already be inspected and removed.",
        "⟦Réádíñg thé gámé dátá; léárñéd stylés çáñ álréády bé íñspéçtéd áñd rémóvéd.⟧",
    ),
    ("Redo", "⟦Rédó⟧"),
    ("Reload from disk", "⟦Rélóád fróm dísk⟧"),
    ("Restore", "⟦Réstóré⟧"),
    ("Revert", "⟦Révért⟧"),
    ("Review changes", "⟦Révíéw çháñgés⟧"),
    ("SAVE FOLDER", "⟦SÁVÉ FÓLDÉR⟧"),
    ("SETTINGS", "⟦SÉTTÍÑGS⟧"),
    (
        "Save or revert the unsaved changes before restoring a backup: a restore would otherwise silently discard them.",
        "⟦Sávé ór révért thé úñsávéd çháñgés béfóré réstóríñg á báçkúp: á réstóré wóúld óthérwísé síléñtly dísçárd thém.⟧",
    ),
    ("Save to disk", "⟦Sávé tó dísk⟧"),
    ("Savegame", "⟦Sávégámé⟧"),
    ("Sort order", "⟦Sórt órdér⟧"),
    ("Styles", "⟦Stylés⟧"),
    ("TEXT SIZE", "⟦TÉXT SÍZÉ⟧"),
    ("THEME", "⟦THÉMÉ⟧"),
    (
        "The amulet is unlocked early in the story; it has no sockets before that.",
        "⟦Thé ámúlét ís úñlóçkéd éárly íñ thé stóry; ít hás ñó sóçkéts béfóré thát.⟧",
    ),
    (
        "The game data was not found, so items can only be removed, not added. The category comes from the item type stored in the savegame.",
        "⟦Thé gámé dátá wás ñót fóúñd, só ítéms çáñ óñly bé rémóvéd, ñót áddéd. Thé çátégóry çómés fróm thé ítém typé stóréd íñ thé sávégámé.⟧",
    ),
    (
        "The game data was not found, so style names and new styles are unavailable. Levels, weapons and time spent come from the savegame.",
        "⟦Thé gámé dátá wás ñót fóúñd, só stylé ñámés áñd ñéw stylés áré úñáváíláblé. Lévéls, wéápóñs áñd tímé spéñt çómé fróm thé sávégámé.⟧",
    ),
    ("The inventory is empty.", "⟦Thé íñvéñtóry ís émpty.⟧"),
    (
        "The player has not learned any fighting styles.",
        "⟦Thé pláyér hás ñót léárñéd áñy fíghtíñg stylés.⟧",
    ),
    (
        "The theme and text size apply immediately and are remembered in the settings file. System follows the operating system preference; high contrast can be selected here because the windowing layer does not report the platform's forced-colours setting.",
        "⟦Thé thémé áñd téxt sízé ápply ímmédíátély áñd áré rémémbéréd íñ thé séttíñgs fílé. Systém fóllóws thé ópérátíñg systém préféréñçé; hígh çóñtrást çáñ bé séléçtéd héré béçáúsé thé wíñdówíñg láyér dóés ñót répórt thé plátfórm's fórçéd-çólóúrs séttíñg.⟧",
    ),
    (
        "These preferences live in one TOML file. They are remembered between runs and can be edited while JadeForge is closed.",
        "⟦Thésé préféréñçés lívé íñ óñé TÓML fílé. Théy áré rémémbéréd bétwééñ rúñs áñd çáñ bé édítéd whílé JádéFórgé ís çlóséd.⟧",
    ),
    (
        "This is the file as it was before the last save. To restore it, close JadeForge and copy the backup over save.sac.",
        "⟦Thís ís thé fílé ás ít wás béfóré thé lást sávé. Tó réstóré ít, çlósé JádéFórgé áñd çópy thé báçkúp óvér sávé.sáç.⟧",
    ),
    (
        "This savegame has no companions yet. They appear here once the story introduces them.",
        "⟦Thís sávégámé hás ñó çómpáñíóñs yét. Théy áppéár héré óñçé thé stóry íñtródúçés thém.⟧",
    ),
    ("Time played", "⟦Tímé pláyéd⟧"),
    ("UPGRADES", "⟦ÚPGRÁDÉS⟧"),
    ("Undo", "⟦Úñdó⟧"),
    ("Use", "⟦Úsé⟧"),
    ("Vitals", "⟦Vítáls⟧"),
    ("Write savegame", "⟦Wríté sávégámé⟧"),
    ("all changes saved", "⟦áll çháñgés sávéd⟧"),
    ("available", "⟦áváíláblé⟧"),
    ("hidden", "⟦híddéñ⟧"),
    ("no savegame open", "⟦ñó sávégámé ópéñ⟧"),
    ("not set", "⟦ñót sét⟧"),
    ("unsaved changes", "⟦úñsávéd çháñgés⟧"),
    ("LANGUAGE", "⟦LÁNGÚÁGÉ⟧"),
    ("WINDOW", "⟦WÍNDÓW⟧"),
    ("Remember size and position", "⟦Rémémbér sízé ánd pósítíón⟧"),
    (
        "Remember the window size and position between runs",
        "⟦Rémémbér thé wíndów sízé ánd pósítíón bétwéén rúns⟧",
    ),
    ("DIAGNOSTICS", "⟦DÍÁGNÓSTÍÇS⟧"),
    ("LICENSES", "⟦LÍÇÉNSÉS⟧"),
    ("Copy diagnostics", "⟦Çópy díágnóstíçs⟧"),
    (
        "Copy the diagnostics to the clipboard",
        "⟦Çópy thé díágnóstíçs tó thé çlípbóárd⟧",
    ),
];

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::{Path, PathBuf};

    fn source_files(directory: &Path, files: &mut Vec<PathBuf>) {
        let Ok(entries) = std::fs::read_dir(directory) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                source_files(&path, files);
            } else if path.extension().is_some_and(|extension| extension == "rs") {
                files.push(path);
            }
        }
    }

    /// Extracts the first argument of every `t("...")` call in a source file.
    ///
    /// The parser only needs plain single-line literals, which is what the UI
    /// uses; an escaped quote is handled so the scan cannot stop early.
    fn decode_rust(raw: &str) -> String {
        let mut out = String::new();
        let mut chars = raw.chars().peekable();
        while let Some(character) = chars.next() {
            if character != '\\' {
                out.push(character);
                continue;
            }
            match chars.next() {
                Some('\n') => {
                    while chars.peek().is_some_and(|next| next.is_whitespace()) {
                        chars.next();
                    }
                }
                Some('n') => out.push('\n'),
                Some('t') => out.push('\t'),
                Some('r') => out.push('\r'),
                Some('"') => out.push('"'),
                Some('\\') => out.push('\\'),
                Some(other) => out.push(other),
                None => {}
            }
        }
        out
    }

    fn translated_literals(source: &str) -> Vec<String> {
        let bytes = source.as_bytes();
        let mut literals = Vec::new();
        let mut index = 0;
        while let Some(found) = source.get(index..).and_then(|rest| rest.find("t(\"")) {
            let at = index + found;
            if at > 0 {
                let previous = bytes[at - 1];
                if previous.is_ascii_alphanumeric() || previous == b'_' {
                    index = at + 2;
                    continue;
                }
            }
            let start = at + 3;
            let mut position = start;
            let mut escaped = false;
            while position < bytes.len() {
                if escaped {
                    escaped = false;
                } else {
                    match bytes[position] {
                        b'\\' => escaped = true,
                        b'"' => break,
                        _ => {}
                    }
                }
                position += 1;
            }
            if position < bytes.len() {
                let literal = source
                    .get(start..position)
                    .map(decode_rust)
                    .unwrap_or_default();
                if !literal.is_empty() {
                    literals.push(literal);
                }
            }
            index = position.max(index + 1);
        }
        literals
    }

    #[test]
    fn every_translated_literal_is_in_the_catalog() {
        let mut files = Vec::new();
        source_files(
            Path::new(env!("CARGO_MANIFEST_DIR")).join("src").as_path(),
            &mut files,
        );
        let mut missing = Vec::new();
        for file in files {
            if file.file_name().is_some_and(|name| name == "i18n.rs") {
                continue;
            }
            let source = std::fs::read_to_string(&file).expect("read source");
            for literal in translated_literals(&source) {
                if !CATALOG.iter().any(|(source, _)| *source == literal) {
                    missing.push(format!("{}: {literal}", file.display()));
                }
            }
        }
        assert!(
            missing.is_empty(),
            "missing catalog entries:\n{}",
            missing.join("\n")
        );
    }

    #[test]
    fn every_catalogued_string_has_a_pseudolocale_translation() {
        for (source, pseudo) in CATALOG {
            assert_ne!(*source, "");
            assert_ne!(source, pseudo, "a pseudo entry must be visibly different");
        }
    }

    #[test]
    fn unknown_strings_fall_back_to_the_source() {
        install(Locale::Pseudo);
        assert_eq!(
            t("A string that is not catalogued"),
            "A string that is not catalogued"
        );
        install(Locale::English);
    }
}
