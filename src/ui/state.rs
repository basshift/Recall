use gtk4 as gtk;
use libadwaita as adw;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq)]
pub enum TileStatus {
    Hidden,
    Flipped,
    Matched,
}

#[derive(Clone, Debug)]
pub struct Tile {
    pub value: String,
    pub status: TileStatus,
}

#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub enum Difficulty {
    #[default]
    Easy,
    Medium,
    Hard,
    Impossible,
    Trio,
    Infinite,
}

impl Difficulty {
    pub fn fixed_config(self) -> Option<(i32, i32, usize)> {
        match self {
            Difficulty::Easy => Some((3, 4, 2)),
            Difficulty::Medium => Some((4, 6, 2)),
            Difficulty::Hard => Some((6, 7, 2)),
            Difficulty::Impossible => Some((6, 8, 2)),
            Difficulty::Trio | Difficulty::Infinite => None,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Difficulty::Easy => "Easy",
            Difficulty::Medium => "Medium",
            Difficulty::Hard => "Hard",
            Difficulty::Impossible => "Expert",
            Difficulty::Trio => "Trio",
            Difficulty::Infinite => "Infinite",
        }
    }
}

const SYMBOL_POOL: &[&str] = &[
    "🐶", "🐱", "🐭", "🐹", "🐰", "🦊", "🐻", "🐼", "🐨", "🐯", "🦁", "🐮", "🐷", "🐸",
    "🐵", "🐔", "🐦", "🐤", "🐣", "🦆", "🦅", "🐗", "🐴", "🦄", "🐝", "🪲", "🦋", "🐌",
    "🐞", "🐢", "🦎", "🐙", "🦑", "🦐", "🦞", "🦀", "🐠", "🐟", "🐡", "🐬", "🐳", "🦈",
    "🐊", "🦓", "🦒", "🐘", "🦛", "🦏", "🦬", "🐪", "🐫", "🦙", "🦘", "🦥", "🦦", "🦫",
    "🦭", "🦚", "🦜", "🪿", "🦢", "🦩", "🐐", "🐏", "🍏", "🍎", "🍐", "🍊", "🍋", "🍌",
    "🍉", "🍇", "🍓", "🫐", "🍒", "🍑", "🥭", "🍍", "🥥", "🥝", "🍅", "🥑", "🥕", "🌽",
    "🥔", "🍠", "🥦", "🥬", "🥒", "🌶️", "🫑", "🍆", "🍄", "🥜", "🫘", "🍞", "🥐", "🥨",
    "🧀", "🥚", "🍳", "🥞", "🧇", "🍔", "🍕", "🌮", "🌯", "🍜", "🍣", "⚽", "🏀", "🏈",
    "⚾", "🥎", "🎾", "🏐", "🏉", "🥏", "🎱", "🏓", "🏸", "🏒", "🏑", "🥍", "🏏", "🥊",
    "🥋", "⛳", "🏹", "🛹", "🛼", "🥌", "🚴", "🏊", "🤽", "🎨", "🖌️", "🖍️", "🧵", "🧶",
    "🧩", "♟️", "🎯", "🎲", "🃏", "🪁", "🎮", "🕹️", "🎧", "🎤", "🎸", "🎺", "🎷", "📷",
    "📸", "📱", "💻", "⌨️", "🖥️", "🖨️", "🔍", "🔬", "🔭", "⚙️", "🧰", "🔧", "🔨", "🪛",
    "🔩", "📚", "📓", "✏️", "🖊️", "📌", "📎", "🌞", "🌝", "🌎", "🧭", "🗺️", "🪐", "⭐",
    "☀️", "⛅", "🌈", "🌊", "💧", "🔥", "⛰️", "🗻", "🌋", "🏝️", "🏜️", "🏞️", "🌳", "🌴",
    "🌵", "🌱", "🍀", "🌿", "🌾", "🌷", "🌹", "🌺", "🌸", "🪻", "🪷", "🌻", "🚗", "🚕",
    "🚌", "🚎", "🏎️", "🚓", "🚑", "🚒", "🚜", "🚲", "🛵", "🚀",
];

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Default, Deserialize, Serialize)]
pub enum Rank {
    #[default]
    C,
    B,
    A,
    S,
}

