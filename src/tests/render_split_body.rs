use super::common::render_helpers::*;

#[test]
fn split_middle_divider_at_viewport_center() {
    for &w in &[80u16, 100, 140] {
        let buffer = render_split_at_width(w, false);

        let expected = 1u16 + (w - 3) / 2;
        let mut body_row: Option<u16> = None;
        for y in 0..buffer.area.height {
            let mut has_frame = false;
            let mut has_sigil = false;
            for x in 0..buffer.area.width {
                let sym = buffer.cell((x, y)).unwrap().symbol();
                if sym == "┌" || sym == "└" || sym == "├" {
                    has_frame = true;
                }
                if sym == "+" || sym == "-" {
                    has_sigil = true;
                }
            }
            if has_sigil && !has_frame {
                body_row = Some(y);
                break;
            }
        }
        let row = body_row.expect("split view must have a body row");
        let mid = middle_divider_col(&buffer, row).expect("body row has middle divider");
        assert_eq!(
            mid, expected,
            "middle `│` should land at content centre for width {w}: got col {mid}, want {expected}"
        );
    }
}

#[test]
fn split_body_bg_padding_uses_row_color() {
    use crate::theme::app_theme;
    let w: u16 = 100;
    let buffer = render_split_at_width(w, false);
    let theme = app_theme();
    let add_bg = theme.diff.add_bg;
    let del_bg = theme.diff.del_bg;

    let mut change_row: Option<u16> = None;
    for y in 0..buffer.area.height {
        let Some(mid) = middle_divider_col(&buffer, y) else {
            continue;
        };
        let mut has_minus = false;
        for x in 0..mid {
            if buffer.cell((x, y)).unwrap().symbol() == "-" {
                has_minus = true;
                break;
            }
        }
        if has_minus {
            change_row = Some(y);
            break;
        }
    }
    let row = change_row.expect("expected a Change row with `-` sigil");
    let mid = middle_divider_col(&buffer, row).expect("Change row has middle divider");

    let left_pad_cell = buffer.cell((mid - 1, row)).unwrap();
    assert_eq!(
        left_pad_cell.bg, del_bg,
        "LEFT padding of a Del/Change row must carry del_bg, got {:?}",
        left_pad_cell.bg
    );

    let right_border_col = 1u16 + (w - 3) - 1;
    let right_pad_cell = buffer.cell((right_border_col - 1, row)).unwrap();
    assert_eq!(
        right_pad_cell.bg, add_bg,
        "RIGHT padding of an Add/Change row must carry add_bg, got {:?}",
        right_pad_cell.bg
    );
}

#[test]
fn split_truncated_left_bg_extends_to_middle() {
    use crate::theme::app_theme;
    let w: u16 = 80;
    let buffer = render_split_long_lines(w);
    let theme = app_theme();
    let del_bg = theme.diff.del_bg;

    let mut row: Option<u16> = None;
    for y in 0..buffer.area.height {
        let Some(mid) = middle_divider_col(&buffer, y) else {
            continue;
        };
        for x in 0..mid {
            if buffer.cell((x, y)).unwrap().symbol() == "-" {
                row = Some(y);
                break;
            }
        }
        if row.is_some() {
            break;
        }
    }
    let row = row.expect("expected a Del row with `-` sigil");
    let mid = middle_divider_col(&buffer, row).expect("Del row has middle divider");

    let mut ellipsis_col: Option<u16> = None;
    for x in 0..mid {
        if buffer.cell((x, row)).unwrap().symbol() == "…" {
            ellipsis_col = Some(x);
            break;
        }
    }
    let ellipsis_col = ellipsis_col.expect("expected `…` on truncated LEFT half");

    for x in ellipsis_col..mid {
        let cell = buffer.cell((x, row)).unwrap();
        assert_eq!(
            cell.bg,
            del_bg,
            "LEFT-half cell at col {x} between `…` and middle `│` on row {row} \
             must carry del_bg, got {:?} (symbol {:?})",
            cell.bg,
            cell.symbol()
        );
    }
}

