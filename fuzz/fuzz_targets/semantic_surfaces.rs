#![no_main]

use automexia_extension_api::surface::{
    SemanticTable, SurfaceUpdate, MAX_SURFACE_FRAME_BYTES, MAX_TABLE_CELLS,
    MAX_TABLE_ROWS, MAX_TABLE_TEXT_BYTES,
};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|bytes: &[u8]| {
    if bytes.len() > MAX_SURFACE_FRAME_BYTES {
        return;
    }
    if let Ok(table) = serde_json::from_slice::<SemanticTable>(bytes) {
        assert!(table.rows().len() <= MAX_TABLE_ROWS);
        assert!(table.cell_count() <= MAX_TABLE_CELLS);
        assert!(table.text_bytes() <= MAX_TABLE_TEXT_BYTES);
        let encoded = serde_json::to_vec(&table).unwrap();
        assert_eq!(
            serde_json::from_slice::<SemanticTable>(&encoded).unwrap(),
            table
        );
    }
    if let Ok(update) = serde_json::from_slice::<SurfaceUpdate>(bytes) {
        let encoded = serde_json::to_vec(&update).unwrap();
        assert_eq!(
            serde_json::from_slice::<SurfaceUpdate>(&encoded).unwrap(),
            update
        );
    }
});
