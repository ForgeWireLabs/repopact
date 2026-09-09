use std::path::Path;

use repopact_repository::Repository;
use repopact_types::{RepositoryIdentity, ValidationReport};
use repopact_validation::Validator;

/// Reusable, non-Tauri RepoPact façade. WI053 exposes read-only validation;
/// mutation planning and application are intentionally reserved for WI054.
pub struct RepoPactCore {
    repository: Repository,
}

impl RepoPactCore {
    pub fn open(root: impl AsRef<Path>) -> Self {
        Self {
            repository: Repository::open(root),
        }
    }

    pub fn identity(&self) -> RepositoryIdentity {
        self.repository.identity()
    }

    pub fn validate(self) -> ValidationReport {
        Validator::new(self.repository).validate()
    }
}

pub fn validate(root: impl AsRef<Path>) -> ValidationReport {
    RepoPactCore::open(root).validate()
}
