#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MemoryId(usize);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MemoryVisibility {
    ProviderShareable,
    LocalOnly,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MemoryRecord {
    workspace_id: String,
    text: String,
    provenance: String,
    visibility: MemoryVisibility,
}

impl MemoryRecord {
    #[must_use]
    pub fn provenance(&self) -> &str {
        &self.provenance
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct WorkspaceMemoryStore {
    records: Vec<Option<MemoryRecord>>,
}

impl WorkspaceMemoryStore {
    pub fn remember(
        &mut self,
        workspace_id: &str,
        text: &str,
        provenance: &str,
        visibility: MemoryVisibility,
    ) -> MemoryId {
        let id = MemoryId(self.records.len());
        self.records.push(Some(MemoryRecord {
            workspace_id: workspace_id.to_string(),
            text: text.to_string(),
            provenance: provenance.to_string(),
            visibility,
        }));
        id
    }

    #[must_use]
    pub fn get(&self, id: MemoryId) -> Option<&MemoryRecord> {
        self.records.get(id.0).and_then(Option::as_ref)
    }

    pub fn delete(&mut self, id: MemoryId) -> bool {
        self.records
            .get_mut(id.0)
            .is_some_and(|record| record.take().is_some())
    }

    #[must_use]
    pub fn provider_context(&self, workspace_id: &str) -> Vec<&str> {
        self.records
            .iter()
            .filter_map(Option::as_ref)
            .filter(|record| {
                record.workspace_id == workspace_id
                    && record.visibility == MemoryVisibility::ProviderShareable
            })
            .map(|record| record.text.as_str())
            .collect()
    }

    #[must_use]
    pub fn export(&self, workspace_id: &str) -> String {
        self.records
            .iter()
            .filter_map(Option::as_ref)
            .filter(|record| record.workspace_id == workspace_id)
            .map(|record| record.text.as_str())
            .collect::<Vec<_>>()
            .join("\n")
    }
}