#[test]
fn split_truncated_right_bg_extends_to_right_frame() {
    use crate::theme::app_theme;
    let w: u16 = 80;
    let buffer = render_split_long_lines(w);
    let theme = app_theme();
    let add_bg = theme.diff.add_bg;

    let mut row: Option<u16> = None;
    for y in 0..buffer.area.height {
        let Some(mid) = middle_divider_col(&buffer, y) else {
            continue;
        };
        for x in (mid + 1)..buffer.area.width {
            if buffer.cell((x, y)).unwrap().symbol() == "+" {
                row = Some(y);
                break;
            }
        }
        if row.is_some() {
            break;
        }
    }
    let row = row.expect("expected an Add row with `+` sigil on RIGHT half");
    let mid = middle_divider_col(&buffer, row).expect("Add row has middle divider");

    let right_border_col = 1u16 + (w - 3) - 1;

    let mut ellipsis_col: Option<u16> = None;
    for x in (mid + 1)..right_border_col {
        if buffer.cell((x, row)).unwrap().symbol() == "…" {
            ellipsis_col = Some(x);
            break;
        }
    }
    let ellipsis_col = ellipsis_col.expect("expected `…` on truncated RIGHT half");

    for x in ellipsis_col..right_border_col {
        let cell = buffer.cell((x, row)).unwrap();
        assert_eq!(
            cell.bg,
            add_bg,
            "RIGHT-half cell at col {x} between `…` and right frame on row {row} \
             must carry add_bg, got {:?} (symbol {:?})",
            cell.bg,
            cell.symbol()
        );
    }
}

#[test]
fn split_ellipsis_is_last_cell_before_middle_divider() {
    let w: u16 = 80;
    let buffer = render_split_long_lines(w);
    let mut row: Option<u16> = None;
    for y in 0..buffer.area.height {
        let Some(mid) = middle_divider_col(&buffer, y) else {
            continue;
        };
        for x in 0..mid {
            if buffer.cell((x, y)).unwrap().symbol() == "-" {
                row = Some(y);
                break;
            }
        }
        if row.is_some() {
            break;
        }
    }
    let row = row.expect("expected a Del row with `-` sigil");
    let mid = middle_divider_col(&buffer, row).expect("Del row has middle divider");
    let cell_before = buffer.cell((mid - 1, row)).unwrap();
    assert_eq!(
        cell_before.symbol(),
        "…",
        "cell at col {} (middle_col - 1) on row {row} must be `…`, got {:?}",
        mid - 1,
        cell_before.symbol()
    );
}

#[test]
fn split_ellipsis_is_last_cell_before_right_frame() {
    let w: u16 = 80;
    let buffer = render_split_long_lines(w);
    let mut row: Option<u16> = None;
    for y in 0..buffer.area.height {
        let Some(mid) = middle_divider_col(&buffer, y) else {
            continue;
        };
        for x in (mid + 1)..buffer.area.width {
            if buffer.cell((x, y)).unwrap().symbol() == "+" {
                row = Some(y);
                break;
            }
        }
        if row.is_some() {
            break;
        }
    }
    let row = row.expect("expected an Add row with `+` sigil on RIGHT half");

    let right_border_col = 1u16 + (w - 3) - 1;
    let cell_before = buffer.cell((right_border_col - 1, row)).unwrap();
    assert_eq!(
        cell_before.symbol(),
        "…",
        "cell at col {} (right_frame - 1) on row {row} must be `…`, got {:?}",
        right_border_col - 1,
        cell_before.symbol()
    );
}

#[test]
fn split_truncate_preserves_content_up_to_ellipsis() {
    let w: u16 = 80;
    let buffer = render_split_long_lines(w);

    let mut row: Option<u16> = None;
    let mut sigil_col: Option<u16> = None;
    for y in 0..buffer.area.height {
        let Some(mid) = middle_divider_col(&buffer, y) else {
            continue;
        };
        for x in 0..mid {
            if buffer.cell((x, y)).unwrap().symbol() == "-" {
                row = Some(y);
                sigil_col = Some(x);
                break;
            }
        }
        if row.is_some() {
            break;
        }
    }
    let row = row.expect("expected a Del row with `-` sigil");
    let sigil_col = sigil_col.expect("sigil column found");
    let mid = middle_divider_col(&buffer, row).expect("Del row has middle divider");

    let content_start = sigil_col + 2;
    for x in content_start..(mid - 1) {
        let sym = buffer.cell((x, row)).unwrap().symbol();
        assert_eq!(
            sym, "D",
            "LEFT-half cell at col {x} on row {row} must be a content \
             char (`D`), got {sym:?} — boundary span was dropped \
             instead of truncated"
        );
    }

    assert_eq!(
        buffer.cell((mid - 1, row)).unwrap().symbol(),
        "…",
        "expected `…` at col {} on row {row}",
        mid - 1
    );
}

