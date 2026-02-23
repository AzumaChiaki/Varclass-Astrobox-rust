use std::sync::{OnceLock, RwLock};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TabType {
    Add,
    Manage,
    Import,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImportFormat {
    Json,
    Cses,
    Wakeup,
}

impl ImportFormat {
    pub fn as_str(&self) -> &'static str {
        match self {
            ImportFormat::Json => "json",
            ImportFormat::Cses => "cses",
            ImportFormat::Wakeup => "wakeup",
        }
    }
    pub fn from_str(s: &str) -> Self {
        let s = s.trim().to_lowercase();
        if s.contains("cses") {
            ImportFormat::Cses
        } else if s.contains("wakeup") {
            ImportFormat::Wakeup
        } else {
            ImportFormat::Json
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct CourseForm {
    pub day: String,
    pub name: String,
    pub room: String,
    pub start: String,
    pub end: String,
    pub week_type: String,
}

#[derive(Debug)]
pub struct UiState {
    pub root_element_id: Option<String>,
    pub current_tab: TabType,
    pub add_form: CourseForm,
    pub edit_form: CourseForm,
    pub selected_index: Option<usize>,
    pub import_text: String,
    pub import_format: ImportFormat,
    pub message: Option<(String, bool)>,
}

impl Default for UiState {
    fn default() -> Self {
        Self {
            root_element_id: None,
            current_tab: TabType::Add,
            add_form: CourseForm {
                day: "1".to_string(),
                name: String::new(),
                room: String::new(),
                start: "1".to_string(),
                end: "2".to_string(),
                week_type: "all".to_string(),
            },
            edit_form: CourseForm {
                day: "1".to_string(),
                name: String::new(),
                room: String::new(),
                start: "1".to_string(),
                end: "2".to_string(),
                week_type: "all".to_string(),
            },
            selected_index: None,
            import_text: String::new(),
            import_format: ImportFormat::Json,
            message: None,
        }
    }
}

static UI_STATE: OnceLock<RwLock<UiState>> = OnceLock::new();

pub fn ui_state() -> &'static RwLock<UiState> {
    UI_STATE.get_or_init(|| RwLock::new(UiState::default()))
}
