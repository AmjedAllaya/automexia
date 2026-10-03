use super::*;
use rio_backend::config::presentation::{
    TableAppearance, TableBanding, TableBorderStyle,
};

fn appearance(banding: TableBanding) -> TableAppearance {
    TableAppearance {
        banding: Some(banding),
        header_background: Some(Rgba::from_bytes([10, 20, 30, 255])),
        body_background: Some(Rgba::from_bytes([40, 50, 60, 255])),
        alternate_background: Some(Rgba::from_bytes([70, 80, 90, 255])),
        border_style: Some(TableBorderStyle::None),
        ..Default::default()
    }
}

#[test]
fn inherited_table_text_stays_readable_on_custom_backgrounds() {
    let content = content(&mut terminal(
        "NAME  VALUE\r\nalpha  glyphs\r\n\r\nprompt ",
        80,
        12,
    ));
    for rgb in [[250, 244, 232], [8, 14, 20]] {
        let tables = TableAppearance {
            header_background: Some(Rgba::from_bytes([rgb[0], rgb[1], rgb[2], 255])),
            body_background: Some(Rgba::from_bytes([rgb[0], rgb[1], rgb[2], 255])),
            ..Default::default()
        };
        let canvas = render_with_options(
            &content,
            1.0,
            8.0,
            14.0,
            PaintOptions {
                tables,
                ..Default::default()
            },
        );
        assert!(!canvas.text.instances().is_empty());
        let bg = [
            f32::from(rgb[0]) / 255.0,
            f32::from(rgb[1]) / 255.0,
            f32::from(rgb[2]) / 255.0,
            1.0,
        ];
        for glyph in canvas.text.instances() {
            let fg = glyph.color.map(|n| f32::from(n) / 255.0);
            assert!(
                automexia_ui_model::contrast_ratio(fg, bg) >= 4.5,
                "inherited text must remain legible: {rgb:?}"
            );
        }
    }
}

#[test]
fn table_glyphs_follow_the_terminal_baseline_with_dense_borders() {
    use rio_backend::config::presentation::TableBorderWeight;
    let content = content(&mut terminal(
        "HHH   VALUE\r\njpqy  123\r\n\r\nprompt ",
        80,
        12,
    ));
    assert_eq!(content.inline_tables.surfaces.len(), 1);
    for scale in [1.0, 1.25, 2.0, 4.0] {
        for border_style in [
            TableBorderStyle::Solid,
            TableBorderStyle::Dashed,
            TableBorderStyle::Double,
        ] {
            let mut canvas = RasterCanvas::new(scale);
            draw(
                &mut canvas,
                &content,
                [8.0, 8.0, 14.0, 28.0],
                20.0,
                scale,
                PaintOptions {
                    cell_baseline: Some(6.0),
                    tables: TableAppearance {
                        border_style: Some(border_style),
                        border_weight: Some(TableBorderWeight::Thick),
                        ..Default::default()
                    },
                    ..Default::default()
                },
                source_colors,
            );
            let glyph = canvas
                .text
                .instances()
                .first()
                .expect("visible header glyph");
            let bottom = (glyph.pos[1]
                + f32::from(glyph.bearings[1])
                + glyph.glyph_size[1] as f32)
                / scale;
            assert!(
                (bottom - 30.0).abs() <= 1.0,
                "capital must sit on the canonical baseline: {bottom}"
            );
        }
    }
}

#[test]
fn table_contrast_does_not_rewrite_explicit_text_or_ansi_colors() {
    let content = content(&mut terminal(
        "NAME  VALUE\r\n\x1b[31malpha\x1b[0m  123\r\n\r\nprompt ",
        80,
        12,
    ));
    let canvas = render_with_options(
        &content,
        1.0,
        8.0,
        14.0,
        PaintOptions {
            tables: TableAppearance {
                header_foreground: Some(Rgb::from_bytes([13, 35, 57])),
                body_foreground: Some(Rgb::from_bytes([21, 43, 65])),
                header_background: Some(Rgba::from_bytes([250, 244, 232, 255])),
                body_background: Some(Rgba::from_bytes([250, 244, 232, 255])),
                ..Default::default()
            },
            ..Default::default()
        },
    );
    for expected in [[13, 35, 57, 255], [21, 43, 65, 255], [255, 0, 0, 255]] {
        assert!(canvas
            .text
            .instances()
            .iter()
            .any(|glyph| glyph.color == expected));
    }
}

