//! Opt-in retained text buffers and revision-checked component adoption.
//! Hosts acquire artifacts; language packages parse/link/complete them. This
//! module performs no I/O and never saves a buffer as a side effect of adding a
//! dependency. Invalid edited text remains available with its diagnostics.
use crate::{Diagnostic, Reference, Source, StoredEvaluation};
use intersect_model::components::{ComponentImage, MountedModel, SharedSchema};
use intersect_model::project_usages::DependencyLock;
use std::sync::Arc;

#[derive(Clone, Debug)]
pub struct PendingReference {
    pub owner: String,
    pub feature: String,
    pub spelling: String,
    pub target_class: String,
    pub uri: String,
    pub start: usize,
    pub end: usize,
    pub index: Option<usize>,
}

pub struct ParsedComponentSource {
    pub evaluation: StoredEvaluation,
    pub pending: Vec<PendingReference>,
    pub consumed: usize,
}

pub struct ComponentAnalysis {
    pub model: Option<MountedModel>,
    pub diagnostics: Vec<Diagnostic>,
    pub references: Vec<Reference>,
}

impl ComponentAnalysis {
    pub fn failed(uri: &str, message: String) -> Self {
        Self {
            model: None,
            diagnostics: vec![Diagnostic::new(uri, 0, 0, message)],
            references: vec![],
        }
    }
}

/// An installed language owns its grammar, scopes, completion and validation.
/// Errors are operational failures; ordinary syntax/link errors are diagnostics.
pub trait ComponentLanguage {
    fn context(&self) -> Arc<SharedSchema>;
    fn analyze(
        &self,
        sources: &[Source],
        usages: &[Arc<ComponentImage>],
    ) -> Result<ComponentAnalysis, String>;
}

pub struct DocumentBuffer {
    saved: Source,
    current: Source,
}
impl DocumentBuffer {
    pub fn uri(&self) -> &str {
        &self.current.uri
    }
    pub fn text(&self) -> &str {
        &self.current.text
    }
    pub fn saved_text(&self) -> &str {
        &self.saved.text
    }
    pub fn is_dirty(&self) -> bool {
        self.current.text != self.saved.text
    }
}

/// Complete candidate state; its private owner/revision guards prevent stale or
/// foreign adoption. A candidate never owns authority to write an editor file.
pub struct PreparedUsageChange {
    owner: Arc<()>,
    revision: u64,
    usages: Vec<Arc<ComponentImage>>,
    analysis: ComponentAnalysis,
    lock: Option<DependencyLock>,
}

pub struct ComponentSession<'a> {
    language: &'a dyn ComponentLanguage,
    owner: Arc<()>,
    revision: u64,
    documents: Vec<DocumentBuffer>,
    usages: Vec<Arc<ComponentImage>>,
    analysis: ComponentAnalysis,
    lock: Option<DependencyLock>,
}

impl<'a> ComponentSession<'a> {
    pub fn open(
        language: &'a dyn ComponentLanguage,
        saved_sources: &[Source],
    ) -> Result<Self, String> {
        let mut seen = std::collections::BTreeSet::new();
        if saved_sources.iter().any(|source| !seen.insert(&source.uri)) {
            return Err("Duplicate project source URI".into());
        }
        let analysis = language.analyze(saved_sources, &[])?;
        Ok(Self {
            language,
            owner: Arc::new(()),
            revision: 0,
            documents: saved_sources
                .iter()
                .map(|saved| DocumentBuffer {
                    saved: saved.clone(),
                    current: saved.clone(),
                })
                .collect(),
            usages: vec![],
            analysis,
            lock: None,
        })
    }

