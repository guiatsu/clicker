use crate::settings::Point;

pub fn append(positions: &mut Vec<Point>, point: Point) -> usize {
    positions.push(point);
    positions.len()
}

pub fn remove(positions: &mut Vec<Point>, index: usize) -> Option<Point> {
    (index < positions.len()).then(|| positions.remove(index))
}

pub fn move_up(positions: &mut [Point], index: usize) -> bool {
    if index == 0 || index >= positions.len() {
        return false;
    }
    positions.swap(index, index - 1);
    true
}

pub fn move_down(positions: &mut [Point], index: usize) -> bool {
    if index + 1 >= positions.len() {
        return false;
    }
    positions.swap(index, index + 1);
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preserves_capture_order_and_supports_reordering_and_removal() {
        let first = Point { x: 10, y: 20 };
        let second = Point { x: -30, y: 40 };
        let third = Point { x: 50, y: -60 };
        let mut positions = Vec::new();
        assert_eq!(append(&mut positions, first), 1);
        assert_eq!(append(&mut positions, second), 2);
        assert_eq!(append(&mut positions, third), 3);
        assert_eq!(positions, [first, second, third]);
        assert!(move_up(&mut positions, 2));
        assert_eq!(positions, [first, third, second]);
        assert_eq!(remove(&mut positions, 1), Some(third));
        assert_eq!(positions, [first, second]);
        assert_eq!(remove(&mut positions, 5), None);
    }
}
