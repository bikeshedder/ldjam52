//! Helpers for the isometric map: conversion between world positions and map
//! cells, depth sorting and walkability.

use bevy::prelude::*;

use crate::plugins::tiled::{TiledMap, is_flat_image, is_tabletop_image};

/// Offset from a character sprite's center to its feet.
pub const FEET_OFFSET: Vec2 = Vec2::new(0.0, -56.0);

/// World position of the center of the floor diamond of the given cell.
pub fn cell_center(cell: Vec2) -> Vec2 {
    Vec2::new(
        (cell.x - cell.y) * 64.0 + 64.0,
        36.0 - (cell.x + cell.y) * 32.0,
    )
}

/// Fractional map cell for the given world position. Cell centers map to
/// integer coordinates.
pub fn world_to_cell(pos: Vec2) -> Vec2 {
    let u = (pos.x - 64.0) / 64.0;
    let v = (36.0 - pos.y) / 32.0;
    Vec2::new((u + v) / 2.0, (v - u) / 2.0)
}

/// Translation for a character standing with its feet in the given cell.
pub fn character_translation(cell: Vec2) -> Vec3 {
    let pos = cell_center(cell) - FEET_OFFSET;
    pos.extend(depth_at(pos + FEET_OFFSET))
}

/// Depth (z) for an upright sprite whose base touches the given world position.
pub fn depth_at(feet: Vec2) -> f32 {
    let cell = world_to_cell(feet);
    crate::plugins::tiled::upright_depth(cell.x + cell.y, 0.5)
}

/// Which map cells can be walked on.
#[derive(Resource, Default)]
pub struct Walkable {
    width: i32,
    height: i32,
    cells: Vec<bool>,
}

impl Walkable {
    pub fn from_map(map: &TiledMap) -> Self {
        let map = &map.map;
        let (width, height) = (map.width as i32, map.height as i32);
        let mut floor = vec![false; (width * height) as usize];
        let mut blocked = vec![false; (width * height) as usize];
        for layer in map.layers() {
            let tiled::LayerType::Tiles(tiled::TileLayer::Finite(data)) = layer.layer_type() else {
                continue;
            };
            let is_floor = matches!(layer.name.as_str(), "Floor" | "Carpet");
            for y in 0..height {
                for x in 0..width {
                    let Some(tile) = data.get_tile(x, y) else {
                        continue;
                    };
                    let index = (y * width + x) as usize;
                    if is_floor {
                        floor[index] = true;
                        continue;
                    }
                    let image = tile
                        .get_tile()
                        .and_then(|t| t.image.as_ref().map(|i| i.source.clone()))
                        .unwrap_or_default();
                    // Flat decoration, paintings hanging on the wall behind and
                    // doors in front of walls don't block the cell.
                    let image = image.to_string_lossy();
                    if !(is_flat_image(&image)
                        || image.contains("Painting")
                        || image.contains("Door")
                        || is_tabletop_image(&image))
                    {
                        blocked[index] = true;
                    }
                }
            }
        }
        Self {
            width,
            height,
            cells: floor.iter().zip(&blocked).map(|(f, b)| *f && !b).collect(),
        }
    }

    /// Whether a character can stand at the given world position. It keeps
    /// `margin` (in map cells) of distance to blocked cells behind it, so it doesn't
    /// overlap walls and furniture drawn behind it. Things in front of the
    /// character are drawn over it, so no distance is needed there.
    pub fn is_free(&self, pos: Vec2, margin: f32) -> bool {
        let center = world_to_cell(pos);
        let diagonal = margin * std::f32::consts::FRAC_1_SQRT_2;
        [
            Vec2::ZERO,
            Vec2::new(-margin, 0.0),
            Vec2::new(0.0, -margin),
            Vec2::new(-diagonal, -diagonal),
        ]
        .into_iter()
        .all(|offset| self.is_walkable_cell(center + offset))
    }

    fn is_walkable_cell(&self, cell: Vec2) -> bool {
        let cell = cell.round().as_ivec2();
        cell.x >= 0
            && cell.y >= 0
            && cell.x < self.width
            && cell.y < self.height
            && self.cells[(cell.y * self.width + cell.x) as usize]
    }
}
