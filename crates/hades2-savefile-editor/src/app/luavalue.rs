use egui::ahash::HashMap;
use egui::{CollapsingHeader, Grid, TextEdit, Widget};
use hades2::saves::LuaValue;

pub fn show_value(
    ui: &mut egui::Ui,
    val: &mut LuaValue,
    pos: (usize, usize),
    nodes_visible: Option<&HashMap<(usize, usize), bool>>,
) -> bool {
    let response = match val {
        LuaValue::Nil => ui.label("Nil"),
        LuaValue::Bool(val) => ui.checkbox(val, ""),
        LuaValue::Number(val) => match ui.is_enabled() {
            true => ui.add(egui::DragValue::new(val)),
            false => ui.label(val.to_string()),
        },
        LuaValue::String(val) => match ui.is_enabled() {
            true => TextEdit::singleline(val)
                .min_size(egui::Vec2::from([30., 0.]))
                .ui(ui),
            false => ui.label(val.as_ref()),
        },
        LuaValue::Table(table) => return show_table(ui, table, pos, nodes_visible),
    };
    response.changed()
}

fn show_table(
    ui: &mut egui::Ui,
    table: &mut [(LuaValue, LuaValue)],
    pos: (usize, usize),
    nodes_visible: Option<&HashMap<(usize, usize), bool>>,
) -> bool {
    let test_visible = |pos| {
        let (is_visible, all_children_visible) = match nodes_visible {
            Some(visibles) => match visibles.get(&pos) {
                Some(true) => (true, true),
                Some(false) => (true, false),
                None => (false, false),
            },
            None => (true, true),
        };
        let nodes_visible_children = match all_children_visible {
            true => None,
            false => nodes_visible,
        };
        (is_visible, nodes_visible_children)
    };

    let mut changed = false;

    let has_primitives = table
        .iter()
        .next()
        .map_or(false, |(_, val)| val.is_primitive());

    let mut entries = table
        .iter_mut()
        .enumerate()
        .map(|(i, entry)| {
            let new_pos = (pos.0 + 1, i);
            (new_pos, entry)
        })
        .peekable();

    if has_primitives {
        Grid::new(pos).show(ui, |ui| {
            while let Some(&mut (pos, (key, val))) = entries.peek_mut() {
                if !val.is_primitive() {
                    break;
                }

                let (is_visible, nodevis_children) = test_visible(pos);

                if !is_visible {
                    entries.next();
                    continue;
                }

                ui.add_enabled_ui(false, |ui| {
                    show_value(ui, key, pos, nodevis_children);
                });

                changed |= show_value(ui, val, pos, nodevis_children);
                ui.end_row();

                entries.next();
            }
        });
    }

    for (pos, (key, val)) in entries {
        let (is_visible, nodevis_children) = test_visible(pos);
        if !is_visible {
            continue;
        }

        let LuaValue::Table(inner) = val else {
            unreachable!()
        };
        let name = key.primitive_to_str().unwrap_or(String::new());

        CollapsingHeader::new(name).id_source(pos).show(ui, |ui| {
            changed |= show_table(ui, inner, pos, nodevis_children);
        });
    }

    changed
}
