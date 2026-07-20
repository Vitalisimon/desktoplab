use desktoplab_compatibility::{CommercialUseState, ModelArtifactProvenance};
use desktoplab_model_manager::{
    ArtifactResponse, FrontierArtifactDownload, FrontierDownloadError, FrontierModelStore,
    HttpsRangeArtifactSource, ModelFootprintTier, ModelStoreCapacity, ModelStoreEntry,
    ModelStoreInventory, ResumableArtifactSource,
};
use sha2::{Digest, Sha256};
use std::io::Cursor;
use std::sync::Mutex;
use tempfile::tempdir;
use xtask::check_logical_line_limit;

#[test]
fn partial_download_resumes_verifies_checksum_and_promotes_atomically() {
    let directory = tempdir().unwrap();
    let bytes = b"frontier-model-weights";
    let partial = directory.path().join("model_large.partial");
    std::fs::write(&partial, &bytes[..9]).unwrap();
    let request = request("model.large", bytes);
    let source = FixtureSource::new(bytes.to_vec());
    let store =
        FrontierModelStore::new(directory.path(), ModelStoreCapacity::new(1_000_000, 10_000))
            .unwrap();

    let outcome = store.download(&request, &source).unwrap();

    assert_eq!(source.offsets(), vec![9]);
    assert_eq!(outcome.resumed_from_bytes(), 9);
    assert_eq!(outcome.written_bytes(), (bytes.len() - 9) as u64);
    assert_eq!(std::fs::read(outcome.path()).unwrap(), bytes);
    assert!(!partial.exists());
}

#[test]
fn checksum_mismatch_is_quarantined_and_never_becomes_a_model() {
    let directory = tempdir().unwrap();
    let bytes = b"wrong-weights";
    let provenance = ModelArtifactProvenance::verified(
        "https://models.example.invalid/weights.bin",
        "a".repeat(64),
    )
    .unwrap();
    let request = FrontierArtifactDownload::reviewed(
        "model.large",
        &provenance,
        CommercialUseState::Allowed,
        bytes.len() as u64,
    )
    .unwrap();
    let store =
        FrontierModelStore::new(directory.path(), ModelStoreCapacity::new(1_000_000, 0)).unwrap();

    let error = store
        .download(&request, &FixtureSource::new(bytes.to_vec()))
        .unwrap_err();
    let FrontierDownloadError::ChecksumMismatch(invalid_path) = error else {
        panic!("expected checksum mismatch");
    };
    assert!(invalid_path.exists());
    assert!(!directory.path().join("model_large.weights").exists());
}

#[test]
fn storage_forecast_covers_hundred_gb_to_multi_tb_and_preserves_reserve() {
    let capacity = ModelStoreCapacity::new(8_000_000_000_000, 500_000_000_000);
    for (size, tier) in [
        (100_000_000_000, ModelFootprintTier::Gb100),
        (500_000_000_000, ModelFootprintTier::Gb500),
        (1_000_000_000_000, ModelFootprintTier::Tb1),
        (4_000_000_000_000, ModelFootprintTier::MultiTb),
    ] {
        let forecast = capacity.forecast(size, 0);
        assert_eq!(forecast.tier(), tier);
        assert!(forecast.fits());
        assert_eq!(forecast.reserve_bytes(), 500_000_000_000);
    }
    assert!(!capacity.forecast(7_600_000_000_000, 0).fits());
}

#[test]
fn eviction_recommendation_is_lru_and_never_selects_pinned_models() {
    let inventory = ModelStoreInventory::new(vec![
        ModelStoreEntry::new("model.pinned", "/models/pinned", 500, 1).pinned(),
        ModelStoreEntry::new("model.old", "/models/old", 300, 2),
        ModelStoreEntry::new("model.new", "/models/new", 400, 3),
    ]);

    let recommendation = inventory.eviction_recommendation(600);
    assert_eq!(recommendation.model_ids(), &["model.old", "model.new"]);
    assert_eq!(recommendation.reclaimed_bytes(), 700);
    assert!(recommendation.is_sufficient());
    assert!(
        !recommendation
            .model_ids()
            .contains(&"model.pinned".to_string())
    );
}

#[test]
fn unknown_or_restricted_license_cannot_create_download_request() {
    let provenance = ModelArtifactProvenance::verified(
        "https://models.example.invalid/weights.bin",
        "b".repeat(64),
    )
    .unwrap();
    let error = FrontierArtifactDownload::reviewed(
        "model.large",
        &provenance,
        CommercialUseState::Unknown,
        100,
    )
    .unwrap_err();
    assert_eq!(error, FrontierDownloadError::LicenseNotApproved);
}

#[test]
fn production_range_source_rejects_insecure_urls_before_network_access() {
    let source = HttpsRangeArtifactSource::new().unwrap();
    let Err(error) = source.fetch("http://127.0.0.1:9/weights", 0) else {
        panic!("insecure source must be rejected");
    };

    assert_eq!(error, FrontierDownloadError::InsecureSource);
}

#[test]
fn huge_model_store_sources_stay_below_line_guards() {
    for (path, source, limit) in [
        (
            "crates/desktoplab-model-manager/src/frontier_download.rs",
            include_str!("../src/frontier_download.rs"),
            360,
        ),
        (
            "crates/desktoplab-model-manager/src/frontier_store.rs",
            include_str!("../src/frontier_store.rs"),
            260,
        ),
    ] {
        check_logical_line_limit(path, source, limit)
            .expect("model store source should stay focused");
    }
}

fn request(model_id: &str, bytes: &[u8]) -> FrontierArtifactDownload {
    let provenance = ModelArtifactProvenance::verified(
        "https://models.example.invalid/weights.bin",
        format!("{:x}", Sha256::digest(bytes)),
    )
    .unwrap();
    FrontierArtifactDownload::reviewed(
        model_id,
        &provenance,
        CommercialUseState::Allowed,
        bytes.len() as u64,
    )
    .unwrap()
}

struct FixtureSource {
    bytes: Vec<u8>,
    offsets: Mutex<Vec<u64>>,
}

impl FixtureSource {
    fn new(bytes: Vec<u8>) -> Self {
        Self {
            bytes,
            offsets: Mutex::new(Vec::new()),
        }
    }

    fn offsets(&self) -> Vec<u64> {
        self.offsets.lock().unwrap().clone()
    }
}

impl ResumableArtifactSource for FixtureSource {
    fn fetch(&self, _url: &str, offset: u64) -> Result<ArtifactResponse, FrontierDownloadError> {
        self.offsets.lock().unwrap().push(offset);
        Ok(ArtifactResponse::new(
            Box::new(Cursor::new(self.bytes[offset as usize..].to_vec())),
            offset > 0,
        ))
    }
}
