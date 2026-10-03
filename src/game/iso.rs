//! Helpers for the isometric map: conversion between world positions and map
//! cells, depth sorting and walkability.

use bevy::prelude::*;

use crate::plugins::tiled::TiledMap;

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
                    if !image.to_string_lossy().contains("Glow_Floor") {
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

    pub fn is_walkable(&self, pos: Vec2) -> bool {
        let cell = world_to_cell(pos).round().as_ivec2();
        cell.x >= 0
            && cell.y >= 0
            && cell.x < self.width
            && cell.y < self.height
            && self.cells[(cell.y * self.width + cell.x) as usize]
    }
}
