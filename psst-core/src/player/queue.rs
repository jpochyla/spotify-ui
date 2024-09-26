use rand::prelude::SliceRandom;

use super::PlaybackItem;

#[derive(Debug)]
pub enum QueueBehavior {
    Sequential,
    Random,
    LoopTrack,
    LoopAll,
}

impl Default for QueueBehavior {
    fn default() -> Self {
        Self::Sequential
    }
}

pub struct Queue {
    items: Vec<PlaybackItem>,
    added_items_in_main_queue: Vec<(usize, usize)>,
    user_items: Vec<PlaybackItem>,
    position: usize,
    user_items_position: usize,
    positions: Vec<usize>,
    behavior: QueueBehavior,
    playing_from_user_items: bool,
}

impl Queue {
    pub fn new() -> Self {
        Self {
            items: Vec::new(),
            added_items_in_main_queue: Vec::new(),
            user_items: Vec::new(),
            position: 0,
            user_items_position: 0,
            positions: Vec::new(),
            behavior: QueueBehavior::default(),
            playing_from_user_items: false,
        }
    }
    
    pub fn clear(&mut self) {
        self.items.clear();
        self.positions.clear();
        self.position = 0;
    }

    pub fn clear_user_items(&mut self) {
        self.user_items.clear();
        self.user_items_position = 0;
    }

    pub fn fill(&mut self, items: Vec<PlaybackItem>, position: usize) {
        self.positions.clear();
        self.added_items_in_main_queue.clear();
        self.items = items;
        self.position = position;
        self.compute_positions();
    }
    
    pub fn skip_to_place_in_queue(&mut self, index: usize) {
        if self.playing_from_user_items {
            self.user_items = self.user_items.split_off(index + 1);
        }
        else {
            self.user_items = self.user_items.split_off(index);
        }
        self.user_items_position = 0;
    }

    pub fn add(&mut self, item: PlaybackItem) {
        self.user_items.push(item);
    }

    pub fn get_playing_from_user_items_bool(&mut self) -> bool{
        self.playing_from_user_items
    }

    pub fn remove(&mut self, index: usize) {
        if self.playing_from_user_items {
            self.user_items.remove(index+1);
        }
        else {
            self.user_items.remove(index);
        }
        if self.user_items_position < index && self.user_items_position > 0 {
            self.user_items_position -= 1;
        }
    }

    fn handle_added_queue(&mut self) {
        if !self.added_items_in_main_queue.is_empty() {
            let item_index = self.added_items_in_main_queue[0].0;
            let position_index = self.added_items_in_main_queue[0].1;
            self.items.remove(item_index - 1);
            self.positions.remove(position_index);

            self.added_items_in_main_queue.remove(0);
            if self.position > 0 {
                self.position -= 1;
            }
        }

        if self.user_items.len() > self.user_items_position {
            self.items.insert(
                self.positions.len(),
                self.user_items[self.user_items_position],
            );

            self.positions
                .insert(self.position + 1, self.positions.len());
            self.user_items_position += 1;

            self.added_items_in_main_queue.push((self.positions.len(), self.position + 1));
            self.playing_from_user_items = true;
        }
        else {
            self.playing_from_user_items = false;
        }
    }

    pub fn set_behaviour(&mut self, behavior: QueueBehavior) {
        self.behavior = behavior;
        self.compute_positions();
    }

    fn compute_positions(&mut self) {
        // In the case of switching away from shuffle, the position should be set back to
        // where it appears in the actual playlist order.
        let playlist_position = if self.positions.len() > 1 {
            self.positions[self.position]
        } else {
            self.position
        };
        // Start with an ordered 1:1 mapping.
        self.positions = (0..self.items.len()).collect();

        if let QueueBehavior::Random = self.behavior {
            // Swap the current position with the first item, so we will start from the
            // beginning, with the full queue ahead of us.  Then shuffle the rest of the
            // items and set the position to 0.
            if self.positions.len() > 1 {
                self.positions.swap(0, self.position);
                self.positions[1..].shuffle(&mut rand::thread_rng());
            }
            self.position = 0;
        } else {
            self.position = playlist_position;
        }
    }

    pub fn skip_to_previous(&mut self) {
        self.position = self.previous_position();
    }

    pub fn skip_to_next(&mut self) {
        self.handle_added_queue();
        self.position = self.next_position();
    }

    pub fn skip_to_following(&mut self) {
        self.handle_added_queue();
        self.position = self.following_position();
    }

    pub fn get_current(&self) -> Option<&PlaybackItem> {
        let position = self.positions.get(self.position).copied()?;
        self.items.get(position)
    }

    pub fn get_following(&self) -> Option<&PlaybackItem> {
        if let Some(position) = self.positions.get(self.position).copied() {
            if let Some(item) = self.items.get(position) {
                return Some(item);
            }
        } else {
            return self.user_items.first();
        }
        None
    }

    fn previous_position(&self) -> usize {
        match self.behavior {
            QueueBehavior::Sequential
            | QueueBehavior::Random
            | QueueBehavior::LoopTrack
            | QueueBehavior::LoopAll => self.position.saturating_sub(1),
        }
    }

    fn next_position(&self) -> usize {
        match self.behavior {
            QueueBehavior::Sequential | QueueBehavior::Random | QueueBehavior::LoopTrack => {
                self.position + 1
            }
            QueueBehavior::LoopAll => (self.position + 1) % self.items.len(),
        }
    }

    fn following_position(&self) -> usize {
        match self.behavior {
            QueueBehavior::Sequential | QueueBehavior::Random => self.position + 1,
            QueueBehavior::LoopTrack => self.position,
            QueueBehavior::LoopAll => (self.position + 1) % self.items.len(),
        }
    }
}