impl Rank {
    pub fn as_str(self) -> &'static str {
        match self {
            Rank::S => "S",
            Rank::A => "A",
            Rank::B => "B",
            Rank::C => "C",
        }
    }

    pub fn from_str(value: &str) -> Option<Self> {
        match value.trim().to_ascii_uppercase().as_str() {
            "S" => Some(Rank::S),
            "A" => Some(Rank::A),
            "B" => Some(Rank::B),
            "C" => Some(Rank::C),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct ModeRecord {
    pub level: u8,
    pub time_secs: u32,
    pub precision_pct: u8,
    pub rank: Rank,
    pub date_label: String,
}

#[derive(Clone, Debug, Default)]
pub struct InfiniteRecord {
    pub round: u32,
    pub segment_level: u8,
    pub segment_survival: u32,
    pub time_secs: u32,
    pub date_label: String,
}

#[derive(Clone, Debug, Default)]
pub struct PlayerRecords {
    pub classic: Vec<ModeRecord>,
    pub trio: Vec<ModeRecord>,
    pub infinite: Vec<InfiniteRecord>,
}

pub struct AppState {
    pub view_stack: Option<gtk::Stack>,
    pub header: Option<adw::HeaderBar>,
    pub back_button: Option<gtk::Button>,
    pub menu_button: Option<gtk::MenuButton>,
    pub restart_button: Option<gtk::Button>,
    pub continue_button: Option<gtk::Button>,
    pub title_menu: Option<gtk::Label>,
    pub title_game: Option<gtk::Widget>,
    pub title_game_subtitle: Option<gtk::Label>,
    pub header_timer_label: Option<gtk::Label>,
    pub title_victory: Option<gtk::Widget>,
    pub victory_title_label: Option<gtk::Label>,
    pub victory_message_label: Option<gtk::Label>,
    pub victory_stats_label: Option<gtk::Label>,
    pub victory_rank_art: Option<gtk::Image>,
    pub victory_art_resource: Option<String>,
    pub victory_spark_layer: Option<gtk::Fixed>,
    pub board_container: Option<gtk::Box>,
    pub board_shell: Option<gtk::AspectFrame>,
    pub dynamic_css_provider: Option<gtk::CssProvider>,
    pub compact_layout: bool,

    // Game state
    pub tiles: Vec<Tile>,
    pub flipped_indices: Vec<usize>,
    pub grid_buttons: Vec<gtk::Button>,
    pub lock_input: bool,
    pub flip_anim_phase: bool,
    pub game_id: u64,
    pub grid_cols: i32,
    pub grid_rows: i32,
    pub match_size: usize,
    pub difficulty: Difficulty,
    pub trio_level: u8,
    pub infinite_level: u8,
    pub infinite_round: u32,
    pub impossible_mismatch_count: u8,
    pub impossible_punish_stage: u8,
    pub impossible_last_first_index: Option<usize>,
    pub impossible_same_first_streak: u8,
    pub preview_active: bool,
    pub preview_remaining_ms: u32,
    pub preview_handle: Option<glib::SourceId>,
    pub seconds_elapsed: u32,
    pub timer_handle: Option<glib::SourceId>,
    pub spark_timer_handle: Option<glib::SourceId>,
    pub run_mismatches: u32,
    pub run_matches: u32,
    pub active_session_started: bool,
    pub pending_new_game_selection: bool,
    pub victory_title_text: String,
    pub victory_message_text: String,
    pub victory_stats_text: String,
    pub victory_rank: Rank,
    pub records: PlayerRecords,
}

impl Default for AppState {
    fn default() -> Self {
        AppState {
            view_stack: None,
            header: None,
            back_button: None,
            menu_button: None,
            restart_button: None,
            continue_button: None,
            title_menu: None,
            title_game: None,
            title_game_subtitle: None,
            header_timer_label: None,
            title_victory: None,
            victory_title_label: None,
            victory_message_label: None,
            victory_stats_label: None,
            victory_rank_art: None,
            victory_art_resource: None,
            victory_spark_layer: None,
            board_container: None,
            board_shell: None,
            dynamic_css_provider: None,
            compact_layout: false,
            tiles: Vec::new(),
            flipped_indices: Vec::new(),
            grid_buttons: Vec::new(),
            lock_input: false,
            flip_anim_phase: false,
            game_id: 0,
            grid_cols: 0,
            grid_rows: 0,
            match_size: 2,
            difficulty: Difficulty::Easy,
            trio_level: 3,
            infinite_level: 2,
            infinite_round: 1,
            impossible_mismatch_count: 0,
            impossible_punish_stage: 0,
            impossible_last_first_index: None,
            impossible_same_first_streak: 0,
            preview_active: false,
            preview_remaining_ms: 0,
            preview_handle: None,
            seconds_elapsed: 0,
            timer_handle: None,
            spark_timer_handle: None,
            run_mismatches: 0,
            run_matches: 0,
            active_session_started: false,
            pending_new_game_selection: false,
            victory_title_text: String::new(),
            victory_message_text: String::new(),
            victory_stats_text: String::new(),
            victory_rank: Rank::C,
            records: PlayerRecords::default(),
        }
    }
}

impl AppState {
    fn apply_grid_config(&mut self, cols: i32, rows: i32, match_size: usize) {
        self.grid_cols = cols;
        self.grid_rows = rows;
        self.match_size = match_size;
    }

    fn config_for_current_difficulty(&self, difficulty: Difficulty) -> (i32, i32, usize) {
        match difficulty {
            Difficulty::Trio => Self::trio_config(self.trio_level),
            Difficulty::Infinite => Self::infinite_config(self.infinite_level),
            _ => difficulty
                .fixed_config()
                .expect("fixed config required for classic difficulties"),
        }
    }

    fn trio_config(level: u8) -> (i32, i32, usize) {
        match level.clamp(1, 4) {
            1 => (4, 6, 3),
            2 => (5, 6, 3),
            3 => (6, 7, 3),
            _ => (6, 8, 3),
        }
    }

    fn infinite_config(level: u8) -> (i32, i32, usize) {
        match level.clamp(1, 4) {
            1 => (3, 4, 2),
            2 => (4, 6, 2),
            3 => (6, 7, 2),
            _ => (6, 8, 2),
        }
    }

    pub fn new() -> Self {
        let mut st = Self::default();
        st.set_difficulty(Difficulty::Easy);
        st
    }

    pub fn set_difficulty(&mut self, difficulty: Difficulty) {
        self.difficulty = difficulty;
        self.reset_impossible_pressure();
        if difficulty == Difficulty::Infinite {
            self.infinite_round = 1;
        }
        let (cols, rows, match_size) = self.config_for_current_difficulty(difficulty);
        self.apply_grid_config(cols, rows, match_size);
        self.reset_game();
    }

    pub fn set_trio_level(&mut self, level: u8) {
        self.trio_level = level.clamp(1, 4);
        if self.difficulty == Difficulty::Trio {
            let (cols, rows, match_size) = Self::trio_config(self.trio_level);
            self.apply_grid_config(cols, rows, match_size);
            self.reset_game();
        }
    }

    pub fn set_infinite_level(&mut self, level: u8) {
        self.infinite_level = level.clamp(1, 4);
        if self.difficulty == Difficulty::Infinite {
            let (cols, rows, match_size) = Self::infinite_config(self.infinite_level);
            self.apply_grid_config(cols, rows, match_size);
            self.reset_game();
        }
    }

    pub fn apply_infinite_level_without_reset(&mut self, level: u8) {
        self.infinite_level = level.clamp(1, 4);
        let (cols, rows, match_size) = Self::infinite_config(self.infinite_level);
        self.apply_grid_config(cols, rows, match_size);
    }

    pub fn reset_infinite_round(&mut self) {
        self.infinite_round = 1;
    }

    pub fn advance_infinite_round(&mut self) {
        self.infinite_round = self.infinite_round.saturating_add(1);
    }

    pub fn invalidate_callbacks(&mut self) {
        self.game_id = self.game_id.wrapping_add(1);
    }

    pub fn reset_impossible_pressure(&mut self) {
        self.impossible_mismatch_count = 0;
        self.impossible_punish_stage = 0;
        self.impossible_last_first_index = None;
        self.impossible_same_first_streak = 0;
    }

    pub fn reshuffle_hidden_tiles(&mut self) {
        use rand::Rng;

        let mut hidden_indices = Vec::new();
        for (idx, tile) in self.tiles.iter().enumerate() {
            if tile.status == TileStatus::Hidden {
                hidden_indices.push(idx);
            }
        }

        if hidden_indices.len() < 2 {
            return;
        }

        let mut rng = rand::rng();
        for shuffle_end in (1..hidden_indices.len()).rev() {
            let swap_pos = rng.random_range(0..=shuffle_end);
            if swap_pos == shuffle_end {
                continue;
            }

            let left_index = hidden_indices[shuffle_end];
            let right_index = hidden_indices[swap_pos];
            let (first, second) = if left_index < right_index {
                let (left, right) = self.tiles.split_at_mut(right_index);
                (&mut left[left_index].value, &mut right[0].value)
            } else {
                let (left, right) = self.tiles.split_at_mut(left_index);
                (&mut right[0].value, &mut left[right_index].value)
            };
            std::mem::swap(first, second);
        }
    }

    pub fn reset_game(&mut self) {
        self.invalidate_callbacks();
        self.tiles.clear();
        self.flipped_indices.clear();
        self.lock_input = false;
        self.reset_impossible_pressure();
        if self.difficulty != Difficulty::Infinite || self.infinite_round <= 1 {
            self.run_mismatches = 0;
            self.run_matches = 0;
        }

        let total_tiles = (self.grid_cols * self.grid_rows) as usize;
        let remainder = total_tiles % self.match_size;
        assert_eq!(
            remainder,
            0,
            "grid config must divide evenly by match size"
        );
        let group_count = total_tiles / self.match_size;
        assert!(
            group_count <= SYMBOL_POOL.len(),
            "grid config requires more unique symbols than available"
        );

        use rand::seq::SliceRandom;
        let mut rng = rand::rng();
        let mut values = Vec::with_capacity(total_tiles);

        let mut symbol_pool = SYMBOL_POOL.to_vec();
        symbol_pool.shuffle(&mut rng);
        for symbol in symbol_pool.iter().take(group_count) {
            for _ in 0..self.match_size {
                values.push(symbol);
            }
        }

        values.shuffle(&mut rng);

        for value in values {
            self.tiles.push(Tile {
                status: TileStatus::Hidden,
                value: value.to_string(),
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{AppState, Difficulty};

    #[test]
    fn classic_difficulties_divide_evenly_by_match_size() {
        for difficulty in [
            Difficulty::Easy,
            Difficulty::Medium,
            Difficulty::Hard,
            Difficulty::Impossible,
        ] {
            let (cols, rows, match_size) = difficulty
                .fixed_config()
                .expect("classic difficulty should have fixed config");
            assert_eq!(((cols * rows) as usize) % match_size, 0);
        }
    }

    #[test]
    fn trio_levels_divide_evenly_by_match_size() {
        for level in 1..=4 {
            let (cols, rows, match_size) = AppState::trio_config(level);
            assert_eq!(((cols * rows) as usize) % match_size, 0);
        }
    }

    #[test]
    fn infinite_levels_divide_evenly_by_match_size() {
        for level in 1..=4 {
            let (cols, rows, match_size) = AppState::infinite_config(level);
            assert_eq!(((cols * rows) as usize) % match_size, 0);
        }
    }

    #[test]
    fn all_current_configs_fit_within_symbol_pool() {
        for difficulty in [
            Difficulty::Easy,
            Difficulty::Medium,
            Difficulty::Hard,
            Difficulty::Impossible,
        ] {
            let (cols, rows, match_size) = difficulty
                .fixed_config()
                .expect("classic difficulty should have fixed config");
            let group_count = (cols * rows) as usize / match_size;
            assert!(group_count <= super::SYMBOL_POOL.len());
        }

        for level in 1..=4 {
            let (cols, rows, match_size) = AppState::trio_config(level);
            let group_count = (cols * rows) as usize / match_size;
            assert!(group_count <= super::SYMBOL_POOL.len());
        }

        for level in 1..=4 {
            let (cols, rows, match_size) = AppState::infinite_config(level);
            let group_count = (cols * rows) as usize / match_size;
            assert!(group_count <= super::SYMBOL_POOL.len());
        }
    }
}
