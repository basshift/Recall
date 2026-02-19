use std::cell::RefCell;
use std::rc::Rc;
use std::{fs, path::PathBuf};

use gtk4 as gtk;
use gtk4::glib;
use gtk4::prelude::*;
use libadwaita as adw;
use adw::prelude::*;

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
        2 => "Normal",
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

fn encode_mode_record(record: &ModeRecord) -> String {
    format!(
        "{}|{}|{}|{}|{}",
        record.level,
        record.rank.as_str(),
        record.time_secs,
        record.precision_pct,
        record.date_label.replace('\n', " ")
    )
}

fn encode_infinite_record(record: &InfiniteRecord) -> String {
    format!(
        "{}|{}|{}|{}|{}",
        record.round,
        record.segment_level,
        record.segment_survival,
        record.time_secs,
        record.date_label.replace('\n', " ")
    )
}

fn json_escape(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for ch in value.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            _ => out.push(ch),
        }
    }
    out
}

fn json_unescape(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    let mut chars = value.chars();
    while let Some(ch) = chars.next() {
        if ch != '\\' {
            out.push(ch);
            continue;
        }
        match chars.next() {
            Some('n') => out.push('\n'),
            Some('r') => out.push('\r'),
            Some('t') => out.push('\t'),
            Some('\\') => out.push('\\'),
            Some('"') => out.push('"'),
            Some(other) => {
                out.push('\\');
                out.push(other);
            }
            None => out.push('\\'),
        }
    }
    out
}

fn parse_json_entry_line(raw: &str) -> Option<String> {
    let mut line = raw.trim();
    if line.ends_with(',') {
        line = &line[..line.len().saturating_sub(1)];
    }
    if !line.starts_with('"') || !line.ends_with('"') || line.len() < 2 {
        return None;
    }
    Some(json_unescape(&line[1..line.len() - 1]))
}

fn table_cell(text: &str, class_name: &str, width_chars: i32) -> gtk::Label {
    let label = gtk::Label::new(Some(text));
    label.add_css_class(class_name);
    label.add_css_class("body");
    label.set_halign(gtk::Align::Fill);
    label.set_hexpand(true);
    label.set_xalign(0.5);
    if width_chars > 0 {
        label.set_width_chars(width_chars);
    }
    label
}

fn section_title(text: &str) -> gtk::Label {
    let label = gtk::Label::new(Some(text));
    label.add_css_class("score-section-title");
    label.add_css_class("heading");
    label.set_halign(gtk::Align::Center);
    label.set_xalign(0.5);
    label
}

fn now_date_label() -> String {
    if let Ok(dt) = glib::DateTime::now_local()
        && let Ok(text) = dt.format("%Y-%m-%d %H:%M")
    {
        return text.to_string();
    }
    "Unknown date".to_string()
}

fn load_legacy_records(raw: &str) -> PlayerRecords {
    let mut records = PlayerRecords::default();
    for line in raw.lines() {
        if let Some(rest) = line.strip_prefix("classic_entry=") {
            if let Some(entry) = parse_mode_record(rest) {
                records.classic.push(entry);
            }
        } else if let Some(rest) = line.strip_prefix("tri_entry=") {
            if let Some(entry) = parse_mode_record(rest) {
                records.tri.push(entry);
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
                records.tri.push(entry);
            }
        } else if let Some(rest) = line.strip_prefix("infinite=")
            && let Some(entry) = parse_legacy_infinite_best(rest)
        {
            records.infinite.push(entry);
        }
    }
    records
}