#[test]
fn split_truncate_boundary_span_split_correctly() {
    let w: u16 = 80;
    let buffer = render_split_long_lines(w);

    let mut row: Option<u16> = None;
    let mut sigil_col: Option<u16> = None;
    for y in 0..buffer.area.height {
        let Some(mid) = middle_divider_col(&buffer, y) else {
            continue;
        };
        for x in 0..mid {
            if buffer.cell((x, y)).unwrap().symbol() == "-" {
                row = Some(y);
                sigil_col = Some(x);
                break;
            }
        }
        if row.is_some() {
            break;
        }
    }
    let row = row.expect("expected a Del row with `-` sigil");
    let sigil_col = sigil_col.expect("sigil column found");
    let mid = middle_divider_col(&buffer, row).expect("Del row has middle divider");

    let mut d_count = 0u16;
    for x in (sigil_col + 2)..(mid - 1) {
        if buffer.cell((x, row)).unwrap().symbol() == "D" {
            d_count += 1;
        }
    }

    let expected = (mid - 1) - (sigil_col + 2);
    assert_eq!(
        d_count,
        expected,
        "boundary span must be truncated (not dropped): expected \
         {expected} `D` cells between sigil (col {sigil_col}) and \
         ellipsis (col {}), got {d_count}",
        mid - 1
    );
    assert!(
        d_count > 0,
        "at least one `D` content cell must survive the truncation"
    );
}

#[test]
fn split_blank_row_uses_dotted_middle() {
    let w: u16 = 100;
    let buffer = render_split_at_width(w, false);
    let row = first_blank_spacer_row(&buffer).expect("expected a blank spacer row inside the box");

    let middle_col = 1u16 + (w - 3) / 2;
    let cell = buffer.cell((middle_col, row)).unwrap();
    assert_eq!(
        cell.symbol(),
        "│",
        "expected solid `│` at middle col {middle_col} on blank spacer row {row}, got {:?}",
        cell.symbol()
    );
}

#[test]
fn split_body_row_middle_is_dotted() {
    let w: u16 = 100;
    let buffer = render_split_at_width(w, false);

    let mut body_row: Option<u16> = None;
    for y in 0..buffer.area.height {
        let mut has_frame = false;
        let mut has_sigil = false;
        for x in 0..buffer.area.width {
            let sym = buffer.cell((x, y)).unwrap().symbol();
            if sym == "┌" || sym == "├" || sym == "└" || sym == "┐" || sym == "┘" {
                has_frame = true;
            }
            if sym == "+" || sym == "-" {
                has_sigil = true;
            }
        }
        if has_sigil && !has_frame {
            body_row = Some(y);
            break;
        }
    }
    let row = body_row.expect("expected a body row with a `+` or `-` sigil");
    let middle_col = 1u16 + (w - 3) / 2;
    let cell = buffer.cell((middle_col, row)).unwrap();
    assert_eq!(
        cell.symbol(),
        "│",
        "expected solid `│` at middle col on content-bearing body row {row}, got {:?}",
        cell.symbol()
    );
}

#[test]
fn split_body_gutter_rail_is_continuous_pipe() {
    let w: u16 = 100;
    for line_numbers in [true] {
        let buffer = render_split_at_width(w, line_numbers);
        let width = buffer.area.width as usize;
        for row in 0..buffer.area.height as usize {
            let mut text = String::new();
            for col in 0..width {
                text.push_str(buffer[(col as u16, row as u16)].symbol());
            }
            let chars: Vec<char> = text.chars().collect();
            let Some(first) = chars.first() else { continue };
            if *first != '│' {
                continue;
            }

            let inner_pipes: Vec<usize> = chars
                .iter()
                .enumerate()
                .filter(|(i, c)| **c == '│' && *i != 0 && *i != width - 1)
                .map(|(i, _)| i)
                .collect();
            assert!(
                !inner_pipes.is_empty(),
                "expected at least one inner `│` (right gutter rail) on split body row {row}, got {text:?}"
            );
        }
    }
}

