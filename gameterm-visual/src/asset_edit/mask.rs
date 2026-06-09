use super::{
    SceneAssetEditError, SceneAssetFeatureMap, SceneAssetNormalizedPoint, SceneAssetPixelRect,
};
use std::collections::VecDeque;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SceneAssetMask {
    pub(crate) width: u32,
    pub(crate) height: u32,
    pub(crate) pixels: Vec<bool>,
}

impl SceneAssetMask {
    pub(crate) fn from_pixels(width: u32, height: u32, pixels: Vec<bool>) -> Self {
        debug_assert_eq!(pixels.len(), width as usize * height as usize);
        Self {
            width,
            height,
            pixels,
        }
    }

    pub(crate) fn pixels(&self) -> &[bool] {
        &self.pixels
    }

    pub(crate) fn len(&self) -> usize {
        self.pixels.len()
    }

    pub(crate) fn selected_count(&self) -> usize {
        selected_pixel_count(&self.pixels)
    }

    pub(crate) fn union_pixels(&mut self, pixels: &[bool]) {
        if pixels.len() != self.pixels.len() {
            return;
        }
        for (target, selected) in self.pixels.iter_mut().zip(pixels.iter().copied()) {
            *target |= selected;
        }
    }

    pub(crate) fn intersect_pixels(&mut self, pixels: &[bool]) {
        if pixels.len() != self.pixels.len() {
            return;
        }
        for (target, selected) in self.pixels.iter_mut().zip(pixels.iter().copied()) {
            *target &= selected;
        }
    }

    pub(crate) fn eroded(&self, radius: u32) -> Self {
        if radius == 0 {
            return self.clone();
        }
        let radius = radius as i32;
        let mut pixels = vec![false; self.pixels.len()];
        for y in 0..self.height {
            for x in 0..self.width {
                let index = mask_index(self.width, x, y);
                if !self.pixels[index] {
                    continue;
                }
                let mut keep = true;
                'neighbors: for dy in -radius..=radius {
                    for dx in -radius..=radius {
                        let nx = x as i32 + dx;
                        let ny = y as i32 + dy;
                        if nx < 0 || ny < 0 || nx >= self.width as i32 || ny >= self.height as i32 {
                            continue;
                        }
                        if !self.pixels[mask_index(self.width, nx as u32, ny as u32)] {
                            keep = false;
                            break 'neighbors;
                        }
                    }
                }
                pixels[index] = keep;
            }
        }
        Self::from_pixels(self.width, self.height, pixels)
    }

    pub(crate) fn dilated(&self, radius: u32) -> Self {
        if radius == 0 {
            return self.clone();
        }
        let radius = radius as i32;
        let mut pixels = vec![false; self.pixels.len()];
        for y in 0..self.height {
            for x in 0..self.width {
                let mut selected = false;
                'neighbors: for dy in -radius..=radius {
                    for dx in -radius..=radius {
                        let nx = x as i32 + dx;
                        let ny = y as i32 + dy;
                        if nx < 0 || ny < 0 || nx >= self.width as i32 || ny >= self.height as i32 {
                            continue;
                        }
                        if self.pixels[mask_index(self.width, nx as u32, ny as u32)] {
                            selected = true;
                            break 'neighbors;
                        }
                    }
                }
                pixels[mask_index(self.width, x, y)] = selected;
            }
        }
        Self::from_pixels(self.width, self.height, pixels)
    }

    pub(crate) fn opened(&self, radius: u32) -> Self {
        self.eroded(radius).dilated(radius)
    }

    pub(crate) fn closed(&self, radius: u32) -> Self {
        self.dilated(radius).eroded(radius)
    }

    pub(crate) fn without_small_components(&self, min_size: usize) -> Self {
        if min_size == 0 {
            return self.clone();
        }
        let mut pixels = self.pixels.clone();
        for component in self.selected_components() {
            if component.len() < min_size {
                for index in component {
                    pixels[index] = false;
                }
            }
        }
        Self::from_pixels(self.width, self.height, pixels)
    }

    pub(crate) fn with_filled_small_holes(&self, max_size: usize) -> Self {
        if max_size == 0 {
            return self.clone();
        }
        let mut visited = vec![false; self.pixels.len()];
        let mut pixels = self.pixels.clone();
        for y in 0..self.height {
            for x in 0..self.width {
                let start = mask_index(self.width, x, y);
                if self.pixels[start] || visited[start] {
                    continue;
                }
                let (component, touches_edge) = self.unselected_component(start, &mut visited);
                if !touches_edge && component.len() <= max_size {
                    for index in component {
                        pixels[index] = true;
                    }
                }
            }
        }
        Self::from_pixels(self.width, self.height, pixels)
    }

    pub(crate) fn protect_feature_regions(
        &mut self,
        feature_map: &SceneAssetFeatureMap,
        region_names: &[String],
    ) -> Result<(), SceneAssetEditError> {
        for region in region_names {
            let trimmed = region.trim();
            if trimmed.is_empty() {
                continue;
            }
            self.protect_rect(feature_map.pixel_region(trimmed, self.width, self.height)?);
        }
        Ok(())
    }

    fn protect_rect(&mut self, rect: SceneAssetPixelRect) {
        for y in rect.y..rect.bottom().min(self.height) {
            for x in rect.x..rect.right().min(self.width) {
                self.pixels[mask_index(self.width, x, y)] = false;
            }
        }
    }

    pub(crate) fn select_rect(&mut self, rect: SceneAssetPixelRect) {
        for y in rect.y..rect.bottom().min(self.height) {
            for x in rect.x..rect.right().min(self.width) {
                self.pixels[mask_index(self.width, x, y)] = true;
            }
        }
    }

    pub(crate) fn select_polygon(
        &mut self,
        polygon: &[SceneAssetNormalizedPoint],
    ) -> Result<(), SceneAssetEditError> {
        validate_polygon(polygon)?;
        for y in 0..self.height {
            for x in 0..self.width {
                let point = SceneAssetNormalizedPoint {
                    x: (x as f32 + 0.5) / self.width.max(1) as f32,
                    y: (y as f32 + 0.5) / self.height.max(1) as f32,
                };
                if point_in_polygon(point, polygon) {
                    self.pixels[mask_index(self.width, x, y)] = true;
                }
            }
        }
        Ok(())
    }

    fn selected_components(&self) -> Vec<Vec<usize>> {
        let mut visited = vec![false; self.pixels.len()];
        let mut components = Vec::new();
        for y in 0..self.height {
            for x in 0..self.width {
                let start = mask_index(self.width, x, y);
                if !self.pixels[start] || visited[start] {
                    continue;
                }
                let mut component = Vec::new();
                let mut queue = VecDeque::from([start]);
                visited[start] = true;
                while let Some(index) = queue.pop_front() {
                    component.push(index);
                    let (cx, cy) = mask_xy(self.width, index);
                    for (nx, ny) in mask_neighbors(self.width, self.height, cx, cy) {
                        let neighbor_index = mask_index(self.width, nx, ny);
                        if self.pixels[neighbor_index] && !visited[neighbor_index] {
                            visited[neighbor_index] = true;
                            queue.push_back(neighbor_index);
                        }
                    }
                }
                components.push(component);
            }
        }
        components
    }

    fn unselected_component(&self, start: usize, visited: &mut [bool]) -> (Vec<usize>, bool) {
        let mut component = Vec::new();
        let mut touches_edge = false;
        let mut queue = VecDeque::from([start]);
        visited[start] = true;
        while let Some(index) = queue.pop_front() {
            component.push(index);
            let (x, y) = mask_xy(self.width, index);
            touches_edge |= x == 0 || y == 0 || x + 1 == self.width || y + 1 == self.height;
            for (nx, ny) in mask_neighbors(self.width, self.height, x, y) {
                let neighbor_index = mask_index(self.width, nx, ny);
                if !self.pixels[neighbor_index] && !visited[neighbor_index] {
                    visited[neighbor_index] = true;
                    queue.push_back(neighbor_index);
                }
            }
        }
        (component, touches_edge)
    }
}