fn load_json_records(raw: &str) -> PlayerRecords {
    #[derive(Clone, Copy)]
    enum Section {
        Classic,
        Tri,
        Infinite,
    }

    let mut section: Option<Section> = None;
    let mut records = PlayerRecords::default();

    for line in raw.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("\"classic\"") {
            section = Some(Section::Classic);
            continue;
        }
        if trimmed.starts_with("\"tri\"") {
            section = Some(Section::Tri);
            continue;
        }
        if trimmed.starts_with("\"infinite\"") {
            section = Some(Section::Infinite);
            continue;
        }
        if trimmed.starts_with(']') {
            section = None;
            continue;
        }

        let Some(active_section) = section else {
            continue;
        };
        let Some(entry_line) = parse_json_entry_line(trimmed) else {
            continue;
        };
        match active_section {
            Section::Classic => {
                if let Some(entry) = parse_mode_record(&entry_line) {
                    records.classic.push(entry);
                }
            }
            Section::Tri => {
                if let Some(entry) = parse_mode_record(&entry_line) {
                    records.tri.push(entry);
                }
            }
            Section::Infinite => {
                if let Some(entry) = parse_infinite_record(&entry_line) {
                    records.infinite.push(entry);
                }
            }
        }
    }

    records
}

fn serialize_legacy_records(records: &PlayerRecords) -> String {
    let mut out = String::new();
    for entry in &records.classic {
        out.push_str("classic_entry=");
        out.push_str(&encode_mode_record(entry));
        out.push('\n');
    }
    for entry in &records.tri {
        out.push_str("tri_entry=");
        out.push_str(&encode_mode_record(entry));
        out.push('\n');
    }
    for entry in &records.infinite {
        out.push_str("infinite_entry=");
        out.push_str(&encode_infinite_record(entry));
        out.push('\n');
    }
    out
}

fn serialize_json_records(records: &PlayerRecords) -> String {
    let mut out = String::new();
    out.push_str("{\n");
    out.push_str("  \"classic\": [\n");
    for (idx, entry) in records.classic.iter().enumerate() {
        let suffix = if idx + 1 == records.classic.len() { "" } else { "," };
        out.push_str("    \"");
        out.push_str(&json_escape(&encode_mode_record(entry)));
        out.push('"');
        out.push_str(suffix);
        out.push('\n');
    }
    out.push_str("  ],\n");
    out.push_str("  \"tri\": [\n");
    for (idx, entry) in records.tri.iter().enumerate() {
        let suffix = if idx + 1 == records.tri.len() { "" } else { "," };
        out.push_str("    \"");
        out.push_str(&json_escape(&encode_mode_record(entry)));
        out.push('"');
        out.push_str(suffix);
        out.push('\n');
    }
    out.push_str("  ],\n");
    out.push_str("  \"infinite\": [\n");
    for (idx, entry) in records.infinite.iter().enumerate() {
        let suffix = if idx + 1 == records.infinite.len() { "" } else { "," };
        out.push_str("    \"");
        out.push_str(&json_escape(&encode_infinite_record(entry)));
        out.push('"');
        out.push_str(suffix);
        out.push('\n');
    }
    out.push_str("  ]\n");
    out.push_str("}\n");
    out
}

fn ensure_seed_records(records: &mut PlayerRecords) {
    if !records.classic.is_empty() || !records.tri.is_empty() || !records.infinite.is_empty() {
        return;
    }
    records.classic = vec![
        ModeRecord { level: 2, time_secs: 72, precision_pct: 100, rank: Rank::S, date_label: "2026-02-11 20:31".to_string() },
        ModeRecord { level: 4, time_secs: 171, precision_pct: 91, rank: Rank::A, date_label: "2026-02-13 22:17".to_string() },
        ModeRecord { level: 3, time_secs: 114, precision_pct: 87, rank: Rank::B, date_label: "2026-02-14 19:06".to_string() },
    ];
    records.tri = vec![
        ModeRecord { level: 2, time_secs: 129, precision_pct: 95, rank: Rank::A, date_label: "2026-02-12 18:44".to_string() },
        ModeRecord { level: 3, time_secs: 205, precision_pct: 89, rank: Rank::B, date_label: "2026-02-14 21:52".to_string() },
        ModeRecord { level: 4, time_secs: 284, precision_pct: 83, rank: Rank::B, date_label: "2026-02-15 00:09".to_string() },
    ];
    records.infinite = vec![
        InfiniteRecord { round: 16, segment_level: 4, segment_survival: 6, time_secs: 780, date_label: "2026-02-13 23:10".to_string() },
        InfiniteRecord { round: 13, segment_level: 4, segment_survival: 3, time_secs: 598, date_label: "2026-02-14 20:26".to_string() },
        InfiniteRecord { round: 10, segment_level: 3, segment_survival: 4, time_secs: 470, date_label: "2026-02-12 22:02".to_string() },
    ];
}