#[test]
fn split_empty_context_row_has_numbers_and_rails() {
    let width: u16 = 100;
    let buffer = render_split_empty_context(width);
    let w = buffer.area.width;
    let h = buffer.area.height;
    let mid = 1u16 + (width - 3) / 2;

    let mut esac_row: Option<u16> = None;
    for y in 0..h {
        let mut row = String::new();
        for x in 0..w {
            row.push_str(buffer[(x, y)].symbol());
        }

        let left: String = row.chars().take(mid as usize).collect();
        if left.contains("esac") {
            esac_row = Some(y);
            break;
        }
    }
    let esac_row = esac_row.expect("expected an `esac` context row in the split view");

    let blank_row = esac_row + 1;
    assert!(
        blank_row < h,
        "expected the blank context row at {blank_row} to fit inside the buffer height {h}"
    );

    assert_eq!(
        buffer.cell((1, blank_row)).unwrap().symbol(),
        "│",
        "row {blank_row}: expected left frame `│` at col 1"
    );
    assert_eq!(
        buffer.cell((mid, blank_row)).unwrap().symbol(),
        "│",
        "row {blank_row}: expected middle `│` divider at col {mid}"
    );

    let has_sigil = (0..w).any(|x| {
        let s = buffer.cell((x, blank_row)).unwrap().symbol();
        s == "-" || s == "+"
    });
    assert!(
        !has_sigil,
        "row {blank_row}: blank context row must not carry a `-`/`+` sigil"
    );

    let dump: String = (0..w)
        .map(|x| buffer.cell((x, blank_row)).unwrap().symbol().to_string())
        .collect();

    let has_left_rail = (2..mid).any(|x| buffer.cell((x, blank_row)).unwrap().symbol() == "│");
    assert!(
        has_left_rail,
        "blank context row {blank_row}: expected a `│` rail in the LEFT gutter \
         (between border col 1 and mid col {mid}). Dump: [{dump}]"
    );

    let right_border = (0..w)
        .rev()
        .find(|&x| buffer.cell((x, blank_row)).unwrap().symbol() == "│")
        .expect("expected a right frame `│` on the blank context row");
    let has_right_rail =
        (mid + 1..right_border).any(|x| buffer.cell((x, blank_row)).unwrap().symbol() == "│");
    assert!(
        has_right_rail,
        "blank context row {blank_row}: expected a `│` rail in the RIGHT gutter \
         (between mid col {mid} and right border {right_border}). Dump: [{dump}]"
    );

    let left_digits: String = (2..mid)
        .map(|x| buffer.cell((x, blank_row)).unwrap().symbol())
        .filter(|s| s.chars().all(|c| c.is_ascii_digit()))
        .collect();
    let right_digits: String = (mid + 1..right_border)
        .map(|x| buffer.cell((x, blank_row)).unwrap().symbol())
        .filter(|s| s.chars().all(|c| c.is_ascii_digit()))
        .collect();
    let old_num: u32 = left_digits
        .parse()
        .expect("blank context row must have digits in the LEFT number gutter");
    let new_num: u32 = right_digits
        .parse()
        .expect("blank context row must have digits in the RIGHT number gutter");
    assert!(
        old_num > 0,
        "blank context row {blank_row}: expected OLD line number > 0, got {old_num}"
    );
    assert!(
        new_num > 0,
        "blank context row {blank_row}: expected NEW line number > 0, got {new_num}"
    );

    let esac_left_rail = (2..mid).any(|x| buffer.cell((x, esac_row)).unwrap().symbol() == "│");
    let esac_right_border = (0..w)
        .rev()
        .find(|&x| buffer.cell((x, esac_row)).unwrap().symbol() == "│")
        .expect("expected a right frame `│` on the `esac` row");
    let esac_right_rail =
        (mid + 1..esac_right_border).any(|x| buffer.cell((x, esac_row)).unwrap().symbol() == "│");
    assert!(
        esac_left_rail && esac_right_rail,
        "`esac` row {esac_row}: expected continuous rails on both sides so the \
         blank row below joins without a visual gap"
    );
}
