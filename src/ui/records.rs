use std::cell::RefCell;
use std::io;
use std::rc::Rc;
use std::{fs, path::PathBuf};

use gtk4 as gtk;
use gtk4::glib;
use gtk4::prelude::*;
use libadwaita as adw;
use adw::prelude::*;
use serde::{Deserialize, Serialize};

use crate::i18n::tr;

use super::infinite;
use super::state::{AppState, Difficulty, InfiniteRecord, ModeRecord, PlayerRecords, Rank};

const RECORDS_FILE_NAME: &str = "records.json";
const LEGACY_RECORDS_FILE_NAME: &str = "records.v1";
const MODE_HISTORY_LIMIT: usize = 200;
const INFINITE_HISTORY_LIMIT: usize = 200;

fn format_mm_ss(total_secs: u32) -> String {
    let mins = total_secs / 60;
    let secs = total_secs % 60;
    format!("{:02}:{:02}", mins, secs)
}

fn classic_level_name(level: u8) -> &'static str {
    match level.clamp(1, 4) {
        1 => "Easy",
        2 => "Medium",
        3 => "Hard",
        _ => "Expert",
    }
}

fn rank_for_precision(level: u8, precision_pct: u8) -> Rank {
    if precision_pct >= 100 {
        return Rank::S;
    }
    let a_threshold = match level.clamp(1, 4) {
        1 => 85,
        2 => 90,
        3 => 88,
        _ => 85,
    };
    let b_threshold = match level.clamp(1, 4) {
        1 => 70,
        2 => 80,
        3 => 75,
        _ => 70,
    };
    if precision_pct >= a_threshold {
        Rank::A
    } else if precision_pct >= b_threshold {
        Rank::B
    } else {
        Rank::C
    }
}

fn records_path() -> Option<PathBuf> {
    Some(glib::user_config_dir().join("recall").join(RECORDS_FILE_NAME))
}

fn legacy_records_path() -> Option<PathBuf> {
    Some(glib::user_config_dir().join("recall").join(LEGACY_RECORDS_FILE_NAME))
}

fn parse_mode_record(raw: &str) -> Option<ModeRecord> {
    let mut parts = raw.split('|');
    Some(ModeRecord {
        level: parts.next()?.parse().ok()?,
        rank: Rank::from_str(parts.next()?)?,
        time_secs: parts.next()?.parse().ok()?,
        precision_pct: parts.next()?.parse().ok()?,
        date_label: parts.next()?.to_string(),
    })
}

fn parse_infinite_record(raw: &str) -> Option<InfiniteRecord> {
    let mut parts = raw.split('|');
    Some(InfiniteRecord {
        round: parts.next()?.parse().ok()?,
        segment_level: parts.next()?.parse().ok()?,
        segment_survival: parts.next()?.parse().ok()?,
        time_secs: parts.next()?.parse().ok()?,
        date_label: parts.next()?.to_string(),
    })
}

fn parse_legacy_mode_best(raw: &str) -> Option<ModeRecord> {
    let mut parts = raw.split('|');
    Some(ModeRecord {
        level: parts.next()?.parse().ok()?,
        rank: Rank::from_str(parts.next()?)?,
        time_secs: parts.next()?.parse().ok()?,
        precision_pct: parts.next()?.parse().ok()?,
        date_label: String::new(),
    })
}

fn parse_legacy_infinite_best(raw: &str) -> Option<InfiniteRecord> {
    let mut parts = raw.split('|');
    Some(InfiniteRecord {
        round: parts.next()?.parse().ok()?,
        segment_level: parts.next()?.parse().ok()?,
        segment_survival: parts.next()?.parse().ok()?,
        time_secs: parts.next()?.parse().ok()?,
        date_label: String::new(),
    })
}

#[derive(Default, Deserialize, Serialize)]
struct RecordsFile {
    #[serde(default)]
    classic: Vec<ModeRecordWire>,
    #[serde(default, alias = "tri")]
    trio: Vec<ModeRecordWire>,
    #[serde(default)]
    infinite: Vec<InfiniteRecordWire>,
}

#[derive(Deserialize, Serialize)]
struct ModeRecordWire {
    level: u8,
    time_secs: u32,
    precision_pct: u8,
    rank: Rank,
    date_label: String,
}