pub fn load_records() -> PlayerRecords {
    let mut records = PlayerRecords::default();

    if let Some(path) = records_path()
        && let Ok(raw) = fs::read_to_string(path)
    {
        records = load_json_records(&raw);
    } else if let Some(path) = legacy_records_path()
        && let Ok(raw) = fs::read_to_string(path)
    {
        records = load_legacy_records(&raw);
    }

    ensure_seed_records(&mut records);
    save_records(&records);
    records
}

fn save_records(records: &PlayerRecords) {
    if let Some(path) = legacy_records_path() {
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let _ = fs::write(path, serialize_legacy_records(records));
    }
    if let Some(path) = records_path() {
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let _ = fs::write(path, serialize_json_records(records));
    }
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

fn build_mode_grid(entries: &[ModeRecord], target_rows: usize) -> gtk::Grid {
    let grid = gtk::Grid::new();
    grid.set_halign(gtk::Align::Fill);
    grid.set_hexpand(true);
    grid.set_column_homogeneous(true);
    grid.set_column_spacing(10);
    grid.set_row_spacing(5);
    grid.attach(&table_cell("Level", "score-table-head", 7), 0, 0, 1, 1);
    grid.attach(&table_cell("Time", "score-table-head", 6), 1, 0, 1, 1);
    grid.attach(&table_cell("Harmony", "score-table-head", 7), 2, 0, 1, 1);

    for idx in 0..target_rows {
        let row = (idx + 1) as i32;
        let (level_text, time_text, rank_text) = if let Some(entry) = entries.get(idx) {
            (
                classic_level_name(entry.level).to_string(),
                format_mm_ss(entry.time_secs),
                entry.rank.as_str().to_string(),
            )
        } else {
            ("---".to_string(), "---".to_string(), "---".to_string())
        };
        grid.attach(
            &table_cell(&level_text, "score-table-row", 7),
            0,
            row,
            1,
            1,
        );
        grid.attach(
            &table_cell(&time_text, "score-table-row", 6),
            1,
            row,
            1,
            1,
        );
        grid.attach(
            &table_cell(&rank_text, "score-table-row", 7),
            2,
            row,
            1,
            1,
        );
    }

    grid
}

fn build_infinite_grid(entries: &[InfiniteRecord], target_rows: usize) -> gtk::Grid {
    let grid = gtk::Grid::new();
    grid.set_halign(gtk::Align::Fill);
    grid.set_hexpand(true);
    grid.set_column_homogeneous(true);
    grid.set_column_spacing(10);
    grid.set_row_spacing(5);
    grid.attach(&table_cell("Round", "score-table-head", 6), 0, 0, 1, 1);
    grid.attach(&table_cell("Milestone", "score-table-head", 10), 1, 0, 1, 1);
    grid.attach(&table_cell("Time", "score-table-head", 6), 2, 0, 1, 1);

    for idx in 0..target_rows {
        let row = (idx + 1) as i32;
        let (round_text, milestone_text, time_text) = if let Some(entry) = entries.get(idx) {
            (
                entry.round.to_string(),
                format!(
                    "{} x{}",
                    infinite::level_name(entry.segment_level),
                    entry.segment_survival
                ),
                format_mm_ss(entry.time_secs),
            )
        } else {
            ("---".to_string(), "---".to_string(), "---".to_string())
        };
        grid.attach(
            &table_cell(&round_text, "score-table-row", 6),
            0,
            row,
            1,
            1,
        );
        grid.attach(
            &table_cell(&milestone_text, "score-table-row", 10),
            1,
            row,
            1,
            1,
        );
        grid.attach(
            &table_cell(&time_text, "score-table-row", 6),
            2,
            row,
            1,
            1,
        );
    }

    grid
}

fn build_precision_tab(mode_title: &str, icon: &str, records: &[ModeRecord]) -> gtk::Box {
    let tab = gtk::Box::new(gtk::Orientation::Vertical, 8);
    tab.set_hexpand(true);
    tab.set_halign(gtk::Align::Fill);
    let _ = (mode_title, icon);

    let list = gtk::Box::new(gtk::Orientation::Vertical, 4);
    list.add_css_class("score-list-page");
    list.set_hexpand(true);
    list.set_halign(gtk::Align::Fill);
    let top_entries = {
        let mut rows = records.to_vec();
        sort_mode_records(&mut rows);
        rows.truncate(3);
        rows
    };
    let recent_entries = recent_mode_records(records, 10);

    list.append(&section_title("TOP 3"));
    list.append(&build_mode_grid(&top_entries, 3));
    list.append(&section_title("LATEST 10"));
    list.append(&build_mode_grid(&recent_entries, 10));
    tab.append(&list);
    tab
}

fn build_infinite_tab(records: &[InfiniteRecord]) -> gtk::Box {
    let tab = gtk::Box::new(gtk::Orientation::Vertical, 8);
    tab.set_hexpand(true);
    tab.set_halign(gtk::Align::Fill);

    let list = gtk::Box::new(gtk::Orientation::Vertical, 4);
    list.add_css_class("score-list-page");
    list.set_hexpand(true);
    list.set_halign(gtk::Align::Fill);
    let top_entries = top_infinite_records(records, 3);
    let recent_entries = recent_infinite_records(records, 10);
    list.append(&section_title("TOP 3"));
    list.append(&build_infinite_grid(&top_entries, 3));
    list.append(&section_title("LATEST 10"));
    list.append(&build_infinite_grid(&recent_entries, 10));
    tab.append(&list);
    tab
}

pub fn register_non_infinite_result(st: &mut AppState) {
    let attempts = st.run_matches.saturating_add(st.run_mismatches);
    let precision_pct = if attempts == 0 {
        100
    } else {
        ((st.run_matches as f64 / attempts as f64) * 100.0).round() as u8
    };
    let level = if st.difficulty == Difficulty::Tri {
        st.tri_level
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
    if st.difficulty == Difficulty::Tri {
        st.records.tri.push(best_candidate);
        let overflow = st.records.tri.len().saturating_sub(MODE_HISTORY_LIMIT);
        if overflow > 0 {
            st.records.tri.drain(0..overflow);
        }
    } else {
        st.records.classic.push(best_candidate);
        let overflow = st.records.classic.len().saturating_sub(MODE_HISTORY_LIMIT);
        if overflow > 0 {
            st.records.classic.drain(0..overflow);
        }
    }
    save_records(&st.records);

    st.victory_title_text = match rank {
        Rank::S => "Flawless Memory!".to_string(),
        Rank::A => "Sharp Mind!".to_string(),
        Rank::B => "Keep the Momentum!".to_string(),
        Rank::C => "Growing Strong!".to_string(),
    };
    st.victory_message_text = if st.difficulty == Difficulty::Tri {
        format!("Tri {} completed", classic_level_name(level))
    } else {
        format!("Classic {} completed", classic_level_name(level))
    };
    st.victory_stats_text = format!(
        "Time: {}\nPrecision: {}%\nHarmony: {}",
        format_mm_ss(st.seconds_elapsed),
        precision_pct,
        rank.as_str()
    );
}

pub fn register_infinite_round_result(st: &mut AppState) {
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
    save_records(&st.records);
}

pub fn show_memory_dialog(state: &Rc<RefCell<AppState>>, app: &adw::Application) -> adw::Dialog {
    let parent_window = app.active_window();
    let dialog = adw::Dialog::new();
    dialog.set_can_close(true);

    let title = gtk::Label::new(Some("LOCAL SCORE"));
    title.add_css_class("game-title-main");
    title.set_halign(gtk::Align::Center);

    let header = adw::HeaderBar::new();
    header.set_title_widget(Some(&title));
    header.set_show_end_title_buttons(true);

    let share_button = gtk::MenuButton::builder()
        .icon_name("send-to-symbolic")
        .tooltip_text("Records options")
        .build();
    share_button.add_css_class("flat");

    let share_menu = gtk::Box::new(gtk::Orientation::Vertical, 4);
    share_menu.set_margin_top(6);
    share_menu.set_margin_bottom(6);
    share_menu.set_margin_start(6);
    share_menu.set_margin_end(6);

    let export_button = gtk::Button::with_label("Export records");
    export_button.set_halign(gtk::Align::Start);
    export_button.add_css_class("flat");

    let import_button = gtk::Button::with_label("Import records");
    import_button.set_halign(gtk::Align::Start);
    import_button.add_css_class("flat");

    {
        let dialog = dialog.clone();
        export_button.connect_clicked(move |_| {
            let alert = adw::AlertDialog::builder()
                .heading("Export records")
                .body("Export will be enabled in the next iteration.")
                .build();
            alert.add_response("ok", "OK");
            alert.present(Some(&dialog));
        });
    }

    {
        let dialog = dialog.clone();
        import_button.connect_clicked(move |_| {
            let alert = adw::AlertDialog::builder()
                .heading("Import records")
                .body("Import will be enabled in the next iteration.")
                .build();
            alert.add_response("ok", "OK");
            alert.present(Some(&dialog));
        });
    }

    share_menu.append(&export_button);
    share_menu.append(&import_button);
    let share_popover = gtk::Popover::new();
    share_popover.set_child(Some(&share_menu));
    share_button.set_popover(Some(&share_popover));
    header.pack_start(&share_button);

    let content = gtk::Box::new(gtk::Orientation::Vertical, 8);
    content.set_margin_top(10);
    content.set_margin_bottom(10);
    content.set_margin_start(10);
    content.set_margin_end(10);
    content.add_css_class("memory-dialog-content");
    content.set_halign(gtk::Align::Fill);

    let (classic_records, tri_records, infinite_records) = {
        let st = state.borrow();
        (
            st.records.classic.clone(),
            st.records.tri.clone(),
            st.records.infinite.clone(),
        )
    };

    let mode_switcher = gtk::StackSwitcher::new();
    mode_switcher.set_halign(gtk::Align::Center);
    mode_switcher.add_css_class("score-mode-switcher");
    let mode_stack = gtk::Stack::new();
    mode_stack.set_halign(gtk::Align::Fill);
    mode_stack.set_hexpand(true);
    mode_stack.set_transition_type(gtk::StackTransitionType::SlideLeftRight);
    mode_stack.set_transition_duration(180);
    mode_switcher.set_stack(Some(&mode_stack));

    let classic_tab = build_precision_tab("Classic", "◯", &classic_records);
    mode_stack.add_titled(&classic_tab, Some("score-classic"), "Classic");
    let tri_tab = build_precision_tab("Tri", "△", &tri_records);
    mode_stack.add_titled(&tri_tab, Some("score-tri"), "Tri");
    let infinite_tab = build_infinite_tab(&infinite_records);
    mode_stack.add_titled(&infinite_tab, Some("score-infinite"), "Infinite");

    content.append(&mode_switcher);
    content.append(&mode_stack);

    let toolbar = adw::ToolbarView::new();
    toolbar.add_top_bar(&header);
    toolbar.set_content(Some(&content));

    dialog.set_child(Some(&toolbar));
    dialog.present(parent_window.as_ref());
    dialog
}
