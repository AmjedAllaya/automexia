//! Bounded, immediate-mode terminal backgrounds between grid cells and glyphs.
//! Uses the ordinary rectangle compositor and shared frame buffers; it never
//! records text, changes cells, or survives frame finalization/discard.

use super::{batch, Compositor, Rect, Vertex};

const MAX_RECTS: usize = 8192;

#[derive(Default)]
pub(super) struct UnderText {
    comp: Compositor,
    count: usize,
}

impl UnderText {
    pub(super) fn rect(&mut self, rect: [f32; 4], color: [f32; 4]) {
        let [x, y, width, height] = rect;
        if self.count >= MAX_RECTS
            || !rect
                .iter()
                .chain(color.iter())
                .all(|value| value.is_finite())
            || !(x + width).is_finite()
            || !(y + height).is_finite()
            || width <= 0.0
            || height <= 0.0
            || color[3] <= 0.0
        {
            return;
        }
        self.comp.batches.rect(
            &Rect {
                x,
                y,
                width,
                height,
            },
            0.0,
            &color.map(|channel| channel.clamp(0.0, 1.0)),
            0,
        );
        self.count += 1;
    }

    pub(super) fn finish(
        &mut self,
        instances: &mut Vec<batch::QuadInstance>,
        vertices: &mut Vec<Vertex>,
        commands: &mut Vec<batch::DrawCmd>,
    ) {
        self.comp.finish(instances, vertices, commands);
        self.count = 0;
    }

    pub(super) fn clear(&mut self) {
        self.comp.batches.reset();
        self.count = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn drain(phase: &mut UnderText) -> Vec<batch::QuadInstance> {
        let (mut instances, mut vertices, mut commands) =
            (Vec::new(), Vec::new(), Vec::new());
        phase.finish(&mut instances, &mut vertices, &mut commands);
        assert!(vertices.is_empty(), "the phase accepts rectangles only");
        instances
    }

    #[test]
    fn terminal_background_phase_is_bounded_and_rejects_invalid_geometry() {
        let mut phase = UnderText::default();
        for rect in [
            [f32::NAN, 0.0, 2.0, 2.0],
            [0.0, f32::INFINITY, 2.0, 2.0],
            [0.0, 0.0, -1.0, 2.0],
            [0.0, 0.0, 2.0, 0.0],
            [f32::MAX, 0.0, f32::MAX, 2.0],
        ] {
            phase.rect(rect, [1.0; 4]);
        }
        phase.rect([0.0, 0.0, 2.0, 2.0], [0.0; 4]);
        phase.rect([0.0, 0.0, 2.0, 2.0], [1.0, 0.0, 0.0, f32::NAN]);
        assert!(drain(&mut phase).is_empty());
        for row in 0..MAX_RECTS + 1 {
            phase.rect([0.0, row as f32, 2.0, 1.0], [1.0; 4]);
        }
        let instances = drain(&mut phase);
        assert_eq!(instances.len(), MAX_RECTS);
        assert!(instances.iter().all(|rect| rect.pos[1] < MAX_RECTS as f32));
        phase.rect([0.0, 0.0, 2.0, 2.0], [1.0; 4]);
        assert_eq!(drain(&mut phase).len(), 1, "each frame has its own budget");
    }

    #[test]
    fn terminal_backgrounds_do_not_survive_finish_or_discard() {
        let mut phase = UnderText::default();
        phase.rect([0.0, 0.0, 2.0, 2.0], [1.0; 4]);
        assert_eq!(drain(&mut phase).len(), 1);
        assert!(drain(&mut phase).is_empty());
        phase.rect([0.0, 0.0, 2.0, 2.0], [1.0; 4]);
        phase.clear();
        assert!(drain(&mut phase).is_empty());
        phase.rect([0.0, 0.0, 2.0, 2.0], [2.0, -1.0, 0.5, 1.0]);
        assert_eq!(drain(&mut phase)[0].color, [1.0, 0.0, 0.5, 1.0]);
    }
}