#[derive(Deserialize, Serialize)]
struct InfiniteRecordWire {
    round: u32,
    segment_level: u8,
    segment_survival: u32,
    time_secs: u32,
    date_label: String,
}

impl From<ModeRecordWire> for ModeRecord {
    fn from(value: ModeRecordWire) -> Self {
        Self {
            level: value.level,
            time_secs: value.time_secs,
            precision_pct: value.precision_pct,
            rank: value.rank,
            date_label: value.date_label,
        }
    }
}

impl From<&ModeRecord> for ModeRecordWire {
    fn from(value: &ModeRecord) -> Self {
        Self {
            level: value.level,
            time_secs: value.time_secs,
            precision_pct: value.precision_pct,
            rank: value.rank,
            date_label: value.date_label.clone(),
        }
    }
}

impl From<InfiniteRecordWire> for InfiniteRecord {
    fn from(value: InfiniteRecordWire) -> Self {
        Self {
            round: value.round,
            segment_level: value.segment_level,
            segment_survival: value.segment_survival,
            time_secs: value.time_secs,
            date_label: value.date_label,
        }
    }
}

impl From<&InfiniteRecord> for InfiniteRecordWire {
    fn from(value: &InfiniteRecord) -> Self {
        Self {
            round: value.round,
            segment_level: value.segment_level,
            segment_survival: value.segment_survival,
            time_secs: value.time_secs,
            date_label: value.date_label.clone(),
        }
    }
}

impl From<RecordsFile> for PlayerRecords {
    fn from(value: RecordsFile) -> Self {
        Self {
            classic: value.classic.into_iter().map(ModeRecord::from).collect(),
            trio: value.trio.into_iter().map(ModeRecord::from).collect(),
            infinite: value
                .infinite
                .into_iter()
                .map(InfiniteRecord::from)
                .collect(),
        }
    }
}

impl From<&PlayerRecords> for RecordsFile {
    fn from(value: &PlayerRecords) -> Self {
        Self {
            classic: value.classic.iter().map(ModeRecordWire::from).collect(),
            trio: value.trio.iter().map(ModeRecordWire::from).collect(),
            infinite: value
                .infinite
                .iter()
                .map(InfiniteRecordWire::from)
                .collect(),
        }
    }
}

fn time_suffix_label(text: &str) -> gtk::Label {
    let label = gtk::Label::new(Some(text));
    label.add_css_class("score-row-time");
    label.add_css_class("numeric");
    label.set_halign(gtk::Align::End);
    label.set_valign(gtk::Align::Center);
    label
}

fn rank_suffix_label(text: &str) -> gtk::Label {
    let label = gtk::Label::new(Some(text));
    label.add_css_class("score-row-rank");
    label.add_css_class("caption");
    label.set_halign(gtk::Align::End);
    label.set_valign(gtk::Align::Center);
    label
}

fn now_date_label() -> String {
    if let Ok(dt) = glib::DateTime::now_local()
        && let Ok(text) = dt.format("%Y-%m-%d %H:%M")
    {
        return text.to_string();
    }
    tr("Unknown date")
}

fn load_legacy_records(raw: &str) -> PlayerRecords {
    let mut records = PlayerRecords::default();
    for line in raw.lines() {
        if let Some(rest) = line.strip_prefix("classic_entry=") {
            if let Some(entry) = parse_mode_record(rest) {
                records.classic.push(entry);
            }
        } else if let Some(rest) = line.strip_prefix("trio_entry=") {
            if let Some(entry) = parse_mode_record(rest) {
                records.trio.push(entry);
            }
        } else if let Some(rest) = line.strip_prefix("tri_entry=") {
            if let Some(entry) = parse_mode_record(rest) {
                records.trio.push(entry);
            }
        } else if let Some(rest) = line.strip_prefix("infinite_entry=") {
            if let Some(entry) = parse_infinite_record(rest) {
                records.infinite.push(entry);
            }
        } else if let Some(rest) = line.strip_prefix("classic=") {
            if let Some(entry) = parse_legacy_mode_best(rest) {
                records.classic.push(entry);
            }
        } else if let Some(rest) = line.strip_prefix("tri=") {
            if let Some(entry) = parse_legacy_mode_best(rest) {
                records.trio.push(entry);
            }
        } else if let Some(rest) = line.strip_prefix("infinite=")
            && let Some(entry) = parse_legacy_infinite_best(rest)
        {
            records.infinite.push(entry);
        }
    }
    records
}

