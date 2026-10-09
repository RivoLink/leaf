use super::{apply_split_build, App, DiffFlash};
use std::sync::Arc;

impl App {
    pub(crate) fn diff_toggle_tree(&mut self) -> bool {
        let Some(state) = self.diff_state.as_mut() else {
            return false;
        };
        state.tree_visible = !state.tree_visible;
        if state.tree_visible {
            state.tree_active_idx = state
                .current_file_idx
                .min(state.files.len().saturating_sub(1));
        }
        let now_visible = state.tree_visible;

        if now_visible {
            self.diff_tree_scroll_hint_dismissed = false;
        }
        now_visible
    }

    pub(crate) fn diff_toggle_layout(
        &mut self,
        ss: &syntect::parsing::SyntaxSet,
        theme: &syntect::highlighting::Theme,
    ) -> bool {
        let Some(state) = self.diff_state.as_mut() else {
            return false;
        };
        let target = state.layout.toggled();
        if matches!(target, crate::diff::DiffLayout::Split) && !state.split_ready {
            if self.pending_split_build.is_none() {
                self.build_split_now(ss, theme);
                if let Some(state) = self.diff_state.as_mut() {
                    state.layout = target;
                }
                return true;
            }
            self.set_diff_flash(DiffFlash::SplitBuilding);
            return false;
        }
        state.layout = target;
        true
    }

    pub(crate) fn build_split_now(
        &mut self,
        ss: &syntect::parsing::SyntaxSet,
        theme: &syntect::highlighting::Theme,
    ) -> bool {
        let files = match self.diff_state.as_ref() {
            Some(s) => Arc::clone(&s.files),
            None => return false,
        };
        let built = crate::render::build_split_lines(&files, ss, theme);
        if let Some(state) = self.diff_state.as_mut() {
            apply_split_build(state, built);
        }
        true
    }

    pub(crate) fn diff_recenter_on_current_file(&mut self) -> bool {
        let Some(s) = self.diff_state.as_ref() else {
            return false;
        };
        let target_line = s.first_scroll_target_of_file(s.current_file_idx);
        let state = self.diff_state.as_mut().expect("checked");
        state.nav_pinned_file_idx = Some(state.current_file_idx);

        state.tree_scroll_manual = false;
        if let Some(line) = target_line {
            self.scroll = line;
        }
        self.clamp_scroll();
        true
    }

    pub(crate) fn diff_preview_step_file(
        &mut self,
        delta: isize,
        ss: &syntect::parsing::SyntaxSet,
        theme: &syntect::highlighting::Theme,
    ) -> bool {
        let (next_idx, file, spec, need_build, restored_scroll) = {
            let Some(state) = self.diff_state.as_mut() else {
                return false;
            };
            if state.files.is_empty() {
                return false;
            }
            let last = state.files.len() - 1;
            let cur = state.tree_active_idx.min(last);
            let next = (cur as isize + delta).clamp(0, last as isize) as usize;
            if next == cur {
                return false;
            }

            state.preview_scroll_cache.insert(cur, self.scroll);
            state.tree_active_idx = next;
            state.current_file_idx = next;
            let need_build = !state.preview_cache.contains_key(&next);
            let restored = state.preview_scroll_cache.get(&next).copied().unwrap_or(0);
            (
                next,
                state.files[next].clone(),
                state.spec.clone(),
                need_build,
                restored,
            )
        };
        self.scroll = restored_scroll;
        if need_build {
            let pair = crate::app::build_preview_pair(&file, &spec, ss, theme);
            if let Some(state) = self.diff_state.as_mut() {
                state.preview_cache.insert(next_idx, pair);
            }
        }
        self.clamp_scroll();
        true
    }

    pub(crate) fn diff_toggle_preview(
        &mut self,
        ss: &syntect::parsing::SyntaxSet,
        theme: &syntect::highlighting::Theme,
    ) -> bool {
        let sticky_idx = self.diff_current_file_from_scroll();
        let Some(s) = self.diff_state.as_ref() else {
            return false;
        };
        if s.files.is_empty() {
            return false;
        }
        let active_idx = sticky_idx
            .unwrap_or(s.tree_active_idx)
            .min(s.files.len() - 1);

        let (build_inputs, new_visible, close_target, open_scroll) = {
            let state = self.diff_state.as_mut().expect("checked");
            let was_visible = state.preview_visible;
            let new_visible = !was_visible;
            let (close_target, open_scroll) = if was_visible {
                state
                    .preview_scroll_cache
                    .insert(state.current_file_idx, self.scroll);
                let changed_file = state
                    .pre_preview_file_idx
                    .is_some_and(|idx| idx != state.current_file_idx);
                let restore = state.pre_preview_scroll.unwrap_or(0);
                state.pre_preview_scroll = None;
                state.pre_preview_file_idx = None;
                let target = if changed_file {
                    let jump = state.first_scroll_target_of_file(state.current_file_idx);
                    jump.unwrap_or(restore)
                } else {
                    restore
                };
                (Some(target), 0)
            } else {
                state.pre_preview_scroll = Some(self.scroll);
                state.pre_preview_file_idx = Some(state.current_file_idx);
                let restored = state
                    .preview_scroll_cache
                    .get(&active_idx)
                    .copied()
                    .unwrap_or(0);
                (None, restored)
            };
            state.tree_active_idx = active_idx;
            state.current_file_idx = active_idx;
            state.preview_visible = new_visible;
            let need_build =
                state.preview_visible && !state.preview_cache.contains_key(&active_idx);

            let build_inputs =
                need_build.then(|| (state.files[active_idx].clone(), state.spec.clone()));
            (
                build_inputs,
                state.preview_visible,
                close_target,
                open_scroll,
            )
        };

        if new_visible {
            self.scroll = open_scroll;
        } else if let Some(target) = close_target {
            self.scroll = target;
        }

        if let Some((file, spec)) = build_inputs {
            let pair = crate::app::build_preview_pair(&file, &spec, ss, theme);
            if let Some(state) = self.diff_state.as_mut() {
                state.preview_cache.insert(active_idx, pair);
            }
        }
        self.clamp_scroll();
        true
    }
}
