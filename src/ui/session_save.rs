use std::fs;
use std::path::PathBuf;

use super::state::{AppState, Difficulty, Tile, TileStatus};

const SAVE_FILE_NAME: &str = "last_run.v1";
const SAVE_VERSION: u8 = 1;

#[derive(Clone)]
pub struct SavedRun {
    pub difficulty: Difficulty,
    pub tri_level: u8,
    pub recall_level: u8,
    pub infinite_round: u32,
    pub seconds_elapsed: u32,
    pub run_mismatches: u32,
    pub run_matches: u32,
    pub impossible_mismatch_count: u8,
    pub impossible_punish_stage: u8,
    pub impossible_last_first_index: Option<usize>,
    pub impossible_same_first_streak: u8,
    pub flipped_indices: Vec<usize>,
    pub tiles: Vec<Tile>,
}

fn save_path() -> Option<PathBuf> {
    let home = std::env::var("HOME").ok()?;
    Some(PathBuf::from(home).join(".config/recall").join(SAVE_FILE_NAME))
}

fn difficulty_to_code(difficulty: Difficulty) -> &'static str {
    match difficulty {
        Difficulty::Easy => "easy",
        Difficulty::Medium => "medium",
        Difficulty::Hard => "hard",
        Difficulty::Impossible => "impossible",
        Difficulty::Tri => "tri",
        Difficulty::RecallMode => "recall",
    }
}

fn difficulty_from_code(code: &str) -> Option<Difficulty> {
    match code {
        "easy" => Some(Difficulty::Easy),
        "medium" => Some(Difficulty::Medium),
        "hard" => Some(Difficulty::Hard),
        "impossible" => Some(Difficulty::Impossible),
        "tri" => Some(Difficulty::Tri),
        "recall" => Some(Difficulty::RecallMode),
        _ => None,
    }
}

fn escape_value(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    for ch in raw.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '|' => out.push_str("\\|"),
            _ => out.push(ch),
        }
    }
    out
}

fn unescape_value(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    let mut chars = raw.chars();
    while let Some(ch) = chars.next() {
        if ch != '\\' {
            out.push(ch);
            continue;
        }
        match chars.next() {
            Some('n') => out.push('\n'),
            Some('|') => out.push('|'),
            Some('\\') => out.push('\\'),
            Some(other) => {
                out.push('\\');
                out.push(other);
            }
            None => out.push('\\'),
        }
    }
    out
}

fn split_escaped_pair(raw: &str) -> Option<(String, String)> {
    let mut escaped = false;
    let mut split_at = None;
    for (idx, ch) in raw.char_indices() {
        if escaped {
            escaped = false;
            continue;
        }
        if ch == '\\' {
            escaped = true;
            continue;
        }
        if ch == '|' {
            split_at = Some(idx);
            break;
        }
    }
    let split_at = split_at?;
    let (left, right_with_sep) = raw.split_at(split_at);
    let right = right_with_sep.strip_prefix('|')?;
    Some((left.to_string(), right.to_string()))
}

fn encode_tile(tile: &Tile) -> String {
    let status = match tile.status {
        TileStatus::Hidden => 'H',
        TileStatus::Flipped => 'F',
        TileStatus::Matched => 'M',
    };
    format!("{}|{}", status, escape_value(&tile.value))
}

fn parse_tile(raw: &str) -> Option<Tile> {
    let (status_code, value_code) = split_escaped_pair(raw)?;
    let mut status_chars = status_code.chars();
    let status = match status_chars.next()? {
        'H' => TileStatus::Hidden,
        'F' => TileStatus::Flipped,
        'M' => TileStatus::Matched,
        _ => return None,
    };
    if status_chars.next().is_some() {
        return None;
    }
    Some(Tile {
        status,
        value: unescape_value(&value_code),
    })
}

fn serialize_saved_run(run: &SavedRun) -> String {
    let mut out = String::new();
    out.push_str(&format!("version={}\n", SAVE_VERSION));
    out.push_str("started=1\n");
    out.push_str(&format!("difficulty={}\n", difficulty_to_code(run.difficulty)));
    out.push_str(&format!("tri_level={}\n", run.tri_level));
    out.push_str(&format!("recall_level={}\n", run.recall_level));
    out.push_str(&format!("infinite_round={}\n", run.infinite_round));
    out.push_str(&format!("seconds_elapsed={}\n", run.seconds_elapsed));
    out.push_str(&format!("run_mismatches={}\n", run.run_mismatches));
    out.push_str(&format!("run_matches={}\n", run.run_matches));
    out.push_str(&format!(
        "impossible_mismatch_count={}\n",
        run.impossible_mismatch_count
    ));
    out.push_str(&format!(
        "impossible_punish_stage={}\n",
        run.impossible_punish_stage
    ));
    out.push_str(&format!(
        "impossible_last_first_index={}\n",
        run.impossible_last_first_index
            .map(|value| value.to_string())
            .unwrap_or_else(|| "-".to_string())
    ));
    out.push_str(&format!(
        "impossible_same_first_streak={}\n",
        run.impossible_same_first_streak
    ));
    let flipped_text = run
        .flipped_indices
        .iter()
        .map(|idx| idx.to_string())
        .collect::<Vec<String>>()
        .join(",");
    out.push_str(&format!("flipped_indices={}\n", flipped_text));
    for tile in &run.tiles {
        out.push_str("tile=");
        out.push_str(&encode_tile(tile));
        out.push('\n');
    }
    out
}