fn load_json_records(raw: &str) -> Option<PlayerRecords> {
    let records_file: RecordsFile = serde_json::from_str(raw).ok()?;
    Some(records_file.into())
}

fn serialize_json_records(records: &PlayerRecords) -> String {
    serde_json::to_string_pretty(&RecordsFile::from(records))
        .expect("failed to serialize records file")
}

pub fn load_records() -> PlayerRecords {
    if let Some(path) = records_path()
        && let Ok(raw) = fs::read_to_string(&path)
    {
        if let Some(parsed) = load_json_records(&raw) {
            return parsed;
        } else if let Some(legacy_path) = legacy_records_path()
            && let Ok(legacy_raw) = fs::read_to_string(legacy_path)
        {
            let records = load_legacy_records(&legacy_raw);
            if let Err(err) = migrate_legacy_records(&records) {
                eprintln!("warning: failed to migrate legacy records: {err}");
            }
            return records;
        }

        eprintln!(
            "warning: failed to parse records file; keeping current file untouched: {}",
            path.display()
        );
        return PlayerRecords::default();
    } else if let Some(path) = legacy_records_path()
        && let Ok(raw) = fs::read_to_string(path)
    {
        let records = load_legacy_records(&raw);
        if let Err(err) = migrate_legacy_records(&records) {
            eprintln!("warning: failed to migrate legacy records: {err}");
        }
        return records;
    }

    PlayerRecords::default()
}

fn save_records(records: &PlayerRecords) -> io::Result<()> {
    let json_path = match records_path() {
        Some(path) => path,
        None => return Ok(()),
    };
    if let Some(parent) = json_path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(json_path, serialize_json_records(records))?;

    Ok(())
}

fn migrate_legacy_records(records: &PlayerRecords) -> io::Result<()> {
    save_records(records)?;

    if let Some(path) = legacy_records_path()
        && path.exists()
    {
        fs::remove_file(path)?;
    }

    Ok(())
}

fn sort_mode_records(entries: &mut [ModeRecord]) {
    entries.sort_by(|a, b| {
        b.level
            .cmp(&a.level)
            .then_with(|| b.rank.cmp(&a.rank))
            .then_with(|| b.precision_pct.cmp(&a.precision_pct))
            .then_with(|| a.time_secs.cmp(&b.time_secs))
    });
}

fn top_infinite_records(records: &[InfiniteRecord], limit: usize) -> Vec<InfiniteRecord> {
    let mut entries = records.to_vec();
    entries.sort_by(|a, b| b.round.cmp(&a.round).then_with(|| a.time_secs.cmp(&b.time_secs)));
    entries.truncate(limit);
    entries
}

fn recent_mode_records(records: &[ModeRecord], limit: usize) -> Vec<ModeRecord> {
    records.iter().rev().take(limit).cloned().collect()
}

fn recent_infinite_records(records: &[InfiniteRecord], limit: usize) -> Vec<InfiniteRecord> {
    records.iter().rev().take(limit).cloned().collect()
}

fn build_empty_records_status() -> adw::StatusPage {
    adw::StatusPage::builder()
        .title(tr("No scores yet"))
        .description(tr("Finish a run to populate this section"))
        .icon_name("view-list-symbolic")
        .build()
}

fn build_mode_group(title: &str, entries: &[ModeRecord]) -> adw::PreferencesGroup {
    let group = adw::PreferencesGroup::new();
    group.set_title(title);

    for entry in entries {
        let row = adw::ActionRow::builder()
            .title(tr(classic_level_name(entry.level)))
            .subtitle(format!("{} {}%", tr("Precision"), entry.precision_pct))
            .build();
        row.set_activatable(false);
        row.add_suffix(&time_suffix_label(&format_mm_ss(entry.time_secs)));
        row.add_suffix(&rank_suffix_label(entry.rank.as_str()));
        group.add(&row);
    }

    group
}

