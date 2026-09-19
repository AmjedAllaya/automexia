use automexia_extension_api::surface::*;
use automexia_ui_model::semantic_table::{Navigation, TablePresentation};

fn column(id: &str, width: u16) -> ColumnSchema {
    ColumnSchema::new(
        ColumnId::new(id).unwrap(),
        SurfaceTitle::new("Name").unwrap(),
        DataKind::Text,
    )
    .unwrap()
    .with_layout(
        ColumnWidths::new(1, width, None).unwrap(),
        0,
        Alignment::Left,
        OverflowPolicy::HorizontalScroll,
        ResponsivePolicy::Always,
    )
}

fn table(ids: &[u64]) -> SemanticTable {
    let schema = TableSchema::new(
        TableSchemaId::new("inventory").unwrap(),
        vec![column("name", 12)],
    )
    .unwrap();
    SemanticTable::new(
        schema,
        ids.iter()
            .map(|id| {
                TableRow::new(
                    RowId::new(*id).unwrap(),
                    Some(ResourceHandle::new(*id).unwrap()),
                    vec![CellValue::Text(CellText::new("界e\u{301}").unwrap())],
                )
                .unwrap()
            })
            .collect(),
    )
    .unwrap()
}

#[test]
fn visible_rows_borrow_exact_typed_cells_without_copying_or_wrapping() {
    let mut view = TablePresentation::new(table(&[8, 3, 5]));
    view.fit(1, 2);
    assert_eq!(view.visible_range(), 0..2);
    assert!(std::ptr::eq(
        view.visible_rows().as_ptr(),
        view.table().rows().as_ptr()
    ));
    assert_eq!(
        view.visible_rows()[0].cells(),
        &[CellValue::Text(CellText::new("界e\u{301}").unwrap())]
    );
    assert_eq!(view.selected(), None);
    assert!(!view.navigate(Navigation::Move(0)));
    assert!(!view.navigate(Navigation::Page(0)));
    assert_eq!(view.selected(), None);
    view.navigate(Navigation::ScrollColumns(isize::MAX));
    assert_eq!(view.column_offset(), 11);
    assert_eq!(view.visible_rows()[1].id().get(), 3);
    view.fit(100, 3);
    assert_eq!(view.column_offset(), 0);
    assert_eq!(view.visible_range(), 0..3);
}

#[test]
fn navigation_is_bounded_for_empty_tiny_and_maximum_tables() {
    for count in [0, 1, 100, 2_000, MAX_TABLE_ROWS] {
        let ids = (1..=count as u64).collect::<Vec<_>>();
        let mut view = TablePresentation::new(table(&ids));
        view.fit(1, 1);
        view.navigate(Navigation::Last);
        assert_eq!(
            view.selected().map(|row| row.id().get()),
            ids.last().copied()
        );
        assert_eq!(view.visible_rows().len(), usize::from(count > 0));
        view.navigate(Navigation::Move(isize::MAX));
        view.navigate(Navigation::Page(isize::MAX));
        assert_eq!(
            view.selected().map(|row| row.id().get()),
            ids.last().copied()
        );
        view.navigate(Navigation::Move(isize::MIN));
        assert_eq!(
            view.selected().map(|row| row.id().get()),
            ids.first().copied()
        );
        assert!(!view.navigate(Navigation::Select(usize::MAX)));
        view.fit(0, 0);
        assert!(view.visible_rows().is_empty());
        view.fit(usize::MAX, usize::MAX);
        assert!(view.visible_rows().len() <= 1024);
        assert_eq!(view.column_offset(), 0);
    }
}

#[test]
fn refresh_preserves_selected_identity_and_independent_scrolled_anchor() {
    let mut view = TablePresentation::new(table(&[1, 2, 3, 4, 5]));
    view.fit(12, 2);
    view.navigate(Navigation::Select(1));
    view.navigate(Navigation::ScrollRows(2));
    assert_eq!(view.visible_rows()[0].id().get(), 3);
    view.replace(table(&[5, 4, 2, 3, 1]));
    assert_eq!(view.selected().unwrap().id().get(), 2);
    assert_eq!(view.visible_rows()[0].id().get(), 3);
    view.replace(table(&[5, 4, 3, 1]));
    assert_eq!(view.selected(), None); // Never silently retarget a deleted resource.
    view.navigate(Navigation::First);
    view.replace(table(&[]));
    assert_eq!(view.selected(), None);
    assert!(view.visible_rows().is_empty());
    view.replace(table(&[10]));
    assert_eq!(view.selected(), None);
}

#[test]
fn reused_resource_handles_and_new_schema_cannot_inherit_selection() {
    let mut view = TablePresentation::new(table(&[1, 2]));
    view.fit(4, 1);
    view.navigate(Navigation::Last);
    let original = table(&[1, 2]);
    let replacement = SemanticTable::new(
        original.schema().clone(),
        vec![TableRow::new(
            RowId::new(2).unwrap(),
            Some(ResourceHandle::new(99).unwrap()),
            vec![CellValue::Missing],
        )
        .unwrap()],
    )
    .unwrap();
    view.replace(replacement);
    assert_eq!(view.selected(), None);
    assert_eq!(view.visible_rows()[0].cells(), &[CellValue::Missing]);
    view.navigate(Navigation::First);
    let original = table(&[2]);
    let schema = TableSchema::new(
        TableSchemaId::new("different").unwrap(),
        original.schema().columns().to_vec(),
    )
    .unwrap();
    view.replace(SemanticTable::new(schema, original.rows().to_vec()).unwrap());
    assert_eq!(view.selected(), None);
}

