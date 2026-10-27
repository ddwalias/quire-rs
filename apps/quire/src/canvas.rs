use crate::{QuireApp, equation};
use eframe::egui::{self, Color32, FontId, Rect, Sense, Stroke, Vec2};
use quire_core::{
    RegionContent, RegionLayout,
    layout::{CONTENT_BOTTOM, CONTENT_TOP, PAGE_HEIGHT, PAGE_MARGIN, PAGE_WIDTH},
};

const PAGE_GAP: f32 = 28.0;

impl QuireApp {
    pub(crate) fn move_selected(&mut self, delta: Vec2) {
        let Some(region) = self
            .worksheet
            .regions
            .iter_mut()
            .find(|region| Some(region.id) == self.selected)
        else {
            return;
        };
        let Some(position) = &mut region.layout else {
            return;
        };
        let minimum_width = if matches!(region.content, RegionContent::Math(_)) {
            96
        } else {
            position.width
        };
        position.x = (position.x as f32 + delta.x)
            .round()
            .clamp(24.0, (PAGE_WIDTH - 24 - minimum_width) as f32) as u32;
        position.width = position.width.min(PAGE_WIDTH - 24 - position.x);
        let next_y = position.y as f32 + delta.y;
        if next_y > CONTENT_BOTTOM as f32 && position.page < 999 {
            position.page += 1;
            position.y = CONTENT_TOP;
        } else if next_y < CONTENT_TOP as f32 && position.page > 0 {
            position.page -= 1;
            position.y = CONTENT_BOTTOM;
        } else {
            position.y = next_y
                .round()
                .clamp(CONTENT_TOP as f32, CONTENT_BOTTOM as f32) as u32;
        }
        self.worksheet.sort_by_position();
        self.recalculate();
    }