fn build_infinite_group(title: &str, entries: &[InfiniteRecord]) -> adw::PreferencesGroup {
    let group = adw::PreferencesGroup::new();
    group.set_title(title);

    for entry in entries {
        let milestone = format!(
            "{} x{}",
            tr(infinite::level_name(entry.segment_level)),
            entry.segment_survival
        );
        let row = adw::ActionRow::builder()
            .title(format!("{} {}", tr("Round"), entry.round))
            .subtitle(format!("{} {}", tr("Milestone"), milestone))
            .build();
        row.set_activatable(false);
        row.add_suffix(&time_suffix_label(&format_mm_ss(entry.time_secs)));
        group.add(&row);
    }

    group
}

fn build_records_page_shell() -> gtk::Box {
    let page = gtk::Box::new(gtk::Orientation::Vertical, 18);
    page.add_css_class("score-list-page");
    page.set_hexpand(true);
    page.set_vexpand(true);
    page.set_halign(gtk::Align::Fill);
    page.set_valign(gtk::Align::Fill);
    page
}

fn wrap_records_page(content: &impl IsA<gtk::Widget>) -> gtk::ScrolledWindow {
    let clamp = adw::Clamp::new();
    clamp.set_maximum_size(560);
    clamp.set_tightening_threshold(360);
    clamp.set_child(Some(content));

    let scroller = gtk::ScrolledWindow::new();
    scroller.set_hscrollbar_policy(gtk::PolicyType::Never);
    scroller.set_min_content_height(280);
    scroller.set_vexpand(true);
    scroller.set_child(Some(&clamp));
    scroller
}

fn build_precision_tab(records: &[ModeRecord]) -> gtk::ScrolledWindow {
    let page = build_records_page_shell();
    let top_entries = {
        let mut rows = records.to_vec();
        sort_mode_records(&mut rows);
        rows.truncate(3);
        rows
    };
    let recent_entries = recent_mode_records(records, 10);

    if top_entries.is_empty() && recent_entries.is_empty() {
        page.append(&build_empty_records_status());
    } else {
        if !top_entries.is_empty() {
            page.append(&build_mode_group(
                &tr("Best runs"),
                &top_entries,
            ));
        }
        if !recent_entries.is_empty() {
            page.append(&build_mode_group(
                &tr("Recent runs"),
                &recent_entries,
            ));
        }
    }

    wrap_records_page(&page)
}

fn build_infinite_tab(records: &[InfiniteRecord]) -> gtk::ScrolledWindow {
    let page = build_records_page_shell();
    let top_entries = top_infinite_records(records, 3);
    let recent_entries = recent_infinite_records(records, 10);

    if top_entries.is_empty() && recent_entries.is_empty() {
        page.append(&build_empty_records_status());
    } else {
        if !top_entries.is_empty() {
            page.append(&build_infinite_group(
                &tr("Best runs"),
                &top_entries,
            ));
        }
        if !recent_entries.is_empty() {
            page.append(&build_infinite_group(
                &tr("Recent runs"),
                &recent_entries,
            ));
        }
    }

    wrap_records_page(&page)
}

