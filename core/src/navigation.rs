#[derive(Debug)]
pub struct NavigationEngine {
    pub current_index: usize,
    pub view_count: usize,
}

impl NavigationEngine {
    pub fn new(view_count: usize) -> Self {
        assert!(view_count > 0);
        Self {
            current_index: 0,
            view_count,
        }
    }

    pub fn next(&mut self, total_items: usize) {
        if total_items == 0 {
            return;
        }

        let next = self.current_index + self.view_count;
        if next < total_items {
            self.current_index = next;
        }
    }

    pub fn prev(&mut self) {
        if self.current_index >= self.view_count {
            self.current_index -= self.view_count;
        } else {
            self.current_index = 0;
        }
    }

    pub fn range(&self, total_items: usize) -> (usize, usize) {
        let start = self.current_index;
        let end = (start + self.view_count).min(total_items);
        (start, end)
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn starts_at_zero() {
        let nav = NavigationEngine::new(4);
        assert_eq!(nav.current_index, 0);
    }

    #[test]
    fn next_moves_by_view_count() {
        let mut nav = NavigationEngine::new(2);
        nav.next(10);
        assert_eq!(nav.current_index, 2);
    }

    #[test]
    fn prev_does_not_go_negative() {
        let mut nav = NavigationEngine::new(4);
        nav.prev();
        assert_eq!(nav.current_index, 0);
    }

    #[test]
    fn next_does_not_overflow() {
        let mut nav = NavigationEngine::new(4);
        nav.current_index = 8;
        nav.next(10);
        assert_eq!(nav.current_index, 8);
    }

    #[test]
    fn range_is_correct() {
        let nav = NavigationEngine::new(4);
        let (start, end) = nav.range(10);
        assert_eq!(start, 0);
        assert_eq!(end, 4);
    }

    #[test]
    fn range_clamps_at_end() {
        let nav = NavigationEngine {
            current_index: 8,
            view_count: 4,
        };
        let (start, end) = nav.range(10);
        assert_eq!(start, 8);
        assert_eq!(end, 10);
    }
}