    pub(crate) fn canvas(&mut self, ui: &mut egui::Ui) {
        let zoom = self.zoom;
        egui::CentralPanel::default().frame(egui::Frame::new().fill(Color32::from_rgb(96, 99, 103))).show(ui, |ui| {
            ui.add_enabled_ui(self.pending.is_none() && self.file_mode.is_none(), |ui| {
                egui::ScrollArea::both().id_salt(("worksheet-scroll", self.editor_generation)).auto_shrink([false, false]).show(ui, |ui| {
                    let pages = self.worksheet.regions.iter().filter_map(|region| region.layout).map(|layout| layout.page + 1).max().unwrap_or(1)
                        .max(self.insertion.map(|layout| layout.page + 1).unwrap_or(1));
                    let page_size = egui::vec2(PAGE_WIDTH as f32, PAGE_HEIGHT as f32) * zoom;
                    let width = ui.available_width().max(page_size.x + 64.0);
                    let total_height = pages as f32 * (page_size.y + PAGE_GAP * zoom) + 48.0;
                    let (background, response) = ui.allocate_exact_size(egui::vec2(width, total_height), Sense::click());
                    let left = background.left() + (width - page_size.x) / 2.0;
                    let top = background.top() + 24.0;
                    let page_rect = |page: u32| Rect::from_min_size(egui::pos2(left, top + page as f32 * (page_size.y + PAGE_GAP * zoom)), page_size);
                    let visible = ui.clip_rect();
                    let painter = ui.painter().clone();

                    for page in 0..pages {
                        let rect = page_rect(page);
                        if !visible.intersects(rect.expand(8.0)) { continue; }
                        painter.rect_filled(rect.translate(egui::vec2(3.0, 5.0)), 0, Color32::from_black_alpha(14));
                        painter.rect_filled(rect, 0, Color32::WHITE);
                        painter.rect_stroke(rect, 0, Stroke::new(1.0, Color32::from_rgb(210, 216, 224)), egui::StrokeKind::Inside);
                        let origin = rect.min;
                        if self.show_grid {
                            let grid_color = Color32::from_rgb(195, 220, 240);
                            let body = Rect::from_min_max(origin + egui::vec2(PAGE_MARGIN as f32, CONTENT_TOP as f32) * zoom,
                                origin + egui::vec2((PAGE_WIDTH - PAGE_MARGIN) as f32, (CONTENT_BOTTOM + 16) as f32) * zoom);
                            let grid_painter = painter.with_clip_rect(body.intersect(visible));
                            for x in (PAGE_MARGIN..=PAGE_WIDTH - PAGE_MARGIN).step_by(16) {
                                let x = origin.x + x as f32 * zoom;
                                grid_painter.line_segment([egui::pos2(x, body.top()), egui::pos2(x, body.bottom())], Stroke::new(0.5, grid_color));
                            }
                            for y in (CONTENT_TOP..=CONTENT_BOTTOM + 16).step_by(16) {
                                let y = origin.y + y as f32 * zoom;
                                grid_painter.line_segment([egui::pos2(body.left(), y), egui::pos2(body.right(), y)], Stroke::new(0.5, grid_color));
                            }
                        }
                        #[cfg(test)]
                        { self.targets.insert(format!("page_{page}"), rect); }
                    }
                    let mut changed = false;
                    let mut selected = self.selected;
                    let mut editing = self.editing;
                    let mut moved = None;
                    let mut drag_finished = false;
                    let mut resized = None;
                    // The active editor floats over nearby regions while its
                    // source is open, without changing stored page positions.
                    let mut indices: Vec<_> = (0..self.worksheet.regions.len()).collect();
                    indices.sort_by_key(|index| Some(self.worksheet.regions[*index].id) == self.editing);
                    for index in indices {
                        let region = &mut self.worksheet.regions[index];
                        let Some(position) = region.layout else { continue; };
                        let id = region.id;
                        let page = page_rect(position.page);
                        let origin = page.min + egui::vec2(position.x as f32, position.y as f32) * zoom;
                        let max_width = position.width as f32 * zoom;
                        let available_height = ((PAGE_HEIGHT - 64 - position.y) as f32 * zoom).min(280.0 * zoom);
                        let region_painter = painter.with_clip_rect(page.intersect(visible));
                        let is_math = matches!(region.content, RegionContent::Math(_));
                        let result = self.results.get(index).and_then(Option::as_ref);
                        let successful = result.and_then(|result| result.as_ref().ok());
                        let formula = if is_math { equation::formula(&region_painter, region.content.source(), successful, 22.0 * zoom).ok() } else { None };
                        let text = if !is_math {
                            Some(region_painter.layout(region.content.source().into(), FontId::proportional(15.0 * zoom), equation::INK, max_width))
                        } else { None };
                        let content_size = formula.as_ref().map(|formula| formula.size).or_else(|| text.as_ref().map(|galley| galley.size())).unwrap_or(egui::vec2(220.0, 30.0) * zoom);
                        let error = result.and_then(|result| result.as_ref().err()).filter(|_| !region.content.source().is_empty());
                        let has_overflow = content_size.x > max_width || content_size.y > available_height - 20.0 * zoom;
                        let mut height = content_size.y.min(available_height - 20.0 * zoom).max(30.0 * zoom);
                        if error.is_some() || has_overflow { height += 24.0 * zoom; }
                        let preview_height = height;
                        if editing == Some(id) { height += if is_math { 46.0 * zoom } else { 140.0 * zoom }; }
                        let width = if editing == Some(id) { max_width.min(540.0 * zoom) } else if is_math { (content_size.x + 18.0 * zoom).min(max_width).max(96.0 * zoom) } else { max_width };
                        let rect = Rect::from_min_size(origin - egui::vec2(6.0, 5.0) * zoom, egui::vec2(width + 12.0 * zoom, height + 10.0 * zoom));
                        if self.focus_region == Some(id) && !visible.contains_rect(rect) {
                            ui.scroll_to_rect(rect, Some(egui::Align::Center));
                        }
                        if !visible.intersects(rect) && editing != Some(id) { continue; }
                        let response = ui.interact(rect, egui::Id::unique(("region", self.editor_generation, id)), Sense::click());
                        response.widget_info(|| egui::WidgetInfo::labeled(egui::accesskit::Role::Button, true,
                            format!("{} region {}: {}{}", if is_math { "Math" } else { "Text" }, id, region.content.source(), successful.map(|value| format!(" = {value}")).unwrap_or_default())));
                        #[cfg(test)]
                        { self.targets.insert(format!("region_{id}"), rect); }
                        if response.clicked() { selected = Some(id); }
                        if response.double_clicked() { selected = Some(id); editing = Some(id); self.focus_region = Some(id); }
                        if selected == Some(id) || response.hovered() {
                            region_painter.rect_filled(rect, 2, Color32::from_white_alpha(244));
                            region_painter.rect_stroke(rect, 2, Stroke::new(1.0, if selected == Some(id) { Color32::from_rgb(100, 150, 200) } else { Color32::from_rgb(199, 217, 234) }), egui::StrokeKind::Inside);
                        }
                        let content_clip = Rect::from_min_size(origin, egui::vec2(max_width, (preview_height - if error.is_some() || has_overflow { 24.0 * zoom } else { 0.0 }).max(20.0 * zoom)));
                        let content_painter = region_painter.with_clip_rect(content_clip.intersect(visible));
                        if let Some(formula) = &formula { formula.paint(&content_painter, origin, zoom); }
                        else if let Some(galley) = text { content_painter.galley(origin, galley, equation::INK); }
                        else { content_painter.text(origin, egui::Align2::LEFT_TOP, if region.content.source().is_empty() { "Type an expression" } else { region.content.source() }, FontId::monospace(16.0 * zoom), equation::INK); }
                        if let Some(error) = error {
                            region_painter.with_clip_rect(rect).text(origin + egui::vec2(0.0, preview_height - 19.0 * zoom), egui::Align2::LEFT_TOP,
                                error.to_string(), FontId::proportional(11.0 * zoom), Color32::from_rgb(174, 50, 45));
                        } else if has_overflow {
                            region_painter.text(origin + egui::vec2(0.0, preview_height - 19.0 * zoom), egui::Align2::LEFT_TOP,
                                "More content — double-click to edit", FontId::proportional(11.0 * zoom), Color32::from_rgb(112, 127, 144));
                        }
                        response.on_hover_text(if let Some(error) = error { error.to_string() } else { "Double-click to edit. Drag the handle to move.".into() });

                        if selected == Some(id) {
                            let handle = Rect::from_min_size(rect.min - egui::vec2(16.0 * zoom, 0.0), egui::vec2(14.0 * zoom, 25.0 * zoom));
                            let drag = ui.interact(handle, egui::Id::unique(("drag", self.editor_generation, id)), Sense::drag());
                            #[cfg(test)]
                            { self.targets.insert(format!("drag_{id}"), handle); }
                            for row in 0..3 { for col in 0..2 { region_painter.circle_filled(handle.min + egui::vec2(4.0 + col as f32 * 4.0, 6.0 + row as f32 * 5.0) * zoom, zoom, equation::BLUE); } }
                            if drag.dragged() {
                                // The press frame may include pointer movement
                                // from elsewhere on the page into the handle.
                                if !drag.drag_started() { moved = Some(drag.drag_delta() / zoom); }
                                editing = None;
                            }
                            drag_finished |= drag.drag_stopped();
                            drag.on_hover_cursor(egui::CursorIcon::Grab).on_hover_text("Drag to move. Calculation follows page order.");
                            if !is_math {
                                let handle = Rect::from_center_size(rect.right_bottom(), egui::vec2(12.0, 12.0) * zoom);
                                let resize = ui.interact(handle, egui::Id::unique(("resize", self.editor_generation, id)), Sense::drag());
                                #[cfg(test)]
                                { self.targets.insert(format!("resize_{id}"), handle); }
                                region_painter.rect_filled(handle.shrink(3.0 * zoom), 1, equation::BLUE);
                                if resize.dragged() && !resize.drag_started() { resized = Some((id, resize.drag_delta().x / zoom)); }
                                resize.on_hover_cursor(egui::CursorIcon::ResizeHorizontal).on_hover_text("Drag to change text width");
                            }
                        }
                        if editing == Some(id) {
                            let editor_rect = Rect::from_min_size(origin + egui::vec2(0.0, preview_height + 6.0 * zoom), egui::vec2(width, height - preview_height - 8.0 * zoom));
                            let editor_id = egui::Id::unique(("source", self.editor_generation, id));
                            let needs_focus = self.focus_region == Some(id);
                            if needs_focus { ui.memory_mut(|memory| memory.request_focus(editor_id)); self.focus_region = None; }
                            ui.scope_builder(egui::UiBuilder::new().max_rect(editor_rect).id_salt(("editor", id)), |ui| {
                                ui.set_clip_rect(editor_rect.intersect(visible));
                                let mut show_editor = |ui: &mut egui::Ui| {
                                    let edit = if is_math { egui::TextEdit::singleline(region.content.source_mut()).font(FontId::monospace(14.0 * zoom)) }
                                        else { egui::TextEdit::multiline(region.content.source_mut()).font(FontId::proportional(15.0 * zoom)).desired_rows(4) };
                                    ui.add_sized(editor_rect.size(), edit.id(editor_id).desired_width(width).text_color(equation::INK).background_color(Color32::from_rgb(243, 247, 252)))
                                };
                                let response = if is_math { show_editor(ui) } else {
                                    egui::ScrollArea::vertical().id_salt(("text-source", self.editor_generation, id))
                                        .max_height(editor_rect.height()).show(ui, show_editor).inner
                                };
                                #[cfg(test)]
                                { self.targets.insert(format!("source_{id}"), response.rect.intersect(editor_rect)); }
                                changed |= response.changed();
                                if needs_focus { response.request_focus(); }
                                response.on_hover_text("Edit source. Esc or Ctrl/Cmd+Enter returns to the typeset view.");
                            });
                        }
                    }
                    self.selected = selected; self.editing = editing;
                    if response.clicked()
                        && let Some(pointer) = response.interact_pointer_pos() {
                            for page in 0..pages {
                                let rect = page_rect(page);
                                if rect.contains(pointer) {
                                    let local = (pointer - rect.min) / zoom;
                                    self.insertion = Some(RegionLayout { page, x: local.x.round().clamp(24.0, (PAGE_WIDTH - 120) as f32) as u32,
                                        y: local.y.round().clamp(CONTENT_TOP as f32, CONTENT_BOTTOM as f32) as u32, width: 96 });
                                    if let Some(layout) = &mut self.insertion { layout.width = (PAGE_WIDTH - PAGE_MARGIN).saturating_sub(layout.x).max(96).min(PAGE_WIDTH - 24 - layout.x); }
                                    self.selected = None; self.editing = None;
                                    ui.memory_mut(|memory| { if let Some(id) = memory.focused() { memory.surrender_focus(id); } });
                                    break;
                                }
                            }
                        }
                    if response.double_clicked() && self.insertion.is_some() { self.add_region(false); changed = true; }
                    if self.selected.is_none()
                        && let Some(position) = self.insertion {
                            let point = page_rect(position.page).min + egui::vec2(position.x as f32, position.y as f32) * zoom;
                            painter.line_segment([point - egui::vec2(7.0, 0.0), point + egui::vec2(7.0, 0.0)], Stroke::new(1.0, equation::BLUE));
                            painter.line_segment([point - egui::vec2(0.0, 7.0), point + egui::vec2(0.0, 7.0)], Stroke::new(1.0, equation::BLUE));
                        }
                    if let Some(delta) = moved { self.move_selected(delta); changed = true; }
                    if let Some((id, delta)) = resized
                        && let Some(position) = self.worksheet.regions.iter_mut().find(|region| region.id == id).and_then(|region| region.layout.as_mut()) {
                            position.width = (position.width as f32 + delta).round().clamp(96.0, (PAGE_WIDTH - 24 - position.x) as f32) as u32;
                            changed = true;
                    }
                    if drag_finished { self.worksheet.sort_by_position(); changed = true; }
                    if changed { self.recalculate(); ui.ctx().request_repaint(); }
                });
            });
        });
    }
}
