use std::cmp::Ord;

pub struct MaxHeap<T> {
    data: Vec<T>,
}

impl<T: Ord> MaxHeap<T> {
    pub fn new() -> Self {
        Self {
            data: Vec::new(),
        }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            data: Vec::with_capacity(capacity),
        }
    }

    pub fn insert(&mut self, item: T) {
        self.data.push(item);
        self.heapify_up(self.data.len() - 1);
    }

    pub fn peek(&self) -> Option<&T> {
        self.data.first()
    }

    pub fn pop(&mut self) -> Option<T> {
        if self.data.is_empty() {
            return None;
        }
        
        if self.data.len() == 1 {
            return self.data.pop();
        }
        
        let max = self.data.swap_remove(0);
        self.heapify_down(0);
        Some(max)
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }

    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    pub fn clear(&mut self) {
        self.data.clear();
    }

    fn heapify_up(&mut self, mut index: usize) {
        while index > 0 {
            let parent_index = (index - 1) / 2;
            if self.data[index] <= self.data[parent_index] {
                break;
            }
            self.data.swap(index, parent_index);
            index = parent_index;
        }
    }

    fn heapify_down(&mut self, mut index: usize) {
        loop {
            let left_child = 2 * index + 1;
            let right_child = 2 * index + 2;
            let mut largest = index;

            if left_child < self.data.len() && self.data[left_child] > self.data[largest] {
                largest = left_child;
            }

            if right_child < self.data.len() && self.data[right_child] > self.data[largest] {
                largest = right_child;
            }

            if largest == index {
                break;
            }

            self.data.swap(index, largest);
            index = largest;
        }
    }
}

impl<T: Ord> Default for MaxHeap<T> {
    fn default() -> Self {
        Self::new()
    }
}