use crate::{ModelStoreCapacity, ModelStoreForecast};
use desktoplab_compatibility::{CommercialUseState, ModelArtifactProvenance};
use reqwest::blocking::{Client, Response};
use reqwest::header::{CONTENT_RANGE, RANGE};
use sha2::{Digest, Sha256};
use std::fs::{self, File, OpenOptions};
use std::io::{self, Read};
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FrontierArtifactDownload {
    model_id: String,
    source_url: String,
    checksum_sha256: String,
    expected_size_bytes: u64,
}

impl FrontierArtifactDownload {
    pub fn reviewed(
        model_id: impl Into<String>,
        provenance: &ModelArtifactProvenance,
        commercial_use: CommercialUseState,
        expected_size_bytes: u64,
    ) -> Result<Self, FrontierDownloadError> {
        if commercial_use != CommercialUseState::Allowed {
            return Err(FrontierDownloadError::LicenseNotApproved);
        }
        if expected_size_bytes == 0 {
            return Err(FrontierDownloadError::InvalidExpectedSize);
        }
        Ok(Self {
            model_id: model_id.into(),
            source_url: provenance.source_url().into(),
            checksum_sha256: provenance.checksum_sha256().to_ascii_lowercase(),
            expected_size_bytes,
        })
    }

    #[must_use]
    pub fn model_id(&self) -> &str {
        &self.model_id
    }
}

pub struct ArtifactResponse {
    reader: Box<dyn Read + Send>,
    resumed: bool,
}

impl ArtifactResponse {
    #[must_use]
    pub fn new(reader: Box<dyn Read + Send>, resumed: bool) -> Self {
        Self { reader, resumed }
    }
}

pub trait ResumableArtifactSource {
    fn fetch(&self, url: &str, offset: u64) -> Result<ArtifactResponse, FrontierDownloadError>;
}

#[derive(Clone, Debug)]
pub struct HttpsRangeArtifactSource {
    client: Client,
}

impl HttpsRangeArtifactSource {
    pub fn new() -> Result<Self, FrontierDownloadError> {
        let client = Client::builder()
            .build()
            .map_err(|error| FrontierDownloadError::Source(error.to_string()))?;
        Ok(Self { client })
    }
}

