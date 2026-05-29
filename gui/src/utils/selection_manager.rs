use crate::constants::MAX_HISTORY_SIZE;

#[derive(Default)]
pub struct SelectionManager<T: Clone + Copy + PartialEq> {
    // The current active selection (replaces your old single variable)
    pub current: Option<T>,
    // Stacks for navigation
    history: Vec<Option<T>>,
    future: Vec<Option<T>>,
}

impl<T: Clone + Copy + PartialEq> SelectionManager<T> {
    pub fn new() -> Self {
        Self {
            current: None,
            history: Vec::new(),
            future: Vec::new(),
        }
    }
    /// Selects a specific entity. Used for UI lists, labels, or sidebar items
    /// where the target entity is guaranteed to exist.
    pub fn select(&mut self, entity: T) {
        let target = Some(entity);

        // If it's already selected, do nothing
        if self.current == target {
            return;
        }

        self.transition_to(target);
    }

    /// Updates selection based on a 2D spatial query.
    /// Missing an entity (None) does nothing. Hitting the same entity toggles it off.
    pub fn select_from_spatial_query(&mut self, clicked_entity: Option<T>) {
        match clicked_entity {
            // Case 1: You clicked empty space -> Ignore it completely
            None => {}

            // Case 2: You hit an entity
            Some(entity) => {
                if self.current == Some(entity) {
                    // It's the same entity -> Toggle it off (Deselect)
                    self.transition_to(None);
                } else {
                    // It's a brand new entity -> Select it
                    self.transition_to(Some(entity));
                }
            }
        }
    }

    /// Core engine helper that drives the history state machine forward
    fn transition_to(&mut self, next_state: Option<T>) {
        // 1. Log the current state into the past
        self.history.push(self.current);

        // 2. Keep the history ring bounded
        if self.history.len() > MAX_HISTORY_SIZE {
            self.history.remove(0);
        }

        // 3. Move to the new state
        self.current = next_state;

        // 4. Any new branch mutation invalidates the redo/future stack
        self.future.clear();
    }

    /// Move back to the previous selection (e.g., when pressing Back / Ctrl+Z)
    pub fn go_back(&mut self) {
        if let Some(prev_selection) = self.history.pop() {
            // Push current state to the future stack so we can go forward again
            self.future.push(self.current);
            // Update current
            self.current = prev_selection;
        }
    }

    /// Move forward (e.g., when pressing Forward / Ctrl+Y)
    pub fn go_forward(&mut self) {
        if let Some(next_selection) = self.future.pop() {
            // Push current state back to history
            self.history.push(self.current);
            // Update current
            self.current = next_selection;
        }
    }
}
