use desktoplab_storage::{SettingRecord, SettingValue};

use super::LocalApiRouter;

impl LocalApiRouter {
    pub(crate) fn persist_selected_route_id(&mut self) {
        let Some(storage) = &self.storage else {
            return;
        };
        let result = storage.put_setting(SettingRecord::new(
            "routing.selected_route_id",
            SettingValue::String(self.selected_route_id.clone()),
        ));
        self.record_state_journal_result(result);
    }
}