#[test]
fn table_banding_follows_logical_rows_and_columns_across_wrap_and_rulers() {
    use automexia_ui_model::tables::TableRowKind;
    for source in [
        "NAME   VALUE\r\nalpha  very-long-cell-value\r\ngamma  another-long-value\r\nthird  last\r\n\r\nprompt ",
        "| NAME | VALUE |\r\n| --- | --- |\r\n| alpha | very-long-cell-value |\r\n| gamma | another-long-value |\r\n| third | last |\r\n\r\nprompt ",
    ] {
        for columns in [22, 80] {
            let mut terminal = terminal(source, columns, 30);
            terminal.scroll_display(Scroll::Top);
            let content = content(&mut terminal);
            assert_eq!(content.inline_tables.surfaces.len(), 1);
            for banding in [TableBanding::None, TableBanding::Rows, TableBanding::Columns, TableBanding::Checkerboard] {
                let canvas = render_with_options(&content, 1.0, 8.0, 14.0, PaintOptions { tables: appearance(banding), ..Default::default() });
                let surface = &content.inline_tables.surfaces[0];
                let mut data = 0;
                for (ri, row) in surface.layout.rows.iter().enumerate() {
                    if row.kind == TableRowKind::Rule { continue; }
                    let header = row.kind == TableRowKind::Header;
                    let (first, height) = content.inline_tables.row_geometry(0, ri, &content.command_rows).unwrap();
                    for (ci, column) in surface.layout.columns.iter().enumerate() {
                        let alternate = match banding {
                            TableBanding::None => false,
                            TableBanding::Rows => data % 2 == 1,
                            TableBanding::Columns => ci % 2 == 1,
                            TableBanding::Checkerboard => data % 2 != ci % 2,
                        };
                        let rgb = if header { [10u8, 20, 30] } else if alternate { [70, 80, 90] } else { [40, 50, 60] };
                        for line in 0..height {
                            let y = 8.0 + (first + line as isize) as f32 * 20.0 + 8.0;
                            for x in [column.x as f32 * 8.0 + 8.5, column.content_x as f32 * 8.0 + 8.5] {
                                let actual = painted_background(&canvas, x, y);
                                for channel in 0..3 { close(actual[channel], f32::from(rgb[channel]) / 255.0); }
                            }
                        }
                    }
                    if !header { data += 1; }
                }
                assert_eq!(data, 3);
            }
        }
    }
}

#[test]
fn table_decoration_cannot_overwrite_kubernetes_ansi_or_selection() {
    let mut term = terminal("NAME  READY  STATUS\r\napi   0/1    Unknown\r\n\x1b[41mjobs  1/1    Running\x1b[0m\r\n\r\nprompt ", 80, 12);
    let mut content = content(&mut term);
    content.selection_range = Some(SelectionRange::new(
        Pos::new(Line(1), Column(0)),
        Pos::new(Line(1), Column(1)),
        false,
    ));
    let colors = Colors::default();
    let canvas = render_with_options(
        &content,
        1.0,
        8.0,
        14.0,
        PaintOptions {
            tables: appearance(TableBanding::Checkerboard),
            kubernetes_highlight: Some(HighlightAppearance {
                style: HighlightStyle::Both,
                warning_background: Some(Rgba::from_bytes([12, 100, 210, 255])),
                ..Default::default()
            }),
            ..Default::default()
        },
    );
    let surface = &content.inline_tables.surfaces[0];
    let x = 8.5 + surface.layout.columns[0].content_x as f32 * 8.0;
    assert_eq!(
        painted_background(&canvas, x, 35.5),
        colors.selection_background
    );
    let status_x = 8.5 + surface.layout.columns[2].content_x as f32 * 8.0;
    assert_eq!(
        painted_background(&canvas, status_x, 35.5),
        [12.0 / 255.0, 100.0 / 255.0, 210.0 / 255.0, 1.0]
    );
    assert_eq!(painted_background(&canvas, x, 55.5), [1.0, 0.0, 0.0, 1.0]);
}

#[test]
fn table_border_visibility_and_patterns_reach_actual_renderer() {
    let content = simple(80);
    for style in [
        TableBorderStyle::None,
        TableBorderStyle::Solid,
        TableBorderStyle::Dashed,
        TableBorderStyle::Dotted,
        TableBorderStyle::Double,
    ] {
        let table = TableAppearance {
            border_style: Some(style),
            border_color: Some(Rgba::from_bytes([255, 0, 255, 255])),
            ..Default::default()
        };
        let mut canvas = render_with_options(
            &content,
            1.25,
            8.0,
            14.0,
            PaintOptions {
                tables: table,
                ..Default::default()
            },
        );
        let rules: Vec<_> = canvas
            .rects
            .iter()
            .filter(|(_, color)| *color == [1.0, 0.0, 1.0, 1.0])
            .collect();
        assert_eq!(rules.is_empty(), style == TableBorderStyle::None);
        assert_ne!(
            canvas.pixels(660, 260, false),
            canvas.pixels(660, 260, true),
            "text survives every border style"
        );
        let mut off = table;
        off.outer_border = Some(false);
        off.row_lines = Some(false);
        off.column_lines = Some(false);
        off.header_separator = Some(false);
        let hidden = render_with_options(
            &content,
            1.25,
            8.0,
            14.0,
            PaintOptions {
                tables: off,
                ..Default::default()
            },
        );
        assert!(!hidden
            .rects
            .iter()
            .any(|(_, color)| *color == [1.0, 0.0, 1.0, 1.0]));
    }
}
