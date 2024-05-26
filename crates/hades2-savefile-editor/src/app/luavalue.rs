use egui::ahash::HashMap;
use egui::{CollapsingHeader, Grid, TextEdit, Widget};
use hades2::saves::LuaValue;

#[derive(Eq, PartialEq, Hash, Clone, Default)]
pub struct Pos(Vec<u16>);

impl Pos {
    #[must_use]
    pub fn push(&self, val: u16) -> Pos {
        let mut x = self.clone();
        x.0.push(val);
        x
    }
}

pub fn show_value(
    ui: &mut egui::Ui,
    val: &mut LuaValue,
    pos: Pos,
    nodes_visible: Option<&HashMap<Pos, bool>>,
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
    pos: Pos,
    nodes_visible: Option<&HashMap<Pos, bool>>,
) -> bool {
    fn test_visible<'a>(
        pos: &Pos,
        nodes_visible: Option<&'a HashMap<Pos, bool>>,
    ) -> (bool, Option<&'a HashMap<Pos, bool>>) {
        let (is_visible, all_children_visible) = match nodes_visible {
            Some(visibles) => match visibles.get(pos) {
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
    }

    let mut changed = false;

    let entries = table
        .iter_mut()
        .enumerate()
        .map(|(i, entry)| {
            let new_pos = pos.push(i.try_into().unwrap());
            (new_pos, entry)
        })
        .collect::<Vec<_>>();

    let has_primitives = entries.iter().any(|(pos, (_, val))| {
        val.is_primitive() && nodes_visible.map_or(true, |v| v.contains_key(&pos))
    });

    let mut entries = entries.into_iter().peekable();

    if has_primitives {
        Grid::new(egui::Id::new(&pos)).show(ui, |ui| {
            while let Some(&mut (ref pos, (key, val))) = entries.peek_mut() {
                if !val.is_primitive() {
                    break;
                }

                let (is_visible, nodevis_children) = test_visible(pos, nodes_visible);

                if !is_visible {
                    entries.next();
                    continue;
                }

                ui.add_enabled_ui(false, |ui| {
                    show_value(ui, key, pos.clone(), nodevis_children);
                });

                changed |= show_value(ui, val, pos.clone(), nodevis_children);
                ui.end_row();

                entries.next();
            }
        });
    }

    for (pos, (key, val)) in entries {
        let (is_visible, nodevis_children) = test_visible(&pos, nodes_visible);
        if !is_visible {
            continue;
        }

        let LuaValue::Table(inner) = val else {
            unreachable!()
        };
        let name = key.primitive_to_str().unwrap_or_default();

        CollapsingHeader::new(name).show(ui, |ui| {
            changed |= show_table(ui, inner, pos.clone(), nodevis_children);
        });
    }

    changed
}