#[test]
fn fixed_seed_resize_navigation_keeps_exact_order_and_valid_selection() {
    let ids = (1..=2_000).collect::<Vec<_>>();
    let mut view = TablePresentation::new(table(&ids));
    let mut seed = 17_u64;
    for step in 0..4096 {
        seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
        view.fit((seed as usize) % 200, ((seed >> 16) as usize) % 80);
        let delta = ((seed >> 32) as i32 % 100) as isize;
        view.navigate(if step % 2 == 0 {
            Navigation::Move(delta)
        } else {
            Navigation::ScrollRows(delta)
        });
        let range = view.visible_range();
        assert_eq!(view.visible_rows().len(), range.len());
        for (row, id) in view.visible_rows().iter().zip(&ids[range]) {
            assert_eq!(row.id().get(), *id);
        }
        assert!(view
            .selected()
            .is_none_or(|row| ids.contains(&row.id().get())));
    }
}

#[test]
fn column_geometry_uses_schema_widths_and_separators_not_cell_text() {
    let base = table(&[1]);
    let second = column("other", 7);
    let schema = TableSchema::new(
        TableSchemaId::new("wide").unwrap(),
        vec![base.schema().columns()[0].clone(), second],
    )
    .unwrap();
    let data = SemanticTable::new(
        schema,
        vec![TableRow::new(
            RowId::new(1).unwrap(),
            None,
            vec![
                CellValue::Missing,
                CellValue::Text(CellText::new("long value remains whole").unwrap()),
            ],
        )
        .unwrap()],
    )
    .unwrap();
    let mut view = TablePresentation::new(data);
    view.fit(2, 1);
    assert_eq!(view.content_width(), 20);
    view.navigate(Navigation::ScrollColumns(12));
    let columns = view.visible_columns().collect::<Vec<_>>();
    assert_eq!(columns.len(), 1);
    assert_eq!(
        (
            columns[0].index,
            columns[0].screen_column,
            columns[0].cell_offset,
            columns[0].visible_width
        ),
        (1, 1, 0, 1)
    );
    view.navigate(Navigation::ScrollColumns(isize::MAX));
    let column = view.visible_columns().next().unwrap();
    assert_eq!(
        (column.index, column.cell_offset, column.visible_width),
        (1, 5, 2)
    );
    view.fit(0, 1);
    assert_eq!(view.visible_columns().count(), 0);
}

#[test]
fn maximum_schema_width_and_repeated_replacement_remain_bounded() {
    let schema = TableSchema::new(
        TableSchemaId::new("maximum").unwrap(),
        (0..MAX_TABLE_COLUMNS)
            .map(|index| column(&format!("c{index}"), u16::MAX))
            .collect(),
    )
    .unwrap();
    let mut view =
        TablePresentation::new(SemanticTable::new(schema.clone(), vec![]).unwrap());
    for id in 1..=128 {
        view.replace(
            SemanticTable::new(
                schema.clone(),
                vec![TableRow::new(
                    RowId::new(id).unwrap(),
                    None,
                    vec![CellValue::Missing; MAX_TABLE_COLUMNS],
                )
                .unwrap()],
            )
            .unwrap(),
        );
        assert!(view.selected().is_none());
        view.fit(usize::MAX, 1);
        view.navigate(Navigation::Last);
        view.navigate(Navigation::ScrollColumns(isize::MAX));
        assert_eq!(view.content_width(), 64 * 65_535 + 63);
        let columns = view.visible_columns().collect::<Vec<_>>();
        assert_eq!(columns.len(), 1);
        assert_eq!((columns[0].index, columns[0].visible_width), (63, 16_384));
        assert_eq!(view.table().rows().len(), 1);
        assert_eq!(view.selected().unwrap().id().get(), id);
    }
}

#[test]
fn paging_scroll_and_clear_have_distinct_non_wrapping_selection_semantics() {
    let mut view = TablePresentation::new(table(&[1, 2, 3, 4, 5, 6]));
    view.fit(4, 2);
    assert!(view.navigate(Navigation::First));
    assert!(view.navigate(Navigation::Page(1)));
    assert_eq!(view.selected().unwrap().id().get(), 3);
    assert_eq!(view.visible_range(), 1..3);
    view.navigate(Navigation::Page(-1));
    assert_eq!(view.selected().unwrap().id().get(), 1);
    assert!(!view.navigate(Navigation::Move(-1)));
    view.navigate(Navigation::Last);
    view.navigate(Navigation::ScrollRows(isize::MIN));
    assert_eq!(view.visible_range(), 0..2);
    assert_eq!(view.selected().unwrap().id().get(), 6);
    view.navigate(Navigation::Page(isize::MIN));
    assert_eq!(view.selected().unwrap().id().get(), 1);
    view.navigate(Navigation::ScrollColumns(isize::MAX));
    view.navigate(Navigation::ScrollColumns(isize::MIN));
    assert_eq!(view.column_offset(), 0);
    assert!(view.navigate(Navigation::Clear));
    assert!(!view.navigate(Navigation::Clear));
    assert_eq!(view.selected(), None);
}
