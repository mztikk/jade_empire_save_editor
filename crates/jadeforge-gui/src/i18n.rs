//! The interface language catalog.
//!
//! Static user-facing strings are wrapped in [`t`] at their call site. English
//! is the source language; German is bundled as `locales/de.toml`, and the QA
//! pseudolocale maps every catalogued string to a bracketed, accented variant
//! that exposes clipping and untranslated constants. Empty or missing German
//! entries fall back to the source, so the catalog can be filled in stages
//! without ever showing an invented string.
//!
//! The active locale is thread local, exactly like the theme: the root installs
//! it before rendering and the headless suite can render several languages
//! without leaking one test's choice into another.

use std::cell::Cell;
use std::collections::HashMap;
use std::sync::OnceLock;

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
    match current() {
        Locale::English => text,
        Locale::Pseudo => pseudo(text),
        Locale::German => match german().get(text).map(String::as_str) {
            Some(translated) if !translated.is_empty() => translated,
            _ => text,
        },
    }
}

/// Fills `{name}` placeholders in a translated template.
///
/// The template must be catalogued; unknown templates fall back to the source
/// text. Values are inserted verbatim, so a path or a character name is never
/// translated.
#[must_use]
pub fn fill(template: &'static str, arguments: &[(&str, &str)]) -> String {
    let mut text = t(template).to_owned();
    for (name, value) in arguments {
        text = text.replace(&format!("{{{name}}}"), value);
    }
    text
}

/// A count followed by the singular or plural catalogued noun.
///
/// English and German both use the one/other distinction, which is why the
/// catalog stores two source strings for each noun pair. A language with more
/// plural forms needs a richer helper before it can be added.
#[must_use]
pub fn plural(count: usize, singular: &'static str, plural: &'static str) -> String {
    let noun = if count == 1 { t(singular) } else { t(plural) };
    format!("{count} {noun}")
}