pub fn register_non_infinite_result(st: &mut AppState) {
    let attempts = st.run_matches.saturating_add(st.run_mismatches);
    let precision_pct = if attempts == 0 {
        100
    } else {
        ((st.run_matches as f64 / attempts as f64) * 100.0).round() as u8
    };
    let level = if st.difficulty == Difficulty::Trio {
        st.trio_level
    } else {
        match st.difficulty {
            Difficulty::Easy => 1,
            Difficulty::Medium => 2,
            Difficulty::Hard => 3,
            Difficulty::Impossible => 4,
            _ => 1,
        }
    };
    let rank = rank_for_precision(level, precision_pct);
    let best_candidate = ModeRecord {
        level,
        time_secs: st.seconds_elapsed,
        precision_pct,
        rank,
        date_label: now_date_label(),
    };
    if st.difficulty == Difficulty::Trio {
        st.records.trio.push(best_candidate);
        let overflow = st.records.trio.len().saturating_sub(MODE_HISTORY_LIMIT);
        if overflow > 0 {
            st.records.trio.drain(0..overflow);
        }
    } else {
        st.records.classic.push(best_candidate);
        let overflow = st.records.classic.len().saturating_sub(MODE_HISTORY_LIMIT);
        if overflow > 0 {
            st.records.classic.drain(0..overflow);
        }
    }
    if let Err(err) = save_records(&st.records) {
        eprintln!("warning: failed to save records: {err}");
    }

    st.victory_title_text = match rank {
        Rank::S => tr("Flawless Memory!"),
        Rank::A => tr("Sharp Mind!"),
        Rank::B => tr("Keep the Momentum!"),
        Rank::C => tr("Growing Strong!"),
    };
    st.victory_message_text = if st.difficulty == Difficulty::Trio {
        format!("{} {} {}", tr("Trio"), tr(classic_level_name(level)), tr("completed"))
    } else {
        format!("{} {} {}", tr("Classic"), tr(classic_level_name(level)), tr("completed"))
    };
    st.victory_stats_text = format!(
        "{}: {}\n{}: {}%\n{}: {}",
        tr("Time"),
        format_mm_ss(st.seconds_elapsed),
        tr("Precision"),
        precision_pct,
        tr("Harmony"),
        rank.as_str()
    );
    st.victory_rank = rank;
    st.victory_art_resource = None;
}

pub fn register_infinite_run_result(st: &mut AppState) {
    let round = st.infinite_round;
    let segment = infinite::classic_difficulty_for_round(round);
    let segment_level = match segment {
        Difficulty::Easy => 1,
        Difficulty::Medium => 2,
        Difficulty::Hard => 3,
        Difficulty::Impossible => 4,
        _ => 1,
    };
    let segment_survival = if segment == Difficulty::Impossible {
        infinite::expert_survival_rounds(round)
    } else if segment == Difficulty::Hard {
        infinite::hard_survival_rounds(round)
    } else {
        round
    };
    let candidate = InfiniteRecord {
        round,
        segment_level,
        segment_survival,
        time_secs: st.seconds_elapsed,
        date_label: now_date_label(),
    };
    st.records.infinite.push(candidate);
    let overflow = st.records.infinite.len().saturating_sub(INFINITE_HISTORY_LIMIT);
    if overflow > 0 {
        st.records.infinite.drain(0..overflow);
    }
    if let Err(err) = save_records(&st.records) {
        eprintln!("warning: failed to save records: {err}");
    }
}

pub fn reset_local_records(state: &Rc<RefCell<AppState>>) {
    let mut st = state.borrow_mut();
    st.records = PlayerRecords::default();
    if let Err(err) = save_records(&st.records) {
        eprintln!("warning: failed to reset local records: {err}");
    }
}