fn parse_saved_run(raw: &str) -> Option<SavedRun> {
    let mut version = None;
    let mut started = false;
    let mut difficulty = None;
    let mut tri_level = 3u8;
    let mut recall_level = 2u8;
    let mut infinite_round = 1u32;
    let mut seconds_elapsed = 0u32;
    let mut run_mismatches = 0u32;
    let mut run_matches = 0u32;
    let mut impossible_mismatch_count = 0u8;
    let mut impossible_punish_stage = 0u8;
    let mut impossible_last_first_index = None;
    let mut impossible_same_first_streak = 0u8;
    let mut flipped_indices = Vec::new();
    let mut tiles = Vec::new();

    for line in raw.lines() {
        if let Some(rest) = line.strip_prefix("version=") {
            version = rest.parse::<u8>().ok();
            continue;
        }
        if let Some(rest) = line.strip_prefix("started=") {
            started = rest.trim() == "1";
            continue;
        }
        if let Some(rest) = line.strip_prefix("difficulty=") {
            difficulty = difficulty_from_code(rest.trim());
            continue;
        }
        if let Some(rest) = line.strip_prefix("tri_level=") {
            tri_level = rest.parse::<u8>().ok()?.clamp(1, 4);
            continue;
        }
        if let Some(rest) = line.strip_prefix("recall_level=") {
            recall_level = rest.parse::<u8>().ok()?.clamp(1, 4);
            continue;
        }
        if let Some(rest) = line.strip_prefix("infinite_round=") {
            infinite_round = rest.parse::<u32>().ok()?.max(1);
            continue;
        }
        if let Some(rest) = line.strip_prefix("seconds_elapsed=") {
            seconds_elapsed = rest.parse::<u32>().ok()?;
            continue;
        }
        if let Some(rest) = line.strip_prefix("run_mismatches=") {
            run_mismatches = rest.parse::<u32>().ok()?;
            continue;
        }
        if let Some(rest) = line.strip_prefix("run_matches=") {
            run_matches = rest.parse::<u32>().ok()?;
            continue;
        }
        if let Some(rest) = line.strip_prefix("impossible_mismatch_count=") {
            impossible_mismatch_count = rest.parse::<u8>().ok()?;
            continue;
        }
        if let Some(rest) = line.strip_prefix("impossible_punish_stage=") {
            impossible_punish_stage = rest.parse::<u8>().ok()?;
            continue;
        }
        if let Some(rest) = line.strip_prefix("impossible_last_first_index=") {
            impossible_last_first_index = if rest.trim() == "-" {
                None
            } else {
                Some(rest.parse::<usize>().ok()?)
            };
            continue;
        }
        if let Some(rest) = line.strip_prefix("impossible_same_first_streak=") {
            impossible_same_first_streak = rest.parse::<u8>().ok()?;
            continue;
        }
        if let Some(rest) = line.strip_prefix("flipped_indices=") {
            let trimmed = rest.trim();
            if trimmed.is_empty() {
                flipped_indices.clear();
            } else {
                flipped_indices = trimmed
                    .split(',')
                    .map(str::trim)
                    .filter(|part| !part.is_empty())
                    .map(|part| part.parse::<usize>().ok())
                    .collect::<Option<Vec<usize>>>()?;
            }
            continue;
        }
        if let Some(rest) = line.strip_prefix("tile=") {
            tiles.push(parse_tile(rest)?);
        }
    }

    if version != Some(SAVE_VERSION) || !started {
        return None;
    }

    Some(SavedRun {
        difficulty: difficulty?,
        tri_level,
        recall_level,
        infinite_round,
        seconds_elapsed,
        run_mismatches,
        run_matches,
        impossible_mismatch_count,
        impossible_punish_stage,
        impossible_last_first_index,
        impossible_same_first_streak,
        flipped_indices,
        tiles,
    })
}

fn write_atomic(path: &PathBuf, data: &str) {
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let tmp_path = path.with_extension("tmp");
    if fs::write(&tmp_path, data).is_ok() {
        let _ = fs::rename(&tmp_path, path);
    }
}

pub fn load_saved_run() -> Option<SavedRun> {
    let path = save_path()?;
    let raw = fs::read_to_string(path).ok()?;
    parse_saved_run(&raw)
}

pub fn has_saved_run() -> bool {
    load_saved_run().is_some()
}

pub fn clear_saved_run() {
    if let Some(path) = save_path() {
        let _ = fs::remove_file(path);
    }
}

pub fn save_current_run(st: &AppState) {
    if !st.active_session_started || st.tiles.is_empty() {
        return;
    }

    let run = SavedRun {
        difficulty: st.difficulty,
        tri_level: st.tri_level,
        recall_level: st.recall_level,
        infinite_round: st.infinite_round,
        seconds_elapsed: st.seconds_elapsed,
        run_mismatches: st.run_mismatches,
        run_matches: st.run_matches,
        impossible_mismatch_count: st.impossible_mismatch_count,
        impossible_punish_stage: st.impossible_punish_stage,
        impossible_last_first_index: st.impossible_last_first_index,
        impossible_same_first_streak: st.impossible_same_first_streak,
        flipped_indices: st.flipped_indices.clone(),
        tiles: st.tiles.clone(),
    };

    if let Some(path) = save_path() {
        write_atomic(&path, &serialize_saved_run(&run));
    }
}