fn pseudo(text: &'static str) -> &'static str {
    CATALOG
        .iter()
        .find(|(source, _pseudo)| *source == text)
        .map_or(text, |(_, translated)| *translated)
}

/// German catalog, parsed once from the bundled TOML.
///
/// A malformed file degrades to English rather than failing to start; the test
/// below parses it and checks that every source key exists, and the opt-in
/// completeness test reports untranslated values.
static GERMAN: OnceLock<HashMap<String, String>> = OnceLock::new();

fn german() -> &'static HashMap<String, String> {
    GERMAN.get_or_init(|| {
        toml::from_str::<HashMap<String, String>>(include_str!("../locales/de.toml"))
            .unwrap_or_default()
    })
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
    ("At a glance", "⟦Át á gláñçé⟧"),
    (
        "Attribute points you can still spend; the field is created when the savegame does not have it.",
        "⟦Áttríbúté póíñts yóú çáñ stíll spéñd; thé fíéld ís çréátéd whéñ thé sávégámé dóés ñót hávé ít.⟧",
    ),
    ("Attributes", "⟦Áttríbútés⟧"),
    ("Back to editing", "⟦Báçk tó édítíñg⟧"),
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
    ("GOLD", "⟦GÓLD⟧"),
    ("Game data", "⟦Gámé dátá⟧"),
    (
        "Gold is optional in the savegame: leave it empty to keep the field absent, or enter a value to create it when you save.",
        "⟦Góld ís óptíóñál íñ thé sávégámé: léávé ít émpty tó kéép thé fíéld ábséñt, ór éñtér á válúé tó çréáté ít whéñ yóú sávé.⟧",
    ),
    ("HEALTH", "⟦HÉÁLTH⟧"),
    ("Identity", "⟦Ídéñtíty⟧"),
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
    ("SETTINGS", "⟦SÉTTÍÑGS⟧"),
    (
        "Save or revert the unsaved changes before restoring a backup: a restore would otherwise silently discard them.",
        "⟦Sávé ór révért thé úñsávéd çháñgés béfóré réstóríñg á báçkúp: á réstóré wóúld óthérwísé síléñtly dísçárd thém.⟧",
    ),
    ("Save to disk", "⟦Sávé tó dísk⟧"),
    ("Sort order", "⟦Sórt órdér⟧"),
    ("TEXT SIZE", "⟦TÉXT SÍZÉ⟧"),
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
    ("WINDOW", "⟦WÍNDÓW⟧"),
    ("Remember size and position", "⟦Rémémbér sízé ánd pósítíón⟧"),
    (
        "Remember the window size and position between runs",
        "⟦Rémémbér thé wíndów sízé ánd pósítíón bétwéén rúns⟧",
    ),
    (
        "Remember the window size between runs; the compositor places the window",
        "⟦Rémémbér thé wíndów sízé bétwéén rúns; thé çómpósítór pláçés thé wíndów⟧",
    ),
    (
        "Wayland compositors place windows themselves. Add a window rule in your desktop settings to pin the position.",
        "⟦Wáylánd çómpósítórs pláçé wíndóws thémšélvés. Ádd á wíndów rúlé íñ yóúr désktóp séttíñgs tó píñ thé pósítíón.⟧",
    ),
    ("DIAGNOSTICS", "⟦DÍÁGNÓSTÍÇS⟧"),
    ("LICENSES", "⟦LÍÇÉNSÉS⟧"),
    ("Copy diagnostics", "⟦Çópy díágnóstíçs⟧"),
    (
        "Copy the diagnostics to the clipboard",
        "⟦Çópy thé díágnóstíçs tó thé çlípbóárd⟧",
    ),
    (
        "This savegame has unsaved changes: save them or press Revert before choosing another folder.",
        "⟦Thís sávégámé hás úñsávéd çháñgés: sávé thém ór préss Révért béfóré çhóósíñg áñóthér fóldér.⟧",
    ),
    (
        "No folder was chosen; the save folder on screen is unchanged.",
        "⟦Ñó fóldér wás çhóséñ; thé sávé fóldér óñ sçrééñ ís úñçháñgéd.⟧",
    ),
    (
        "Copied the status message to the clipboard.",
        "⟦Çópíéd thé státús mésságé tó thé çlípbóárd.⟧",
    ),
    (
        "Could not copy the status message: the system clipboard is not available.",
        "⟦Çóúld ñót çópy thé státús mésságé: thé systém çlípbóárd ís ñót áváíláblé.⟧",
    ),
    ("loading game data…", "⟦lóádíñg gámé dátá…⟧"),
    ("English names loaded", "⟦Éñglísh ñámés lóádéd⟧"),
    ("game data not found", "⟦gámé dátá ñót fóúñd⟧"),
    ("game data unreadable", "⟦gámé dátá úñréádáblé⟧"),
    (
        "No folder was chosen; the game data folder is unchanged.",
        "⟦Ñó fóldér wás çhóséñ; thé gámé dátá fóldér ís úñçháñgéd.⟧",
    ),
    ("Scanning…", "⟦Sçáññíñg…⟧"),
    ("Scan", "⟦Sçáñ⟧"),
    ("no screenshot", "⟦ñó sçrééñshót⟧"),
    ("select a save", "⟦séléçt á sávé⟧"),
    ("Discard changes?", "⟦Dísçárd çháñgés?⟧"),
    (
        "Reload the savegame from disk and discard every change that is only in memory?",
        "⟦Rélóád thé sávégámé fróm dísk áñd dísçárd évéry çháñgé thát ís óñly íñ mémóry?⟧",
    ),
    (
        "Open a savegame to edit it.",
        "⟦Ópéñ á sávégámé tó édít ít.⟧",
    ),
    (
        "Some character fields are invalid and were not applied; your other edits can still be saved.",
        "⟦Sómé çháráçtér fíélds áré íñválíd áñd wéré ñót ápplíéd; yóúr óthér édíts çáñ stíll bé sávéd.⟧",
    ),
    (
        "Some character fields are invalid and were not applied; fix the highlighted fields to apply them.",
        "⟦Sómé çháráçtér fíélds áré íñválíd áñd wéré ñót ápplíéd; fíx thé híghlíghtéd fíélds tó ápply thém.⟧",
    ),
    (
        "Changes are only in memory until you save.",
        "⟦Çháñgés áré óñly íñ mémóry úñtíl yóú sávé.⟧",
    ),
    (
        "All changes are saved. There is nothing to write.",
        "⟦Áll çháñgés áré sávéd. Théré ís ñóthíñg tó wríté.⟧",
    ),
    (
        "Point JadeForge at the folder that contains your Jade Empire savegames and press Scan.",
        "⟦Póíñt JádéFórgé át thé fóldér thát çóñtáíñs yóúr Jádé Émpíré sávégámés áñd préss Sçáñ.⟧",
    ),
    (
        "Pick a savegame on the left to start editing.",
        "⟦Píçk á sávégámé óñ thé léft tó stárt édítíñg.⟧",
    ),
    (
        "This savegame changed on disk since it was opened. Saving will be refused; reloading discards the unsaved edits on screen and shows the file that is on disk.",
        "⟦Thís sávégámé çháñgéd óñ dísk síñçé ít wás ópéñéd. Sávíñg wíll bé réfúséd; rélóádíñg dísçárds thé úñsávéd édíts óñ sçrééñ áñd shóws thé fílé thát ís óñ dísk.⟧",
    ),
    (
        "This savegame changed on disk since it was opened. Reload it to edit the current file.",
        "⟦Thís sávégámé çháñgéd óñ dísk síñçé ít wás ópéñéd. Rélóád ít tó édít thé çúrréñt fílé.⟧",
    ),
    (
        "The savegame changed on disk outside JadeForge. Reloading discards every unsaved edit on screen.",
        "⟦Thé sávégámé çháñgéd óñ dísk óútsídé JádéFórgé. Rélóádíñg dísçárds évéry úñsávéd édít óñ sçrééñ.⟧",
    ),
    ("{items} loaded", "⟦{ítéms} lóádéd⟧"),
    (
        "Choose the Jade Empire save folder",
        "⟦Çhóósé thé Jádé Émpíré sávé fóldér⟧",
    ),
    (
        "This savegame has changes that are only in memory. Save them before closing, discard them, or cancel and keep editing.",
        "⟦Thís sávégámé hás çháñgés thát áré óñly íñ mémóry. Sávé thém béfóré çlósíñg, dísçárd thém, ór çáñçél áñd kéép édítíñg.⟧",
    ),
    ("Save", "⟦Sávé⟧"),
    ("Discard", "⟦Dísçárd⟧"),
    ("Cancel", "⟦Çáñçél⟧"),
    ("Select a savegame", "⟦Séléçt á sávégámé⟧"),
    (
        "No savegames in this folder.",
        "⟦Ñó sávégámés íñ thís fóldér.⟧",
    ),
    (
        "No savegame matches this filter.",
        "⟦Ñó sávégámé mátçhés thís fíltér.⟧",
    ),
    ("{shown} OF {total}", "⟦{shówñ} óf {tótál}⟧"),
    (
        "Level {level} · Time played {time}",
        "⟦Lévél {lévél} · Tímé pláyéd {tímé}⟧",
    ),
    (
        "Sort by {name} ({direction})",
        "⟦Sórt by {ñámé} ({díréçtíóñ})⟧",
    ),
    ("Sort by {name}", "⟦Sórt by {ñámé}⟧"),
    (
        "Filter by name or character",
        "⟦Fíltér by ñámé ór çháráçtér⟧",
    ),
    (
        "Filter the item database, or leave empty to browse it",
        "⟦Fíltér thé ítém dátábásé, ór léávé émpty tó brówsé ít⟧",
    ),
    (
        "Filter the style list, or leave empty to browse it",
        "⟦Fíltér thé stylé líst, ór léávé émpty tó brówsé ít⟧",
    ),
    (
        "Filter the gem list, or leave empty to browse it",
        "⟦Fíltér thé gém líst, ór léávé émpty tó brówsé ít⟧",
    ),
    (
        "Reading the game installation and its tables…",
        "⟦Réádíñg thé gámé íñstállátíóñ áñd íts táblés…⟧",
    ),
    (
        "No game installation found next to the savegames: names fall back to ids, while categories, levels and timings still come from the savegame.",
        "⟦Ñó gámé íñstállátíóñ fóúñd ñéxt tó thé sávégámés: ñámés fáll báçk tó íds, whílé çátégóríés, lévéls áñd tímíñgs stíll çómé fróm thé sávégámé.⟧",
    ),
    (
        ". Bundled English names are still available.",
        "⟦. Búñdléd Éñglísh ñámés áré stíll áváíláblé.⟧",
    ),
    ("Unspent attribute points", "⟦Úñspéñt áttríbúté póíñts⟧"),
    ("Body / Mind / Spirit", "⟦Bódy / Míñd / Spírít⟧"),
    (
        "Charm / Intuition / Intimidate",
        "⟦Çhárm / Íñtúítíóñ / Íñtímídáté⟧",
    ),
    ("unknown size", "⟦úñkñówñ sízé⟧"),
    ("Notice", "⟦Ñótíçé⟧"),
    ("linked", "⟦líñkéd⟧"),
    ("loading", "⟦lóádíñg⟧"),
    ("not found", "⟦ñót fóúñd⟧"),
    (
        "{items} and {styles} resolved from {root}",
        "⟦{ítéms} áñd {stylés} résólvéd fróm {róót}⟧",
    ),
    (
        "No game installation found: using bundled English names for {items} and {styles}. Descriptions and upgrades still need the game data.",
        "⟦Ñó gámé íñstállátíóñ fóúñd: úsíñg búñdléd Éñglísh ñámés fór {ítéms} áñd {stylés}. Désçríptíóñs áñd úpgrádés stíll ñééd thé gámé dátá.⟧",
    ),
    (
        "Cannot read the game data: {error}{bundled}",
        "⟦Çáññót réád thé gámé dátá: {érrór}{búñdléd}⟧",
    ),
    (
        "Time played {time} · {kills} · {experience}",
        "⟦Tímé pláyéd {tímé} · {kílls} · {éxpéríéñçé}⟧",
    ),
    (
        "Alignment is a number; below 128 is the Closed Fist, above is the Open Palm.",
        "⟦Álígñméñt ís á ñúmbér; bélów 128 ís thé Çlóséd Físt, ábóvé ís thé Ópéñ Pálm.⟧",
    ),
    (
        "Every valid change is applied in memory immediately",
        "⟦Évéry válíd çháñgé ís ápplíéd íñ mémóry ímmédíátély⟧",
    ),
    ("+1 point", "⟦+1 póíñt⟧"),
    ("+5 points", "⟦+5 póíñts⟧"),
    (
        "{alignment} (the game starts at {default}, below is Closed Fist, above is Open Palm).",
        "⟦{álígñméñt} (thé gámé stárts át {défáúlt}, bélów ís Çlóséd Físt, ábóvé ís Ópéñ Pálm).⟧",
    ),
    ("EXPERIENCE", "⟦ÉXPÉRÍÉÑÇÉ⟧"),
    ("BODY", "⟦BÓDY⟧"),
    ("MIND", "⟦MÍÑD⟧"),
    ("SPIRIT", "⟦SPÍRÍT⟧"),
    ("CHARM", "⟦ÇHÁRM⟧"),
    ("INTUITION", "⟦ÍÑTÚÍTÍÓÑ⟧"),
    ("INTIMIDATE", "⟦ÍÑTÍMÍDÁTÉ⟧"),
    ("ALIGNMENT", "⟦ÁLÍGÑMÉÑT⟧"),
    ("at most 64 characters", "⟦át móst 64 çháráçtérs⟧"),
    ("1 to 255 (level)", "⟦1 tó 255 (lévél)⟧"),
    (
        "empty or 0 to 4294967295 gold",
        "⟦émpty ór 0 tó 4294967295 góld⟧",
    ),
    ("0 to 1000000 (index)", "⟦0 tó 1000000 (íñdéx)⟧"),
    (
        "-1000000 to 1000000, 128 balanced",
        "⟦-1000000 tó 1000000, 128 báláñçéd⟧",
    ),
    ("0 to 100 (chapter)", "⟦0 tó 100 (çháptér)⟧"),
    (
        "Every style in the game data is already learned.",
        "⟦Évéry stylé íñ thé gámé dátá ís álréády léárñéd.⟧",
    ),
    (
        "No styles match this search.",
        "⟦Ñó stylés mátçh thís séárçh.⟧",
    ),
    ("Learn", "⟦Léárñ⟧"),
    ("Styles you can learn", "⟦Stylés yóú çáñ léárñ⟧"),
    ("Remove style?", "⟦Rémóvé stylé?⟧"),
    (
        "This style has no level-up table in the game data, so its upgrades are read-only here.",
        "⟦Thís stylé hás ñó lévél-úp táblé íñ thé gámé dátá, só íts úpgrádés áré réád-óñly héré.⟧",
    ),
    (
        "Upgrade details need the game data: point JadeForge at the game installation to see them.",
        "⟦Úpgrádé détáíls ñééd thé gámé dátá: póíñt JádéFórgé át thé gámé íñstállátíóñ tó séé thém.⟧",
    ),
    (
        "owned: clicking removes this tier and every tier above it",
        "⟦ówñéd: çlíçkíñg rémóvés thís tíér áñd évéry tíér ábóvé ít⟧",
    ),
    (
        "available: clicking buys every missing tier up to this one",
        "⟦áváíláblé: çlíçkíñg búys évéry míssíñg tíér úp tó thís óñé⟧",
    ),
    ("no listed effect", "⟦ñó lístéd éfféçt⟧"),
    ("no style point cost", "⟦ñó stylé póíñt çóst⟧"),
    (
        "The styles section could not be read: {error}",
        "⟦Thé stylés séçtíóñ çóúld ñót bé réád: {érrór}⟧",
    ),
    ("{name} style", "⟦{ñámé} stylé⟧"),
    ("Learn {name} style", "⟦Léárñ {ñámé} stylé⟧"),
    (
        "Remove {name} from the fighting styles? You can still press Revert before saving.",
        "⟦Rémóvé {ñámé} fróm thé fíghtíñg stylés? Yóú çáñ stíll préss Révért béfóré sávíñg.⟧",
    ),
    ("Remove {name} style", "⟦Rémóvé {ñámé} stylé⟧"),
    (
        "{style}, {family}, tier {tier}, {name}, owned: removes this tier and every tier above it",
        "⟦{stylé}, {fámíly}, tíér {tíér}, {ñámé}, ówñéd: rémóvés thís tíér áñd évéry tíér ábóvé ít⟧",
    ),
    (
        "{style}, {family}, tier {tier}, {name}{effect}, buys every missing tier up to this one",
        "⟦{stylé}, {fámíly}, tíér {tíér}, {ñámé}{éfféçt}, búys évéry míssíñg tíér úp tó thís óñé⟧",
    ),
    (
        "{family} tier {tier}: {name}",
        "⟦{fámíly} tíér {tíér}: {ñámé}⟧",
    ),
    (
        "{effect} · {cost} · {action}",
        "⟦{éfféçt} · {çóst} · {áçtíóñ}⟧",
    ),
    ("icon {icon}", "⟦íçóñ {íçóñ}⟧"),
    ("{time} used", "⟦{tímé} úséd⟧"),
    ("weapon {weapon}", "⟦wéápóñ {wéápóñ}⟧"),
    (
        "Saved style data: {fields}.",
        "⟦Sávéd stylé dátá: {fíélds}.⟧",
    ),
    ("No items in this category.", "⟦Ñó ítéms íñ thís çátégóry.⟧"),
    (
        "The item database is empty.",
        "⟦Thé ítém dátábásé ís émpty.⟧",
    ),
    (
        "No items match this search and category.",
        "⟦Ñó ítéms mátçh thís séárçh áñd çátégóry.⟧",
    ),
    ("SEARCH RESULTS", "⟦SÉÁRÇH RÉSÚLTS⟧"),
    ("Items you can add", "⟦Ítéms yóú çáñ ádd⟧"),
    ("All items", "⟦Áll ítéms⟧"),
    (
        "Show items from every category",
        "⟦Shów ítéms fróm évéry çátégóry⟧",
    ),
    ("Remove item?", "⟦Rémóvé ítém?⟧"),
    (
        "The inventory section could not be read: {error}",
        "⟦Thé íñvéñtóry séçtíóñ çóúld ñót bé réád: {érrór}⟧",
    ),
    ("{name} item", "⟦{ñámé} ítém⟧"),
    ("type {kind}", "⟦typé {kíñd}⟧"),
    ("Show only {label}", "⟦Shów óñly {lábél}⟧"),
    (
        "Remove {name} from the inventory? You can still press Revert before saving.",
        "⟦Rémóvé {ñámé} fróm thé íñvéñtóry? Yóú çáñ stíll préss Révért béfóré sávíñg.⟧",
    ),
    (
        "{name}: enter a stack size from 1 to 255.",
        "⟦{ñámé}: éñtér á stáçk sízé fróm 1 tó 255.⟧",
    ),
    ("Decrease {name} stack", "⟦Déçréásé {ñámé} stáçk⟧"),
    ("Increase {name} stack", "⟦Íñçréásé {ñámé} stáçk⟧"),
    ("No amulet yet", "⟦Ñó ámúlét yét⟧"),
    ("no effects", "⟦ñó éfféçts⟧"),
    ("Add", "⟦Ádd⟧"),
    (
        "The game data with the gem definitions was not found.",
        "⟦Thé gámé dátá wíth thé gém défíñítíóñs wás ñót fóúñd.⟧",
    ),
    (
        "No gem matches this filter.",
        "⟦Ñó gém mátçhés thís fíltér.⟧",
    ),
    ("Gems you can add", "⟦Géms yóú çáñ ádd⟧"),
    ("From the game data", "⟦Fróm thé gámé dátá⟧"),
    (
        "All seven sockets are unlocked.",
        "⟦Áll sévéñ sóçkéts áré úñlóçkéd.⟧",
    ),
    (
        "More sockets unlock as the story progresses.",
        "⟦Móré sóçkéts úñlóçk ás thé stóry prógréssés.⟧",
    ),
    ("Move to inventory", "⟦Móvé tó íñvéñtóry⟧"),
    ("Move to amulet", "⟦Móvé tó ámúlét⟧"),
    ("Amulet full", "⟦Ámúlét fúll⟧"),
    ("Remove", "⟦Rémóvé⟧"),
    ("Remove gem?", "⟦Rémóvé gém?⟧"),
    (
        "The amulet section could not be read: {error}",
        "⟦Thé ámúlét séçtíóñ çóúld ñót bé réád: {érrór}⟧",
    ),
    (
        "The gems section could not be read: {error}",
        "⟦Thé géms séçtíóñ çóúld ñót bé réád: {érrór}⟧",
    ),
    ("{name} gem, {effects}", "⟦{ñámé} gém, {éfféçts}⟧"),
    ("Add {name}", "⟦Ádd {ñámé}⟧"),
    (
        "{socketed} of {sockets} filled · amulet tier {tier}",
        "⟦{sóçkétéd} óf {sóçkéts} fílléd · ámúlét tíér {tíér}⟧",
    ),
    ("{action} ({name})", "⟦{áçtíóñ} ({ñámé})⟧"),
    ("Remove {name}", "⟦Rémóvé {ñámé}⟧"),
    (
        "Remove {name}? You can still press Revert before saving.",
        "⟦Rémóvé {ñámé}? Yóú çáñ stíll préss Révért béfóré sávíñg.⟧",
    ),
    ("{name} gem", "⟦{ñámé} gém⟧"),
    ("Hide", "⟦Hídé⟧"),
    ("Make available", "⟦Máké áváíláblé⟧"),
    (
        "The companions section could not be read: {error}",
        "⟦Thé çómpáñíóñs séçtíóñ çóúld ñót bé réád: {érrór}⟧",
    ),
    (
        "{name}: enter a level from 1 to 60.",
        "⟦{ñámé}: éñtér á lévél fróm 1 tó 60.⟧",
    ),
    ("{name} companion", "⟦{ñámé} çómpáñíóñ⟧"),
    ("Decrease {name} level", "⟦Déçréásé {ñámé} lévél⟧"),
    ("Increase {name} level", "⟦Íñçréásé {ñámé} lévél⟧"),
    ("Hide {name}", "⟦Hídé {ñámé}⟧"),
    ("Make {name} available", "⟦Máké {ñámé} áváíláblé⟧"),
    ("timestamps are UTC", "⟦tíméstámps áré ÚTÇ⟧"),
    ("Restore this backup?", "⟦Réstóré thís báçkúp?⟧"),
    (
        "The current savegame is replaced by the chosen backup. JadeForge first writes a new backup of the current file, so the restore can be undone.",
        "⟦Thé çúrréñt sávégámé ís répláçéd by thé çhóséñ báçkúp. JádéFórgé fírst wrítés á ñéw báçkúp óf thé çúrréñt fílé, só thé réstóré çáñ bé úñdóñé.⟧",
    ),
    (
        "Backups of the open savegame, newest first. A restore verifies the chosen file, keeps a new backup of the file it replaces, and writes atomically. Open savegame: {open}",
        "⟦Báçkúps óf thé ópéñ sávégámé, ñéwést fírst. Á réstóré vérífíés thé çhóséñ fílé, kééps á ñéw báçkúp óf thé fílé ít répláçés, áñd wrítés átómíçálly. Ópéñ sávégámé: {ópéñ}⟧",
    ),
    (
        "Restore the backup from {timestamp}",
        "⟦Réstóré thé báçkúp fróm {tíméstámp}⟧",
    ),
    ("Original: {path}", "⟦Órígíñál: {páth}⟧"),
    (
        "There are unsaved changes, but no described field changed.",
        "⟦Théré áré úñsávéd çháñgés, bút ñó désçríbéd fíéld çháñgéd.⟧",
    ),
    ("There is nothing to write.", "⟦Théré ís ñóthíñg tó wríté.⟧"),
    (
        "{count} character field(s) are still invalid and will not be written. Go back to fix them if you need those values.",
        "⟦{çóúñt} çháráçtér fíéld(s) áré stíll íñválíd áñd wíll ñót bé wríttéñ. Gó báçk tó fíx thém íf yóú ñééd thósé válúés.⟧",
    ),
    ("{changes} in {groups}", "⟦{çháñgés} íñ {gróúps}⟧"),
    ("was {before}, now {after}", "⟦wás {béfóré}, ñów {áftér}⟧"),
    ("Copy path", "⟦Çópy páth⟧"),
    ("Open folder", "⟦Ópéñ fóldér⟧"),
    (
        "Copied the {what} path to the clipboard.",
        "⟦Çópíéd thé {whát} páth tó thé çlípbóárd.⟧",
    ),
    (
        "Could not copy the {what} path: the system clipboard is not available.",
        "⟦Çóúld ñót çópy thé {whát} páth: thé systém çlípbóárd ís ñót áváíláblé.⟧",
    ),
    (
        "Opened the {what} folder in the file manager.",
        "⟦Ópéñéd thé {whát} fóldér íñ thé fílé máñágér.⟧",
    ),
    (
        "Could not open the {what} folder {folder}: {error}",
        "⟦Çóúld ñót ópéñ thé {whát} fóldér {fóldér}: {érrór}⟧",
    ),
    ("Copy the {what} path", "⟦Çópy thé {whát} páth⟧"),
    ("Open the {what} folder", "⟦Ópéñ thé {whát} fóldér⟧"),
    (
        "Reading the game installation…",
        "⟦Réádíñg thé gámé íñstállátíóñ…⟧",
    ),
    (
        "Bundled English names (no installation)",
        "⟦Búñdléd Éñglísh ñámés (ñó íñstállátíóñ)⟧",
    ),
    ("No installation configured", "⟦Ñó íñstállátíóñ çóñfígúréd⟧"),
    ("Version", "⟦Vérsíóñ⟧"),
    ("Open savegame", "⟦Ópéñ sávégámé⟧"),
    ("none", "⟦ñóñé⟧"),
    ("File size", "⟦Fílé sízé⟧"),
    ("not read", "⟦ñót réád⟧"),
    ("Renderer override", "⟦Réñdérér óvérrídé⟧"),
    ("auto", "⟦áútó⟧"),
    ("Remember window", "⟦Rémémbér wíñdów⟧"),
    ("yes", "⟦yés⟧"),
    ("no", "⟦ñó⟧"),
    ("Backup folder", "⟦Báçkúp fóldér⟧"),
    ("JadeForge diagnostics", "⟦JádéFórgé díágñóstíçs⟧"),
    ("unavailable", "⟦úñáváíláblé⟧"),
    (
        "Copied the diagnostics to the clipboard.",
        "⟦Çópíéd thé díágñóstíçs tó thé çlípbóárd.⟧",
    ),
    (
        "Could not copy the diagnostics: the system clipboard is not available.",
        "⟦Çóúld ñót çópy thé díágñóstíçs: thé systém çlípbóárd ís ñót áváíláblé.⟧",
    ),
    ("descending", "⟦désçéñdíñg⟧"),
    ("ascending", "⟦ásçéñdíñg⟧"),
    ("Settings file", "⟦Séttíñgs fílé⟧"),
    ("Latest backup", "⟦Látést báçkúp⟧"),
    (
        "JadeForge is MIT licensed. The bundled English item and style names are game content and are not covered by that license; see NOTICE and docs/NAMES.md.",
        "⟦JádéFórgé ís MÍT líçéñséd. Thé búñdléd Éñglísh ítém áñd stylé ñámés áré gámé çóñtéñt áñd áré ñót çóvéréd by thát líçéñsé; séé ÑÓTÍÇÉ áñd dóçs/ÑÁMÉS.md.⟧",
    ),
    ("Save (opens the review)", "⟦Sávé (ópéñs thé révíéw)⟧"),
    ("Focus the active filter", "⟦Fóçús thé áçtívé fíltér⟧"),
    ("Undo the last edit", "⟦Úñdó thé lást édít⟧"),
    ("Redo an undone edit", "⟦Rédó áñ úñdóñé édít⟧"),
    ("Scan the save folder", "⟦Sçáñ thé sávé fóldér⟧"),
    ("Open this help page", "⟦Ópéñ thís hélp págé⟧"),
    (
        "Clear the active filter or close this page",
        "⟦Çléár thé áçtívé fíltér ór çlósé thís págé⟧",
    ),
    ("Unreadable: {error}", "⟦Úñréádáblé: {érrór}⟧"),
    (
        "Use {percent} percent text size",
        "⟦Úsé {pérçéñt} pérçéñt téxt sízé⟧",
    ),
    (
        "Use {language} for the interface",
        "⟦Úsé {láñgúágé} fór thé íñtérfáçé⟧",
    ),
    ("{theme} at {scale} %", "⟦{thémé} át {sçálé} %⟧"),
    ("{name}, {direction}", "⟦{ñámé}, {díréçtíóñ}⟧"),
    ("Version {version}", "⟦Vérsíóñ {vérsíóñ}⟧"),
    ("expanded", "⟦éxpáñdéd⟧"),
    ("collapsed", "⟦çóllápséd⟧"),
    (
        "{name}, {subtitle}, {state}, Details",
        "⟦{ñámé}, {súbtítlé}, {státé}, Détáíls⟧",
    ),
    (
        "Follow the operating system's light or dark preference",
        "⟦Fóllów thé ópérátíñg systém's líght ór dárk préféréñçé⟧",
    ),
    (
        "Gem socketed into the amulet.",
        "⟦Gém sóçkétéd íñtó thé ámúlét.⟧",
    ),
    ("System", "⟦Systém⟧"),
    ("Dark", "⟦Dárk⟧"),
    ("Light", "⟦Líght⟧"),
    ("High contrast", "⟦Hígh çóñtrást⟧"),
    (
        "The original dark jade theme",
        "⟦Thé órígíñál dárk jádé thémé⟧",
    ),
    (
        "A light parchment theme for bright rooms",
        "⟦Á líght párçhméñt thémé fór bríght róóms⟧",
    ),
    (
        "Black and white with a yellow focus ring",
        "⟦Bláçk áñd whíté wíth á yéllów fóçús ríñg⟧",
    ),
    ("Cannot be read", "⟦Çáññót bé réád⟧"),
    ("Way of the Open Palm", "⟦Wáy óf thé Ópéñ Pálm⟧"),
    ("Way of the Closed Fist", "⟦Wáy óf thé Çlóséd Físt⟧"),
    ("Balanced", "⟦Báláñçéd⟧"),
    ("Power-ups", "⟦Pówér-úps⟧"),
    ("Plot items", "⟦Plót ítéms⟧"),
    ("Magic weapons", "⟦Mágíç wéápóñs⟧"),
    ("Improvements", "⟦Ímpróvéméñts⟧"),
    ("Martial", "⟦Mártíál⟧"),
    ("Magic", "⟦Mágíç⟧"),
    ("Support", "⟦Súppórt⟧"),
    ("Transformation", "⟦Tráñsfórmátíóñ⟧"),
    ("Improvised", "⟦Ímpróvíséd⟧"),
    ("Unique", "⟦Úñíqúé⟧"),
    (
        "Choose the folder that contains your Jade Empire savegames.",
        "⟦Çhóósé thé fóldér thát çóñtáíñs yóúr Jádé Émpíré sávégámés.⟧",
    ),
    ("Scanning for savegames…", "⟦Sçáññíñg fór sávégámés…⟧"),
    (
        "This savegame has unsaved changes: save them or press Revert before scanning.",
        "⟦Thís sávégámé hás úñsávéd çháñgés: sávé thém ór préss Révért béfóré sçáññíñg.⟧",
    ),
    (
        "Scan cancelled; the savegames on screen are unchanged.",
        "⟦Sçáñ çáñçélléd; thé sávégámés óñ sçrééñ áré úñçháñgéd.⟧",
    ),
    (
        "This savegame has unsaved changes: save them or press Revert first.",
        "⟦Thís sávégámé hás úñsávéd çháñgés: sávé thém ór préss Révért fírst.⟧",
    ),
    (
        "That savegame no longer exists; scan the folder again.",
        "⟦Thát sávégámé ñó lóñgér éxísts; sçáñ thé fóldér ágáíñ.⟧",
    ),
    ("There is nothing to undo.", "⟦Théré ís ñóthíñg tó úñdó.⟧"),
    ("There is nothing to redo.", "⟦Théré ís ñóthíñg tó rédó.⟧"),
    ("Savegame structure", "⟦Sávégámé strúçtúré⟧"),
    ("Block layout", "⟦Blóçk láyóút⟧"),
    ("as opened", "⟦ás ópéñéd⟧"),
    (
        "structs were added or orphaned",
        "⟦strúçts wéré áddéd ór órpháñéd⟧",
    ),
    ("No savegame is open.", "⟦Ñó sávégámé ís ópéñ.⟧"),
    (
        "Reloading from disk would discard the unsaved changes. Save or revert them first.",
        "⟦Rélóádíñg fróm dísk wóúld dísçárd thé úñsávéd çháñgés. Sávé ór révért thém fírst.⟧",
    ),
    (
        "Reloaded the savegame from disk.",
        "⟦Rélóádéd thé sávégámé fróm dísk.⟧",
    ),
    (
        "Save or revert the current edits before restoring a backup.",
        "⟦Sávé ór révért thé çúrréñt édíts béfóré réstóríñg á báçkúp.⟧",
    ),
    (
        "That file is not a backup of the open savegame.",
        "⟦Thát fílé ís ñót á báçkúp óf thé ópéñ sávégámé.⟧",
    ),
    (
        "Nothing changed, so there was nothing to apply.",
        "⟦Ñóthíñg çháñgéd, só théré wás ñóthíñg tó ápply.⟧",
    ),
    (
        "Nothing changed, so there is nothing to write.",
        "⟦Ñóthíñg çháñgéd, só théré ís ñóthíñg tó wríté.⟧",
    ),
    (
        "A stack needs at least one item; use Remove instead.",
        "⟦Á stáçk ñééds át léást óñé ítém; úsé Rémóvé íñstéád.⟧",
    ),
    ("Item stack", "⟦Ítém stáçk⟧"),
    ("Item added", "⟦Ítém áddéd⟧"),
    ("not present", "⟦ñót préséñt⟧"),
    ("Item removed", "⟦Ítém rémóvéd⟧"),
    ("removed", "⟦rémóvéd⟧"),
    ("The item was already gone.", "⟦Thé ítém wás álréády góñé.⟧"),
    ("Style upgrade", "⟦Stylé úpgrádé⟧"),
    (
        "That upgrade is already in the requested state.",
        "⟦Thát úpgrádé ís álréády íñ thé réqúéstéd státé.⟧",
    ),
    ("Empty socket", "⟦Émpty sóçkét⟧"),
    (
        "That gem is not in the game data.",
        "⟦Thát gém ís ñót íñ thé gámé dátá.⟧",
    ),
    ("Gem added", "⟦Gém áddéd⟧"),
    ("inventory", "⟦íñvéñtóry⟧"),
    ("amulet", "⟦ámúlét⟧"),
    ("Gem moved", "⟦Gém móvéd⟧"),
    ("Gem removed", "⟦Gém rémóvéd⟧"),
    (
        "That style is not in the game data.",
        "⟦Thát stylé ís ñót íñ thé gámé dátá.⟧",
    ),
    (
        "Learn a style first so JadeForge can copy the field layout.",
        "⟦Léárñ á stylé fírst só JádéFórgé çáñ çópy thé fíéld láyóút.⟧",
    ),
    ("Style added", "⟦Stylé áddéd⟧"),
    (
        "Keep at least one style: JadeForge needs an existing style as the layout template for learning one.",
        "⟦Kéép át léást óñé stylé: JádéFórgé ñééds áñ éxístíñg stylé ás thé láyóút témpláté fór léárñíñg óñé.⟧",
    ),
    ("Style removed", "⟦Stylé rémóvéd⟧"),
    (
        "The style was already gone.",
        "⟦Thé stylé wás álréády góñé.⟧",
    ),
    (
        "Companion levels go from 1 to 60.",
        "⟦Çómpáñíóñ lévéls gó fróm 1 tó 60.⟧",
    ),
    ("Companion level", "⟦Çómpáñíóñ lévél⟧"),
    ("Companion availability", "⟦Çómpáñíóñ áváílábílíty⟧"),
    (
        "Scanning {directory} for savegames…",
        "⟦Sçáññíñg {díréçtóry} fór sávégámés…⟧",
    ),
    (
        "No savegames found in {directory}.",
        "⟦Ñó sávégámés fóúñd íñ {díréçtóry}.⟧",
    ),
    (
        "Found {savegames} in {directory}.",
        "⟦Fóúñd {sávégámés} íñ {díréçtóry}.⟧",
    ),
    (
        "Found {savegames} in {directory}, but none of them can be read.",
        "⟦Fóúñd {sávégámés} íñ {díréçtóry}, bút ñóñé óf thém çáñ bé réád.⟧",
    ),
    (
        "Found {savegames} in {directory}; {broken} of them cannot be read.",
        "⟦Fóúñd {sávégámés} íñ {díréçtóry}; {brókéñ} óf thém çáññót bé réád.⟧",
    ),
    (
        "Could not scan {directory}: the savegame reader stopped unexpectedly.",
        "⟦Çóúld ñót sçáñ {díréçtóry}: thé sávégámé réádér stóppéd úñéxpéçtédly.⟧",
    ),
    (
        "Using the game data in {root}.",
        "⟦Úsíñg thé gámé dátá íñ {róót}.⟧",
    ),
    (
        "{root} is not a readable Jade Empire installation: {error}",
        "⟦{róót} ís ñót á réádáblé Jádé Émpíré íñstállátíóñ: {érrór}⟧",
    ),
    (
        "Cannot open {label}: the scan found it unreadable ({error}).",
        "⟦Çáññót ópéñ {lábél}: thé sçáñ fóúñd ít úñréádáblé ({érrór}).⟧",
    ),
    ("Opened {label}.", "⟦Ópéñéd {lábél}.⟧"),
    (
        "Opened {label}, but some sections could not be read: {problems}",
        "⟦Ópéñéd {lábél}, bút sómé séçtíóñs çóúld ñót bé réád: {próbléms}⟧",
    ),
    (
        "Cannot open {path}: {error}",
        "⟦Çáññót ópéñ {páth}: {érrór}⟧",
    ),
    ("Item {item_id}", "⟦Ítém {ítém_íd}⟧"),
    ("Style {style_id}", "⟦Stylé {stylé_íd}⟧"),
    (
        "Cannot undo that step: {error}",
        "⟦Çáññót úñdó thát stép: {érrór}⟧",
    ),
    ("Undid {label}.", "⟦Úñdíd {lábél}.⟧"),
    ("Redid {label}.", "⟦Rédíd {lábél}.⟧"),
    (
        "Restored {backup}. The file it replaced is kept as {safety}.",
        "⟦Réstóréd {báçkúp}. Thé fílé ít répláçéd ís képt ás {sáféty}.⟧",
    ),
    (
        "Cannot restore the backup: {error}",
        "⟦Çáññót réstóré thé báçkúp: {érrór}⟧",
    ),
    (
        "Applied {changes} in memory.",
        "⟦Ápplíéd {çháñgés} íñ mémóry.⟧",
    ),
    (
        "Saved {path}. The previous file is kept as {backup}.",
        "⟦Sávéd {páth}. Thé prévíóús fílé ís képt ás {báçkúp}.⟧",
    ),
    ("Saved {path}.", "⟦Sávéd {páth}.⟧"),
    (
        "The savegame changed on disk since it was opened, so it was not overwritten. Reload it to keep the file on disk, or restore a backup. ({error})",
        "⟦Thé sávégámé çháñgéd óñ dísk síñçé ít wás ópéñéd, só ít wás ñót óvérwríttéñ. Rélóád ít tó kéép thé fílé óñ dísk, ór réstóré á báçkúp. ({érrór})⟧",
    ),
    ("Cannot save: {error}", "⟦Çáññót sávé: {érrór}⟧"),
    (
        "Saved {path}. The invalid character field was not applied: {error}",
        "⟦Sávéd {páth}. Thé íñválíd çháráçtér fíéld wás ñót ápplíéd: {érrór}⟧",
    ),
    (
        "{message} The character form also has an invalid field: {error}",
        "⟦{mésságé} Thé çháráçtér fórm álsó hás áñ íñválíd fíéld: {érrór}⟧",
    ),
    (
        "{name}: a stack cannot hold more than 255.",
        "⟦{ñámé}: á stáçk çáññót hóld móré tháñ 255.⟧",
    ),
    (
        "{name}: {count} in the stack.",
        "⟦{ñámé}: {çóúñt} íñ thé stáçk.⟧",
    ),
    (
        "Cannot change item: {error}",
        "⟦Çáññót çháñgé ítém: {érrór}⟧",
    ),
    (
        "item {item_id} has no savegame category type",
        "⟦ítém {ítém_íd} hás ñó sávégámé çátégóry typé⟧",
    ),
    ("Added {name}.", "⟦Áddéd {ñámé}.⟧"),
    ("Cannot add item: {error}", "⟦Çáññót ádd ítém: {érrór}⟧"),
    (
        "Cannot remove item: {error}",
        "⟦Çáññót rémóvé ítém: {érrór}⟧",
    ),
    ("Upgrade {advance}", "⟦Úpgrádé {ádváñçé}⟧"),
    ("Bought {upgrades}.", "⟦Bóúght {úpgrádés}.⟧"),
    (
        "Cannot change the upgrade: {error}",
        "⟦Çáññót çháñgé thé úpgrádé: {érrór}⟧",
    ),
    ("Gem {item_id}", "⟦Gém {ítém_íd}⟧"),
    (
        "Cannot add the gem: {error}",
        "⟦Çáññót ádd thé gém: {érrór}⟧",
    ),
    (
        "Cannot move the gem: {error}",
        "⟦Çáññót móvé thé gém: {érrór}⟧",
    ),
    (
        "Cannot remove the gem: {error}",
        "⟦Çáññót rémóvé thé gém: {érrór}⟧",
    ),
    ("Cannot add style: {error}", "⟦Çáññót ádd stylé: {érrór}⟧"),
    (
        "Cannot remove style: {error}",
        "⟦Çáññót rémóvé stylé: {érrór}⟧",
    ),
    (
        "Companion level set to {level}.",
        "⟦Çómpáñíóñ lévél sét tó {lévél}.⟧",
    ),
    (
        "Cannot change companion: {error}",
        "⟦Çáññót çháñgé çómpáñíóñ: {érrór}⟧",
    ),
    ("{name} is now {state}.", "⟦{ñámé} ís ñów {státé}.⟧"),
    ("Effect {kind}", "⟦Éfféçt {kíñd}⟧"),
    ("{value} effect {kind}", "⟦{válúé} éfféçt {kíñd}⟧"),
    (
        "The save folder {directory} does not exist. Check the path or use Browse.",
        "⟦Thé sávé fóldér {díréçtóry} dóés ñót éxíst. Çhéçk thé páth ór úsé Brówsé.⟧",
    ),
    (
        "The save folder {directory} cannot be read: permission denied.",
        "⟦Thé sávé fóldér {díréçtóry} çáññót bé réád: pérmíssíóñ déñíéd.⟧",
    ),
    (
        "The save folder {directory} cannot be read: {error}.",
        "⟦Thé sávé fóldér {díréçtóry} çáññót bé réád: {érrór}.⟧",
    ),
    (
        "{directory} is a file, not a save folder.",
        "⟦{díréçtóry} ís á fílé, ñót á sávé fóldér.⟧",
    ),
    (
        "The save folder {directory} cannot be listed: {error}.",
        "⟦Thé sávé fóldér {díréçtóry} çáññót bé lístéd: {érrór}.⟧",
    ),
    (
        "Gem moved to the inventory.",
        "⟦Gém móvéd tó thé íñvéñtóry.⟧",
    ),
    ("unknown", "⟦úñknówñ⟧"),
    ("settings", "⟦séttíñgs⟧"),
    // Merged labels, composed ranges and shared templates.
    ("theme", "⟦thémé⟧"),
    ("language", "⟦láñgúágé⟧"),
    ("save folder", "⟦sávé fóldér⟧"),
    ("Chapter", "⟦Çháptér⟧"),
    ("{stat} MAX", "⟦{stát} MÁX⟧"),
    ("{min} to {max} {unit}", "⟦{míñ} tó {máx} {úñít}⟧"),
    ("XP", "⟦XP⟧"),
    ("points", "⟦póíñts⟧"),
    ("health points", "⟦héálth póíñts⟧"),
    ("chi points", "⟦çhí póíñts⟧"),
    ("focus points", "⟦fóçús póíñts⟧"),
    ("{label}: {value}", "⟦{lábél}: {válúé}⟧"),
    ("Config", "⟦Çóñfíg⟧"),
    ("Removed {what}.", "⟦Rémóvéd {whát}.⟧"),
    // Count nouns: singular and plural for the count helper.
    ("gem", "⟦gém⟧"),
    ("gems", "⟦géms⟧"),
    ("carried gem", "⟦cárríéd gém⟧"),
    ("carried gems", "⟦cárríéd géms⟧"),
    ("socket", "⟦sóckét⟧"),
    ("sockets", "⟦sóckéts⟧"),
    ("companion known", "⟦cómpáníón knówn⟧"),
    ("companions known", "⟦cómpáníóns knówn⟧"),
    ("item", "⟦ítém⟧"),
    ("items", "⟦ítéms⟧"),
    ("match", "⟦mátch⟧"),
    ("matches", "⟦mátchés⟧"),
    ("stack", "⟦stáck⟧"),
    ("stacks", "⟦stácks⟧"),
    ("style", "⟦stýlé⟧"),
    ("styles", "⟦stýlés⟧"),
    ("style to learn", "⟦stýlé tó léárn⟧"),
    ("styles to learn", "⟦stýlés tó léárn⟧"),
    ("style learned", "⟦stýlé léárnéd⟧"),
    ("styles learned", "⟦stýlés léárnéd⟧"),
    ("style point", "⟦stýlé póínt⟧"),
    ("style points", "⟦stýlé póínts⟧"),
    ("change", "⟦chángé⟧"),
    ("changes", "⟦chángés⟧"),
    ("group", "⟦gróúp⟧"),
    ("groups", "⟦gróúps⟧"),
    ("backup", "⟦báckúp⟧"),
    ("backups", "⟦báckúps⟧"),
    ("invalid field", "⟦ínválíd fíéld⟧"),
    ("invalid fields", "⟦ínválíd fíélds⟧"),
    ("experience point", "⟦éxpéríéncé póínt⟧"),
    ("experience points", "⟦éxpéríéncé póínts⟧"),
    ("block", "⟦blóck⟧"),
    ("blocks", "⟦blócks⟧"),
    ("effect", "⟦éfféct⟧"),
    ("effects", "⟦éffécts⟧"),
    ("kill", "⟦kíll⟧"),
    ("kills", "⟦kílls⟧"),
    ("weapon", "⟦wéápóñ⟧"),
    ("weapons", "⟦wéápóñs⟧"),
    ("savegame", "⟦sávégámé⟧"),
    ("savegames", "⟦sávégámés⟧"),
    ("upgrade", "⟦úpgrádé⟧"),
    ("upgrades", "⟦úpgrádés⟧"),
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

    /// Extracts up to `wanted` string literals from every `name(...)` call.
    ///
    /// The parser only needs plain literals, which is what the UI uses; an
    /// escaped quote is handled so the scan cannot stop early. Nested calls are
    /// followed so a `plural(...)` inside a `fill(...)` argument is still seen.
    fn call_literals(source: &str, name: &str, wanted: usize) -> Vec<String> {
        let bytes = source.as_bytes();
        let mut literals = Vec::new();
        let mut index = 0;
        while let Some(found) = source.get(index..).and_then(|rest| rest.find(name)) {
            let at = index + found;
            let after = at + name.len();
            let boundary = at == 0 || {
                let previous = bytes[at - 1];
                !previous.is_ascii_alphanumeric() && previous != b'_'
            };
            let open = source
                .get(after..)
                .and_then(|rest| rest.trim_start().starts_with('(').then_some(true));
            if !boundary || open != Some(true) {
                index = after;
                continue;
            }
            let mut position = after;
            while position < bytes.len() && bytes[position] != b'(' {
                position += 1;
            }
            position += 1;
            let mut depth = 1_usize;
            let mut found_literals = 0_usize;
            while position < bytes.len() && depth > 0 {
                match bytes[position] {
                    b'(' => depth += 1,
                    b')' => depth -= 1,
                    b'"' => {
                        let start = position + 1;
                        let mut end = start;
                        let mut escaped = false;
                        while end < bytes.len() {
                            if escaped {
                                escaped = false;
                            } else {
                                match bytes[end] {
                                    b'\\' => escaped = true,
                                    b'"' => break,
                                    _ => {}
                                }
                            }
                            end += 1;
                        }
                        if end < bytes.len() {
                            let literal =
                                source.get(start..end).map(decode_rust).unwrap_or_default();
                            if !literal.is_empty() {
                                literals.push(literal);
                                found_literals += 1;
                            }
                            position = end;
                            if found_literals >= wanted {
                                break;
                            }
                        }
                    }
                    _ => {}
                }
                position += 1;
            }
            index = position.max(index + 1);
        }
        literals
    }

    fn translated_literals(source: &str) -> Vec<String> {
        let mut literals = call_literals(source, "t", 1);
        literals.extend(call_literals(source, "fill", 1));
        literals.extend(call_literals(source, "plural", 2));
        literals
    }

    fn assert_literals_are_catalogued(directory: &Path, literals: &[String]) {
        let missing: Vec<String> = literals
            .iter()
            .filter(|literal| !CATALOG.iter().any(|(source, _)| source == literal))
            .map(|literal| format!("{}: {literal}", directory.display()))
            .collect();
        assert!(
            missing.is_empty(),
            "missing catalog entries:\n{}",
            missing.join("\n")
        );
    }

    #[test]
    fn every_translated_literal_is_in_the_catalog() {
        let mut files = Vec::new();
        source_files(
            Path::new(env!("CARGO_MANIFEST_DIR")).join("src").as_path(),
            &mut files,
        );
        let mut literals = Vec::new();
        for file in files {
            if file.file_name().is_some_and(|name| name == "i18n.rs") {
                continue;
            }
            let source = std::fs::read_to_string(&file).expect("read source");
            literals.extend(translated_literals(&source));
        }
        assert_literals_are_catalogued(Path::new("gui"), &literals);
    }

    #[test]
    fn every_session_message_template_is_in_the_catalog() {
        let mut files = Vec::new();
        source_files(
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../jadeforge-session/src")
                .as_path(),
            &mut files,
        );
        let mut literals = Vec::new();
        for file in files {
            if file
                .file_name()
                .is_some_and(|name| name == "message.rs" || name == "i18n.rs")
            {
                continue;
            }
            let source = std::fs::read_to_string(&file).expect("read source");
            let Some(source) = source.split("#[cfg(test)]").next() else {
                continue;
            };
            literals.extend(call_literals(source, "tr", 1));
            literals.extend(call_literals(source, "fill", 1));
        }
        assert_literals_are_catalogued(Path::new("session"), &literals);
    }

    #[test]
    fn every_character_field_caption_is_in_the_catalog() {
        use crate::session::CharacterField;
        for field in CharacterField::ALL {
            assert!(
                CATALOG.iter().any(|(source, _)| *source == field.label()),
                "missing catalog entry for the field caption {:?}",
                field.label()
            );
        }
    }

    #[test]
    fn every_filter_placeholder_is_in_the_catalog() {
        for placeholder in [
            crate::app::SAVE_FILTER_PLACEHOLDER,
            crate::app::ITEM_FILTER_PLACEHOLDER,
            crate::app::STYLE_FILTER_PLACEHOLDER,
            crate::app::GEM_FILTER_PLACEHOLDER,
        ] {
            assert!(
                CATALOG.iter().any(|(source, _)| *source == placeholder),
                "missing catalog entry for the filter placeholder {placeholder:?}"
            );
        }
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
        for locale in Locale::ALL {
            install(locale);
            assert_eq!(
                t("A string that is not catalogued"),
                "A string that is not catalogued"
            );
        }
        install(Locale::English);
    }

    #[test]
    fn the_session_hook_uses_the_gui_catalog() {
        install(Locale::German);
        crate::message::install_translator(t);
        assert_eq!(crate::message::tr("About"), "Über JadeForge");
        install(Locale::English);
    }

    #[test]
    fn fill_replaces_every_named_placeholder() {
        install(Locale::English);
        assert_eq!(
            fill("Opened {label}.", &[("label", "Auto 1"), ("unused", "x")]),
            "Opened Auto 1."
        );
        assert_eq!(
            fill(
                "Found {savegames} in {directory}.",
                &[("savegames", "2 savegames"), ("directory", "/saves")]
            ),
            "Found 2 savegames in /saves."
        );
        install(Locale::English);
    }

    #[test]
    fn the_german_catalog_covers_every_catalogued_string() {
        let catalog = german();
        assert!(
            !catalog.is_empty(),
            "locales/de.toml did not parse as a string table"
        );
        let missing: Vec<&str> = CATALOG
            .iter()
            .map(|(source, _)| *source)
            .filter(|source| !catalog.contains_key(*source))
            .collect();
        assert!(
            missing.is_empty(),
            "missing German catalog keys:\n{}",
            missing.join("\n")
        );
        let sources: std::collections::HashSet<&str> =
            CATALOG.iter().map(|(source, _)| *source).collect();
        let mut stale: Vec<&str> = catalog
            .keys()
            .filter(|key| !sources.contains(key.as_str()))
            .map(String::as_str)
            .collect();
        stale.sort_unstable();
        assert!(
            stale.is_empty(),
            "German catalog keys that no longer exist:\n{}",
            stale.join("\n")
        );
    }

    /// Run after filling `locales/de.toml`; empty values fall back to English.
    #[test]
    #[ignore = "run after filling locales/de.toml"]
    fn every_german_value_is_filled() {
        let mut empty: Vec<&str> = german()
            .iter()
            .filter(|(_, value)| value.trim().is_empty())
            .map(|(key, _)| key.as_str())
            .collect();
        empty.sort_unstable();
        assert!(
            empty.is_empty(),
            "{} untranslated German keys:\n{}",
            empty.len(),
            empty.join("\n")
        );
    }
}
