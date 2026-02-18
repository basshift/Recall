use gtk4 as gtk;
use libadwaita as adw;

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
    Tri,
    RecallMode,
}

impl Difficulty {
    pub fn config(self) -> (i32, i32, usize) {
        match self {
            Difficulty::Easy => (3, 4, 2),
            Difficulty::Medium => (4, 6, 2),
            Difficulty::Hard => (6, 7, 2),
            Difficulty::Impossible => (6, 8, 2),
            Difficulty::Tri => (6, 7, 3),
            Difficulty::RecallMode => (3, 4, 2),
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Difficulty::Easy => "Easy",
            Difficulty::Medium => "Normal",
            Difficulty::Hard => "Hard",
            Difficulty::Impossible => "Expert",
            Difficulty::Tri => "Tri",
            Difficulty::RecallMode => "Infinite",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Default)]
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
    pub tri: Vec<ModeRecord>,
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
    pub title_victory: Option<gtk::Widget>,
    pub victory_title_label: Option<gtk::Label>,
    pub victory_message_label: Option<gtk::Label>,
    pub victory_stats_label: Option<gtk::Label>,
    pub victory_rank_art: Option<gtk::Image>,
    pub victory_spark_layer: Option<gtk::Fixed>,
    pub board_container: Option<gtk::Box>,
    pub dynamic_css_provider: Option<gtk::CssProvider>,

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
    pub tri_level: u8,
    pub recall_level: u8,
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
            title_victory: None,
            victory_title_label: None,
            victory_message_label: None,
            victory_stats_label: None,
            victory_rank_art: None,
            victory_spark_layer: None,
            board_container: None,
            dynamic_css_provider: None,
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
            tri_level: 3,
            recall_level: 2,
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
            victory_title_text: "Growing Strong!".to_string(),
            victory_message_text: String::new(),
            victory_stats_text: String::new(),
            records: PlayerRecords::default(),
        }
    }
}

impl AppState {
    fn tri_config(level: u8) -> (i32, i32, usize) {
        match level.clamp(1, 4) {
            1 => (4, 6, 3),
            2 => (5, 6, 3),
            3 => (6, 7, 3),
            _ => (6, 8, 3),
        }
    }

    fn recall_config(level: u8) -> (i32, i32, usize) {
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
        self.impossible_mismatch_count = 0;
        self.impossible_punish_stage = 0;
        self.impossible_last_first_index = None;
        self.impossible_same_first_streak = 0;
        if difficulty == Difficulty::RecallMode {
            self.infinite_round = 1;
        }
        let (cols, rows, match_size) = match difficulty {
            Difficulty::Tri => Self::tri_config(self.tri_level),
            Difficulty::RecallMode => Self::recall_config(self.recall_level),
            _ => difficulty.config(),
        };
        self.grid_cols = cols;
        self.grid_rows = rows;
        self.match_size = match_size;
        self.reset_game();
    }

    pub fn set_tri_level(&mut self, level: u8) {
        self.tri_level = level.clamp(1, 4);
        if self.difficulty == Difficulty::Tri {
            let (cols, rows, match_size) = Self::tri_config(self.tri_level);
            self.grid_cols = cols;
            self.grid_rows = rows;
            self.match_size = match_size;
            self.reset_game();
        }
    }

    pub fn set_recall_level(&mut self, level: u8) {
        self.recall_level = level.clamp(1, 4);
        if self.difficulty == Difficulty::RecallMode {
            let (cols, rows, match_size) = Self::recall_config(self.recall_level);
            self.grid_cols = cols;
            self.grid_rows = rows;
            self.match_size = match_size;
            self.reset_game();
        }
    }

    pub fn apply_infinite_level_without_reset(&mut self, level: u8) {
        self.recall_level = level.clamp(1, 4);
        let (cols, rows, match_size) = Self::recall_config(self.recall_level);
        self.grid_cols = cols;
        self.grid_rows = rows;
        self.match_size = match_size;
    }

    pub fn reset_infinite_round(&mut self) {
        self.infinite_round = 1;
    }

    pub fn advance_infinite_round(&mut self) {
        self.infinite_round = self.infinite_round.saturating_add(1);
    }

    pub fn reset_impossible_pressure(&mut self) {
        self.impossible_mismatch_count = 0;
        self.impossible_punish_stage = 0;
        self.impossible_last_first_index = None;
        self.impossible_same_first_streak = 0;
    }

    pub fn reshuffle_hidden_tiles(&mut self) {
        use rand::seq::SliceRandom;
        let mut hidden_indices = Vec::new();
        let mut hidden_values = Vec::new();

        for (idx, tile) in self.tiles.iter().enumerate() {
            if tile.status == TileStatus::Hidden {
                hidden_indices.push(idx);
                hidden_values.push(tile.value.clone());
            }
        }

        if hidden_indices.len() < 2 {
            return;
        }

        let mut rng = rand::rng();
        hidden_values.shuffle(&mut rng);

        for (idx, value) in hidden_indices.into_iter().zip(hidden_values.into_iter()) {
            self.tiles[idx].value = value;
        }
    }

    pub fn reset_game(&mut self) {
        self.game_id = self.game_id.wrapping_add(1);
        self.tiles.clear();
        self.flipped_indices.clear();
        self.lock_input = false;
        self.reset_impossible_pressure();
        if self.difficulty != Difficulty::RecallMode || self.infinite_round <= 1 {
            self.run_mismatches = 0;
            self.run_matches = 0;
        }

        let total_tiles = (self.grid_cols * self.grid_rows) as usize;
        let group_count = total_tiles / self.match_size;
        let remainder = total_tiles % self.match_size;

        let symbols = [
            // Animals
            "🐶", "🐱", "🐭", "🐹", "🐰", "🦊", "🐻", "🐼", "🐨", "🐯", "🦁", "🐮", "🐷", "🐸", "🐵",
            "🦄", "🐝", "🦋", "🐌", "🐞", "🐢", "🐍", "🐙", "🦑", "🦐", "🦞", "🦀", "🐡", "🐠", "🐬",
            // Fruits
            "🍏", "🍎", "🍐", "🍊", "🍋", "🍌", "🍉", "🍇", "🍓", "🫐", "🍈", "🍒", "🍑", "🥭", "🍍",
            "🥥", "🥝", "🍅", "🥑",
            // Food
            "🍔", "🍕", "🍟", "🌮", "🍦", "🎂", "🍿", "🍣", "🍱", "🍜", "🍝",
            // Sports
            "⚽", "🏀", "🏈", "⚾", "🥎", "🎾", "🏐", "🏉", "🥏", "🎱", "🪀", "🏓", "🏸", "🏒", "🏑",
            // Activities/Objects
            "🎨", "🎬", "🎤", "🎧", "🎮", "🎯", "🎲", "🎳", "🕹️", "🪩", "🃏", "🎭", "🔫", "🪁",
            "🚀", "🚁", "🚂", "🚢", "🌌", "🌍", "🪐", "✈️", "🚄", "🛰️", "🚤", "🌙", "☄️", "⛵", "🚲",
        ];

        use rand::seq::SliceRandom;
        let mut rng = rand::rng();
        let mut values = Vec::with_capacity(total_tiles);

        let mut symbol_pool = symbols.to_vec();
        symbol_pool.shuffle(&mut rng);
        for i in 0..group_count {
            let symbol = symbol_pool[i % symbol_pool.len()];
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

        for _ in 0..remainder {
            self.tiles.push(Tile {
                status: TileStatus::Matched,
                value: String::new(),
            });
        }
    }
}