pub fn show_memory_dialog(state: &Rc<RefCell<AppState>>, app: &adw::Application) -> adw::Dialog {
    let parent_window = app.active_window();
    let dialog = adw::Dialog::new();
    dialog.set_can_close(true);
    dialog.set_content_width(520);
    dialog.set_content_height(420);

    let title = gtk::Label::new(Some(&tr("Local Score")));
    title.add_css_class("game-title-main");
    title.set_halign(gtk::Align::Center);

    let header = adw::HeaderBar::new();
    header.set_title_widget(Some(&title));
    header.set_show_end_title_buttons(true);

    let content = gtk::Box::new(gtk::Orientation::Vertical, 8);
    content.set_margin_top(10);
    content.set_margin_bottom(10);
    content.set_margin_start(10);
    content.set_margin_end(10);
    content.add_css_class("memory-dialog-content");
    content.set_halign(gtk::Align::Fill);
    content.set_vexpand(true);

    let (classic_records, trio_records, infinite_records) = {
        let st = state.borrow();
        (
            st.records.classic.clone(),
            st.records.trio.clone(),
            st.records.infinite.clone(),
        )
    };

    let mode_switcher = gtk::StackSwitcher::new();
    mode_switcher.set_halign(gtk::Align::Center);
    mode_switcher.add_css_class("score-mode-switcher");
    let mode_stack = gtk::Stack::new();
    mode_stack.set_halign(gtk::Align::Fill);
    mode_stack.set_hexpand(true);
    mode_stack.set_vexpand(true);
    mode_stack.set_transition_type(gtk::StackTransitionType::SlideLeftRight);
    mode_stack.set_transition_duration(180);
    mode_switcher.set_stack(Some(&mode_stack));

    let classic_tab = build_precision_tab(&classic_records);
    mode_stack.add_titled(&classic_tab, Some("score-classic"), &tr("Classic"));
    let trio_tab = build_precision_tab(&trio_records);
    mode_stack.add_titled(&trio_tab, Some("score-trio"), &tr("Trio"));
    let infinite_tab = build_infinite_tab(&infinite_records);
    mode_stack.add_titled(&infinite_tab, Some("score-infinite"), &tr("Infinite"));

    content.append(&mode_switcher);
    content.append(&mode_stack);

    let toolbar = adw::ToolbarView::new();
    toolbar.add_top_bar(&header);
    toolbar.set_content(Some(&content));

    dialog.set_child(Some(&toolbar));
    dialog.present(parent_window.as_ref());
    dialog
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mode_record(level: u8, time_secs: u32, precision_pct: u8, rank: Rank, date: &str) -> ModeRecord {
        ModeRecord {
            level,
            time_secs,
            precision_pct,
            rank,
            date_label: date.to_string(),
        }
    }

    fn infinite_record(round: u32, segment_level: u8, segment_survival: u32, time_secs: u32, date: &str) -> InfiniteRecord {
        InfiniteRecord {
            round,
            segment_level,
            segment_survival,
            time_secs,
            date_label: date.to_string(),
        }
    }

    #[test]
    fn json_roundtrip_preserves_records_content() {
        let records = PlayerRecords {
            classic: vec![mode_record(2, 70, 92, Rank::A, "2026-03-01 10:00")],
            trio: vec![mode_record(4, 130, 87, Rank::B, "2026-03-01 10:05")],
            infinite: vec![infinite_record(11, 4, 1, 220, "2026-03-01 10:10")],
        };

        let raw = serialize_json_records(&records);
        let parsed = load_json_records(&raw).expect("serialized records should parse");

        assert_eq!(parsed.classic.len(), 1);
        assert_eq!(parsed.trio.len(), 1);
        assert_eq!(parsed.infinite.len(), 1);

        let classic = &parsed.classic[0];
        assert_eq!(classic.level, 2);
        assert_eq!(classic.time_secs, 70);
        assert_eq!(classic.precision_pct, 92);
        assert!(classic.rank == Rank::A);
        assert_eq!(classic.date_label, "2026-03-01 10:00");

        let trio = &parsed.trio[0];
        assert_eq!(trio.level, 4);
        assert_eq!(trio.time_secs, 130);
        assert_eq!(trio.precision_pct, 87);
        assert!(trio.rank == Rank::B);

        let infinite = &parsed.infinite[0];
        assert_eq!(infinite.round, 11);
        assert_eq!(infinite.segment_level, 4);
        assert_eq!(infinite.segment_survival, 1);
        assert_eq!(infinite.time_secs, 220);
    }

    #[test]
    fn json_loader_accepts_generic_pretty_json() {
        let raw = r#"{
  "classic":[
    {
      "level": 2,
      "time_secs": 70,
      "precision_pct": 92,
      "rank": "A",
      "date_label": "2026-03-01 10:00"
    }
  ],
  "trio": [],
  "infinite": []
}"#;

        let parsed = load_json_records(raw).expect("pretty json should parse");
        assert_eq!(parsed.classic.len(), 1);
        assert_eq!(parsed.classic[0].level, 2);
        assert!(parsed.classic[0].rank == Rank::A);
    }

    #[test]
    fn legacy_loader_accepts_trio_key() {
        let raw = "\
tri=3|A|95|90
classic=1|B|110|80
infinite=7|3|1|300
";
        let parsed = load_legacy_records(raw);
        assert_eq!(parsed.trio.len(), 1);
        assert_eq!(parsed.classic.len(), 1);
        assert_eq!(parsed.infinite.len(), 1);
        assert_eq!(parsed.trio[0].level, 3);
        assert!(parsed.trio[0].rank == Rank::A);
    }

}
