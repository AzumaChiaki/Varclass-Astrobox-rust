use crate::astrobox::psys_host::ui;
use crate::model::Course;
use crate::sync;
use crate::ui::event_handler::*;
use crate::ui::state::{ui_state, CourseForm, ImportFormat, TabType, UiState};

fn msg_element(state: &UiState) -> Option<ui::Element> {
    state.message.as_ref().map(|(text, is_error)| {
        ui::Element::new(ui::ElementType::Div, None)
            .width_full()
            .bg(if *is_error { "#6B1D1D" } else { "#1F4D1F" })
            .radius(8)
            .padding(10)
            .margin_bottom(10)
            .child(ui::Element::new(ui::ElementType::P, Some(text)).text_color("#FFFFFF"))
    })
}

fn tab_button(label: &str, event_id: &str, active: bool) -> ui::Element {
    ui::Element::new(ui::ElementType::Button, Some(label))
        .padding(10)
        .margin_right(8)
        .radius(8)
        .bg(if active { "#424242" } else { "#2A2A2A" })
        .on(ui::Event::Click, event_id)
        .on(ui::Event::PointerUp, event_id)
}

fn form_input(label: &str, value: &str, event_id: &str) -> ui::Element {
    ui::Element::new(ui::ElementType::Div, None)
        .width_full()
        .margin_bottom(10)
        .child(ui::Element::new(ui::ElementType::P, Some(label)).margin_bottom(4))
        .child(
            ui::Element::new(ui::ElementType::Input, Some(value))
                .on(ui::Event::Change, event_id)
                .on(ui::Event::Input, event_id)
                .width_full()
                .bg("#2A2A2A")
                .radius(8)
                .padding(8),
        )
}

fn action_button(label: &str, event_id: &str, primary: bool) -> ui::Element {
    ui::Element::new(ui::ElementType::Button, Some(label))
        .padding(10)
        .radius(8)
        .margin_right(8)
        .bg(if primary { "#2B5BE8" } else { "#2A2A2A" })
        .on(ui::Event::Click, event_id)
        .on(ui::Event::PointerUp, event_id)
}

fn form_from_course(course: &Course) -> CourseForm {
    CourseForm {
        day: course.day.to_string(),
        name: course.name.clone(),
        room: course.room.clone(),
        start: course.start.clone(),
        end: course.end.clone(),
        week_type: course.week_type.clone(),
    }
}

fn add_tab_ui(state: &UiState) -> ui::Element {
    ui::Element::new(ui::ElementType::Div, None)
        .width_full()
        .child(form_input("星期(1-7)", &state.add_form.day, INPUT_ADD_DAY))
        .child(form_input("课程名", &state.add_form.name, INPUT_ADD_NAME))
        .child(form_input("教室", &state.add_form.room, INPUT_ADD_ROOM))
        .child(form_input("开始节次", &state.add_form.start, INPUT_ADD_START))
        .child(form_input("结束节次", &state.add_form.end, INPUT_ADD_END))
        .child(form_input(
            "周类型(all/a/b)",
            &state.add_form.week_type,
            INPUT_ADD_WEEK_TYPE,
        ))
        .child(
            ui::Element::new(ui::ElementType::Div, None)
                .flex()
                .child(action_button("添加课程", EVENT_ADD_COURSE, true))
                .child(action_button("推送到手环", EVENT_PUSH_TO_WATCH, false)),
        )
}

fn course_select(state: &UiState, courses: &[Course]) -> ui::Element {
    let selected_text = state
        .selected_index
        .and_then(|idx| courses.get(idx))
        .map(|c| c.display_name())
        .unwrap_or_else(|| "请选择课程".to_string());

    let mut sel = ui::Element::new(ui::ElementType::Select, Some(&selected_text))
        .on(ui::Event::Change, EVENT_SELECT_COURSE)
        .width_full()
        .bg("#2A2A2A")
        .radius(8)
        .padding(8)
        .margin_bottom(10);

    sel = sel.child(ui::Element::new(ui::ElementType::Option, Some("请选择课程")));
    for (idx, c) in courses.iter().enumerate() {
        let text = format!("{} {}", idx + 1, c.display_name());
        sel = sel.child(ui::Element::new(ui::ElementType::Option, Some(&text)));
    }
    sel
}

fn manage_tab_ui(state: &UiState) -> ui::Element {
    let courses = sync::get_cached_courses();
    let mut root = ui::Element::new(ui::ElementType::Div, None)
        .width_full()
        .child(
            ui::Element::new(ui::ElementType::Div, None)
                .flex()
                .margin_bottom(10)
                .child(action_button("从手环获取课程", EVENT_PULL_FROM_WATCH, true))
                .child(action_button("推送到手环", EVENT_PUSH_TO_WATCH, false)),
        )
        .child(course_select(state, &courses));

    root = root.child(form_input("星期(1-7)", &state.edit_form.day, INPUT_EDIT_DAY));
    root = root.child(form_input("课程名", &state.edit_form.name, INPUT_EDIT_NAME));
    root = root.child(form_input("教室", &state.edit_form.room, INPUT_EDIT_ROOM));
    root = root.child(form_input("开始节次", &state.edit_form.start, INPUT_EDIT_START));
    root = root.child(form_input("结束节次", &state.edit_form.end, INPUT_EDIT_END));
    root = root.child(form_input(
        "周类型(all/a/b)",
        &state.edit_form.week_type,
        INPUT_EDIT_WEEK_TYPE,
    ));

    root.child(
        ui::Element::new(ui::ElementType::Div, None)
            .flex()
            .child(action_button("保存修改", EVENT_SAVE_EDIT, true))
            .child(action_button("删除课程", EVENT_DELETE_COURSE, false)),
    )
}