impl ResumableArtifactSource for HttpsRangeArtifactSource {
    fn fetch(&self, url: &str, offset: u64) -> Result<ArtifactResponse, FrontierDownloadError> {
        if !url.starts_with("https://") {
            return Err(FrontierDownloadError::InsecureSource);
        }
        let mut request = self.client.get(url);
        if offset > 0 {
            request = request.header(RANGE, format!("bytes={offset}-"));
        }
        let response = request
            .send()
            .map_err(|error| FrontierDownloadError::Source(error.to_string()))?;
        response_to_artifact(response, offset)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FrontierModelStore {
    root: PathBuf,
    capacity: ModelStoreCapacity,
}

impl FrontierModelStore {
    pub fn new(root: impl Into<PathBuf>, capacity: ModelStoreCapacity) -> io::Result<Self> {
        let root = root.into();
        fs::create_dir_all(&root)?;
        Ok(Self { root, capacity })
    }

    #[must_use]
    pub fn forecast(&self, request: &FrontierArtifactDownload) -> ModelStoreForecast {
        self.capacity.forecast(
            request.expected_size_bytes,
            partial_size(&self.partial_path(request)),
        )
    }

    pub fn download(
        &self,
        request: &FrontierArtifactDownload,
        source: &dyn ResumableArtifactSource,
    ) -> Result<FrontierDownloadOutcome, FrontierDownloadError> {
        let partial_path = self.partial_path(request);
        let target_path = self.target_path(request);
        if target_path.exists() {
            if checksum(&target_path)? == request.checksum_sha256 {
                return Ok(FrontierDownloadOutcome::already_present(target_path));
            }
            let invalid_path = self.invalid_path(request);
            if invalid_path.exists() {
                fs::remove_file(&invalid_path)?;
            }
            fs::rename(&target_path, invalid_path)?;
        }

        let forecast = self.forecast(request);
        if !forecast.fits() {
            return Err(FrontierDownloadError::InsufficientCapacity(forecast));
        }
        let mut offset = partial_size(&partial_path);
        if offset > request.expected_size_bytes {
            fs::remove_file(&partial_path)?;
            offset = 0;
        }

        let mut response = source.fetch(&request.source_url, offset)?;
        if offset > 0 && !response.resumed {
            fs::remove_file(&partial_path)?;
            offset = 0;
        }
        let mut output = OpenOptions::new()
            .create(true)
            .write(true)
            .append(offset > 0)
            .truncate(offset == 0)
            .open(&partial_path)?;
        let written = io::copy(&mut response.reader, &mut output)?;
        output.sync_all()?;
        let completed_size = partial_size(&partial_path);
        if completed_size != request.expected_size_bytes {
            return Err(FrontierDownloadError::Incomplete {
                expected_bytes: request.expected_size_bytes,
                actual_bytes: completed_size,
            });
        }
        if checksum(&partial_path)? != request.checksum_sha256 {
            let invalid_path = self.invalid_path(request);
            fs::rename(&partial_path, &invalid_path)?;
            return Err(FrontierDownloadError::ChecksumMismatch(invalid_path));
        }
        fs::rename(&partial_path, &target_path)?;
        Ok(FrontierDownloadOutcome {
            path: target_path,
            resumed_from_bytes: offset,
            written_bytes: written,
            already_present: false,
        })
    }

    fn target_path(&self, request: &FrontierArtifactDownload) -> PathBuf {
        self.root
            .join(safe_file_name(&request.model_id))
            .with_extension("weights")
    }

    fn partial_path(&self, request: &FrontierArtifactDownload) -> PathBuf {
        self.root
            .join(safe_file_name(&request.model_id))
            .with_extension("partial")
    }

    fn invalid_path(&self, request: &FrontierArtifactDownload) -> PathBuf {
        self.root
            .join(safe_file_name(&request.model_id))
            .with_extension("invalid")
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FrontierDownloadOutcome {
    path: PathBuf,
    resumed_from_bytes: u64,
    written_bytes: u64,
    already_present: bool,
}

impl FrontierDownloadOutcome {
    fn already_present(path: PathBuf) -> Self {
        Self {
            path,
            resumed_from_bytes: 0,
            written_bytes: 0,
            already_present: true,
        }
    }

    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    #[must_use]
    pub fn resumed_from_bytes(&self) -> u64 {
        self.resumed_from_bytes
    }

    #[must_use]
    pub fn written_bytes(&self) -> u64 {
        self.written_bytes
    }

    #[must_use]
    pub fn is_already_present(&self) -> bool {
        self.already_present
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FrontierDownloadError {
    LicenseNotApproved,
    InvalidExpectedSize,
    InsecureSource,
    Source(String),
    InsufficientCapacity(ModelStoreForecast),
    Incomplete {
        expected_bytes: u64,
        actual_bytes: u64,
    },
    ChecksumMismatch(PathBuf),
    Io(String),
}

impl From<io::Error> for FrontierDownloadError {
    fn from(error: io::Error) -> Self {
        Self::Io(error.to_string())
    }
}

fn response_to_artifact(
    response: Response,
    offset: u64,
) -> Result<ArtifactResponse, FrontierDownloadError> {
    if !response.status().is_success() {
        return Err(FrontierDownloadError::Source(format!(
            "HTTP {}",
            response.status()
        )));
    }
    let resumed = offset > 0 && response.status().as_u16() == 206;
    if resumed && response.headers().get(CONTENT_RANGE).is_none() {
        return Err(FrontierDownloadError::Source(
            "partial response has no content-range".into(),
        ));
    }
    Ok(ArtifactResponse::new(Box::new(response), resumed))
}

fn checksum(path: &Path) -> Result<String, FrontierDownloadError> {
    let mut file = File::open(path)?;
    let mut digest = Sha256::new();
    let mut buffer = [0_u8; 1024 * 1024];
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        digest.update(&buffer[..read]);
    }
    Ok(format!("{:x}", digest.finalize()))
}

fn partial_size(path: &Path) -> u64 {
    path.metadata().map_or(0, |metadata| metadata.len())
}

fn safe_file_name(model_id: &str) -> String {
    model_id
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || matches!(character, '-' | '_') {
                character
            } else {
                '_'
            }
        })
        .collect()
}