    pub fn revision(&self) -> u64 {
        self.revision
    }
    pub fn documents(&self) -> &[DocumentBuffer] {
        &self.documents
    }
    pub fn usages(&self) -> &[Arc<ComponentImage>] {
        &self.usages
    }
    pub fn analysis(&self) -> &ComponentAnalysis {
        &self.analysis
    }
    pub fn dependency_lock(&self) -> Option<&DependencyLock> {
        self.lock.as_ref()
    }
    pub fn model(&self) -> Option<&MountedModel> {
        self.analysis.model.as_ref()
    }
    pub fn sources(&self) -> Vec<Source> {
        self.documents
            .iter()
            .map(|document| document.current.clone())
            .collect()
    }

    /// Retain unsaved text even if analysis reports invalid or incomplete input.
    pub fn edit(&mut self, uri: &str, text: String) -> Result<(), String> {
        let revision = self
            .revision
            .checked_add(1)
            .ok_or("Project revision exhausted")?;
        let document = self
            .documents
            .iter_mut()
            .find(|document| document.uri() == uri)
            .ok_or("Unknown project document")?;
        document.current.text = text;
        self.revision = revision;
        let sources = self.sources();
        match self.language.analyze(&sources, &self.usages) {
            Ok(analysis) => {
                self.analysis = analysis;
                Ok(())
            }
            Err(error) => {
                self.analysis = ComponentAnalysis::failed(uri, error.clone());
                Err(error)
            }
        }
    }

    /// Host acquisition/preparation happens before this operation, so it cannot
    /// hold a stale snapshot of the editor's buffers while the network runs.
    pub fn prepare_usages(
        &self,
        usages: Vec<Arc<ComponentImage>>,
    ) -> Result<PreparedUsageChange, String> {
        if self.lock.is_some() {
            return Err("A locked session requires an explicit updated dependency lock".into());
        }
        self.prepare_inner(usages, None)
    }

    pub fn prepare_locked_usages(
        &self,
        usages: Vec<Arc<ComponentImage>>,
        lock: DependencyLock,
    ) -> Result<PreparedUsageChange, String> {
        if lock.entries().len() != usages.len() {
            return Err("Dependency lock and mounted artifacts differ".into());
        }
        for image in &usages {
            if lock
                .entries()
                .get(image.uri())
                .is_none_or(|entry| &entry.identity != image.identity())
            {
                return Err(format!(
                    "Dependency lock does not select this exact component: {}",
                    image.uri()
                ));
            }
        }
        self.prepare_inner(usages, Some(lock))
    }

    fn prepare_inner(
        &self,
        usages: Vec<Arc<ComponentImage>>,
        lock: Option<DependencyLock>,
    ) -> Result<PreparedUsageChange, String> {
        let context = self.language.context();
        // This validates schema and duplicate admission even for invalid text.
        let _ = MountedModel::new(context, usages.clone())?;
        let analysis = self.language.analyze(&self.sources(), &usages)?;
        Ok(PreparedUsageChange {
            owner: self.owner.clone(),
            revision: self.revision,
            usages,
            analysis,
            lock,
        })
    }

    /// Atomically swap only model context/diagnostics; text and saved baselines
    /// remain byte-for-byte unchanged. Other candidates become stale.
    pub fn adopt_usages(&mut self, prepared: PreparedUsageChange) -> Result<(), String> {
        if !Arc::ptr_eq(&self.owner, &prepared.owner) {
            return Err("Usage proposal belongs to another project session".into());
        }
        if self.revision != prepared.revision {
            return Err(
                "Project changed while analyzing the usage proposal; prepare it again".into(),
            );
        }
        let revision = self
            .revision
            .checked_add(1)
            .ok_or("Project revision exhausted")?;
        self.usages = prepared.usages;
        self.analysis = prepared.analysis;
        self.lock = prepared.lock;
        self.revision = revision;
        Ok(())
    }

    pub fn add_usage(&mut self, usage: Arc<ComponentImage>) -> Result<(), String> {
        let mut usages = self.usages.clone();
        usages.push(usage);
        let prepared = self.prepare_usages(usages)?;
        self.adopt_usages(prepared)
    }
}