pub(crate) fn mask_index(width: u32, x: u32, y: u32) -> usize {
    y as usize * width as usize + x as usize
}

fn mask_xy(width: u32, index: usize) -> (u32, u32) {
    (
        (index % width as usize) as u32,
        (index / width as usize) as u32,
    )
}

fn mask_neighbors(width: u32, height: u32, x: u32, y: u32) -> Vec<(u32, u32)> {
    let mut neighbors = Vec::with_capacity(4);
    if x > 0 {
        neighbors.push((x - 1, y));
    }
    if y > 0 {
        neighbors.push((x, y - 1));
    }
    if x + 1 < width {
        neighbors.push((x + 1, y));
    }
    if y + 1 < height {
        neighbors.push((x, y + 1));
    }
    neighbors
}

pub(crate) fn validate_polygon(
    polygon: &[SceneAssetNormalizedPoint],
) -> Result<(), SceneAssetEditError> {
    if polygon.len() < 3 {
        return Err(SceneAssetEditError::InvalidOperation(
            "restore polygon requires at least three points".to_string(),
        ));
    }
    for point in polygon {
        if !point.x.is_finite()
            || !point.y.is_finite()
            || point.x < 0.0
            || point.y < 0.0
            || point.x > 1.0
            || point.y > 1.0
        {
            return Err(SceneAssetEditError::InvalidOperation(
                "restore polygon points must be finite and inside 0..1".to_string(),
            ));
        }
    }
    Ok(())
}

fn point_in_polygon(
    point: SceneAssetNormalizedPoint,
    polygon: &[SceneAssetNormalizedPoint],
) -> bool {
    let mut inside = false;
    let mut previous = polygon[polygon.len() - 1];
    for &current in polygon {
        let crosses_y = (current.y > point.y) != (previous.y > point.y);
        if crosses_y {
            let slope = (previous.x - current.x) / (previous.y - current.y);
            let intersect_x = slope * (point.y - current.y) + current.x;
            if point.x < intersect_x {
                inside = !inside;
            }
        }
        previous = current;
    }
    inside
}

fn selected_pixel_count(mask: &[bool]) -> usize {
    mask.iter().filter(|&&selected| selected).count()
}
