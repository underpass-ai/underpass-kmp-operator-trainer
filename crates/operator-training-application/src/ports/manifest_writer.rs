//! Port: serialise a `TrainingManifest` to whatever medium the adapter
//! targets. TOML is the first concrete format; other formats land as
//! additional adapters behind this port.

use operator_training_domain::manifest::training_manifest::TrainingManifest;

use crate::errors::manifest_write_error::ManifestWriteError;

pub trait ManifestWriter: std::fmt::Debug + Send + Sync {
    fn write(&self, manifest: &TrainingManifest) -> Result<(), ManifestWriteError>;
}
