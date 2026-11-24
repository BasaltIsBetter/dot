use std::sync::{Mutex, MutexGuard, PoisonError};

use crate::launch::{group_is_alive, group_led_by};

#[derive(Debug, Default)]
pub(super) struct Groups(Mutex<Vec<u32>>);

impl Groups {
    pub(super) fn remember(&self, process: u32) {
        let Some(group) = group_led_by(process) else {
            return;
        };
        let mut groups = self.locked();
        if !groups.contains(&group) {
            groups.push(group);
        }
    }

    pub(super) fn forget_ended(&self) {
        self.locked().retain(|&group| group_is_alive(group));
    }

    pub(super) fn take_all(&self) -> Vec<u32> {
        std::mem::take(&mut *self.locked())
    }

    pub(super) fn locked(&self) -> MutexGuard<'_, Vec<u32>> {
        self.0.lock().unwrap_or_else(PoisonError::into_inner)
    }
}