fn import_format_select(state: &UiState) -> ui::Element {
    let format_label = match state.import_format {
        ImportFormat::Json => "JSON",
        ImportFormat::Cses => "CSES (YAML)",
        ImportFormat::Wakeup => "WakeUp",
    };
    let mut sel = ui::Element::new(ui::ElementType::Select, Some(format_label))
        .on(ui::Event::Change, crate::ui::event_handler::INPUT_IMPORT_FORMAT)
        .width_full()
        .bg("#2A2A2A")
        .radius(8)
        .padding(8)
        .margin_bottom(10);
    sel = sel.child(ui::Element::new(ui::ElementType::Option, Some("JSON")));
    sel = sel.child(ui::Element::new(ui::ElementType::Option, Some("CSES (YAML)")));
    sel = sel.child(ui::Element::new(ui::ElementType::Option, Some("WakeUp")));
    sel
}

fn import_tab_ui(state: &UiState) -> ui::Element {
    let hint = match state.import_format {
        ImportFormat::Json => "粘贴 JSON 课程配置后导入",
        ImportFormat::Cses => "粘贴 CSES YAML 课表后导入",
        ImportFormat::Wakeup => "粘贴 WakeUp 多段 JSON 文本后导入",
    };
    ui::Element::new(ui::ElementType::Div, None)
        .width_full()
        .child(
            ui::Element::new(ui::ElementType::Div, None)
                .width_full()
                .margin_bottom(6)
                .child(
                    ui::Element::new(ui::ElementType::P, Some("解析格式"))
                        .text_color("#BBBBBB")
                        .margin_bottom(4),
                )
                .child(import_format_select(state)),
        )
        .child(
            ui::Element::new(ui::ElementType::P, Some(hint))
                .text_color("#BBBBBB")
                .margin_bottom(6),
        )
        .child(
            ui::Element::new(ui::ElementType::Input, Some(&state.import_text))
                .on(ui::Event::Change, INPUT_IMPORT_TEXT)
                .on(ui::Event::Input, INPUT_IMPORT_TEXT)
                .width_full()
                .height_full()
                .bg("#2A2A2A")
                .radius(8)
                .padding(8)
                .margin_bottom(10),
        )
        .child(
            ui::Element::new(ui::ElementType::Div, None)
                .flex()
                .child(action_button("导入配置", EVENT_IMPORT_PASTE, true))
                .child(action_button("推送到手环", EVENT_PUSH_TO_WATCH, false)),
        )
}

pub fn build_main_ui() -> ui::Element {
    let state = ui_state()
        .read()
        .unwrap_or_else(|poisoned| poisoned.into_inner());

    let mut root = ui::Element::new(ui::ElementType::Div, None)
        .flex()
        .flex_direction(ui::FlexDirection::Column)
        .width_full()
        .padding(16);

    if let Some(m) = msg_element(&state) {
        root = root.child(m);
    }

    let tabs = ui::Element::new(ui::ElementType::Div, None)
        .flex()
        .width_full()
        .margin_bottom(12)
        .child(tab_button(
            "添加课程",
            EVENT_TAB_ADD,
            state.current_tab == TabType::Add,
        ))
        .child(tab_button(
            "课程管理",
            EVENT_TAB_MANAGE,
            state.current_tab == TabType::Manage,
        ))
        .child(tab_button(
            "导入配置",
            EVENT_TAB_IMPORT,
            state.current_tab == TabType::Import,
        ));
    root = root.child(tabs);

    let content = match state.current_tab {
        TabType::Add => add_tab_ui(&state),
        TabType::Manage => manage_tab_ui(&state),
        TabType::Import => import_tab_ui(&state),
    };

    root.child(content)
}

pub fn render_main_ui(element_id: &str) {
    {
        let mut state = ui_state()
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        state.root_element_id = Some(element_id.to_string());
    }
    ui::render(element_id, build_main_ui());
}

pub fn refresh_main_ui() {
    let root_id = {
        let state = ui_state()
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        state.root_element_id.clone()
    };
    if let Some(root_id) = root_id {
        ui::render(&root_id, build_main_ui());
    }
}

pub fn fill_edit_form_by_index(index: usize) {
    let courses = sync::get_cached_courses();
    if let Some(course) = courses.get(index) {
        let mut state = ui_state()
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        state.selected_index = Some(index);
        state.edit_form = form_from_course(course);
    }
}
