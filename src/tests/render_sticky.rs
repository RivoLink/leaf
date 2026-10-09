use super::common::render_helpers::*;

#[test]
fn sticky_hides_when_scroll_row_zero_is_between_files_blank() {
    let (state, lines) = build_two_file_diff_state();

    let bottom0 = state.file_bottom_idx[0];
    let spacer_idx = bottom0 + 1;
    assert!(spacer_idx < lines.len(), "expected trailing spacer");
    assert_eq!(
        state.file_owner[spacer_idx], None,
        "between-files spacer must not be owned by any file"
    );
    assert_eq!(
        crate::render::compute_sticky_top_idx(&state, spacer_idx),
        None,
        "sticky overlay must hide when row 0 is the between-files spacer"
    );
}

#[test]
fn sticky_hides_when_scroll_row_zero_is_file_bottom_border() {
    let (state, _lines) = build_two_file_diff_state();
    let bottom0 = state.file_bottom_idx[0];

    assert_eq!(
        crate::render::compute_sticky_top_idx(&state, bottom0),
        None,
        "sticky must hide when row 0 is the file's `└─` bottom border"
    );

    assert_eq!(
        crate::render::compute_sticky_top_idx(&state, bottom0 - 1),
        None,
        "sticky must hide when the `└─` bottom border is the very next row"
    );
}

#[test]
fn sticky_hides_when_scroll_row_zero_is_at_or_after_file_top() {
    let (state, _lines) = build_two_file_diff_state();
    let top0 = state.file_top_idx[0];

    assert_eq!(
        crate::render::compute_sticky_top_idx(&state, top0),
        None,
        "sticky must hide when viewport row 0 IS the file's top border"
    );
}

#[test]
fn sticky_shows_file_top_when_scrolled_past_it() {
    let (state, _lines) = build_two_file_diff_state();
    let top0 = state.file_top_idx[0];
    let bottom0 = state.file_bottom_idx[0];

    assert!(bottom0 >= top0 + 4, "file 0 must have interior rows");
    let mid = top0 + 3;
    assert!(mid < bottom0 - 1);
    let sticky = crate::render::compute_sticky_top_idx(&state, mid);
    assert_eq!(
        sticky,
        Some(top0),
        "sticky must pin the file's `┌─` top border when scrolled into body"
    );

    let top1 = state.file_top_idx[1];
    let bottom1 = state.file_bottom_idx[1];
    if bottom1 >= top1 + 4 {
        let mid1 = top1 + 3;
        if mid1 < bottom1 - 1 {
            assert_eq!(
                crate::render::compute_sticky_top_idx(&state, mid1),
                Some(top1),
                "sticky must pin file 1's top when scrolled into its body"
            );
        }
    }
}
