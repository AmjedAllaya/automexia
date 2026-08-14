// Copyright (c) 2023-present, Raphael Amorim.
//
// This source code is licensed under the MIT license found in the
// LICENSE file in the root directory of this source tree.

// The graphic value types (GraphicData, GraphicId, ColorType, Resize*,
// GraphicOverlay, the image-key hashers) now live in the leaf crate
// `rio-graphics` so the terminal core can model images without pulling
// the renderer. They are re-exported here so `sugarloaf::graphics::*` and
// `sugarloaf::GraphicData` keep resolving. Only the render cache
// (GraphicDataEntry / Graphics), which owns a GPU-uploadable Handle,
// stays in sugarloaf.

use crate::sugarloaf::Handle;
use rustc_hash::FxHashMap;

pub use rio_graphics::{
    atlas_image_key, kitty_image_key, ColorType, Graphic, GraphicData, GraphicId,
    GraphicOverlay, ResizeCommand, ResizeParameter, MAX_GRAPHIC_DIMENSIONS,
};

pub struct GraphicDataEntry {
    pub handle: Handle,
    pub width: f32,
    pub height: f32,
    pub transmit_time: std::time::Instant,
}

impl GraphicDataEntry {
    /// Create from a GraphicData, taking ownership of pixel data.
    pub fn from_graphic_data(data: GraphicData) -> Self {
        let display_w = data.display_width.unwrap_or(data.width) as f32;
        let display_h = data.display_height.unwrap_or(data.height) as f32;
        Self {
            handle: Handle::from_pixels(
                data.width as u32,
                data.height as u32,
                data.pixels,
            ),
            width: display_w,
            height: display_h,
            transmit_time: data.transmit_time,
        }
    }

    /// Create a cache-addressed RGBA entry without copying or re-hashing pixels.
    pub fn from_shared_rgba(
        content_id: u64,
        width: u32,
        height: u32,
        pixels: std::sync::Arc<[u8]>,
        transmit_time: std::time::Instant,
    ) -> Self {
        Self {
            handle: Handle::from_pixels_with_id(content_id, width, height, pixels),
            width: width as f32,
            height: height as f32,
            transmit_time,
        }
    }
}

#[derive(Default)]
pub struct Graphics {
    inner: FxHashMap<GraphicId, GraphicDataEntry>,
}

impl Graphics {
    #[inline]
    pub fn get(&self, id: &GraphicId) -> Option<&GraphicDataEntry> {
        self.inner.get(id)
    }

    #[inline]
    pub fn insert(&mut self, graphic_data: GraphicData) {
        // Check if existing entry has the same generation (skip re-upload)
        if let Some(existing) = self.inner.get(&graphic_data.id) {
            if existing.transmit_time == graphic_data.transmit_time {
                return;
            }
        }

        let display_w = graphic_data.display_width.unwrap_or(graphic_data.width) as f32;
        let display_h = graphic_data.display_height.unwrap_or(graphic_data.height) as f32;
        self.inner.insert(
            graphic_data.id,
            GraphicDataEntry {
                handle: Handle::from_pixels(
                    graphic_data.width as u32,
                    graphic_data.height as u32,
                    graphic_data.pixels,
                ),
                width: display_w,
                height: display_h,
                transmit_time: graphic_data.transmit_time,
            },
        );
    }

    #[inline]
    pub fn remove(&mut self, graphic_id: &GraphicId) {
        self.inner.remove(graphic_id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shared_rgba_preserves_pixel_allocation_and_stable_identity() {
        let pixels: std::sync::Arc<[u8]> = std::sync::Arc::from(vec![1_u8, 2, 3, 255]);
        let address = pixels.as_ptr();
        let now = std::time::Instant::now();
        let entry =
            GraphicDataEntry::from_shared_rgba(0xFFFF_FFFE_0000_002A, 1, 1, pixels, now);

        assert_eq!(entry.handle.id(), 0xFFFF_FFFE_0000_002A);
        assert_eq!(entry.transmit_time, now);
        match entry.handle.data() {
            crate::components::core::image::Data::Rgba {
                width,
                height,
                pixels,
            } => {
                assert_eq!((*width, *height), (1, 1));
                assert_eq!(pixels.as_ref().as_ptr(), address);
            }
            other => panic!("expected shared RGBA data, got {other:?}"),
        }
    }
}
