use std::sync::{OnceLock, RwLock};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TabType {
    Add,
    Manage,
    Import,
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
    pub message: Option<(String, bool)>,
    pub last_event: String,
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
            message: None,
            last_event: String::new(),
        }
    }
}

static UI_STATE: OnceLock<RwLock<UiState>> = OnceLock::new();

pub fn ui_state() -> &'static RwLock<UiState> {
    UI_STATE.get_or_init(|| RwLock::new(UiState::default()))
}
