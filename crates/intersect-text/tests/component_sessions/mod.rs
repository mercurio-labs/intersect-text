//! Session state controls. Real parser/linker integration lives in the language
//! harness; this deliberately minimal analyzer isolates buffer/adoption guards.
use intersect_model::{
    components::{ComponentImage, MountedModel, SharedSchema},
    frozen::{ById, ModelBuilder},
};
use intersect_text::{
    Source,
    components::{ComponentAnalysis, ComponentLanguage, ComponentSession},
};
use std::sync::Arc;

struct BufferLanguage {
    context: Arc<SharedSchema>,
}
impl BufferLanguage {
    fn new() -> Result<Self, String> {
        let package = intersect_model::compile_rcore("rcore 1; package buffers version \"1\"; metamodel \"urn:buffers\" prefix b { class Item {} }").map_err(|e| e.to_string())?;
        Ok(Self {
            context: SharedSchema::new(package.schema())?,
        })
    }
    fn image(&self) -> Result<Arc<ComponentImage>, String> {
        let model = ModelBuilder::new(self.context.metamodel())
            .link(&ById)
            .map_err(|e| format!("{e:?}"))?
            .validate()
            .map_err(|e| format!("{e:?}"))?;
        ComponentImage::prepare(self.context.clone(), "urn:buffer-test:library", model)
    }
}
impl ComponentLanguage for BufferLanguage {
    fn context(&self) -> Arc<SharedSchema> {
        self.context.clone()
    }
    fn analyze(
        &self,
        sources: &[Source],
        usages: &[Arc<ComponentImage>],
    ) -> Result<ComponentAnalysis, String> {
        if sources
            .iter()
            .any(|source| source.text == "operational failure")
        {
            return Err("Analyzer failed".into());
        }
        if sources.iter().any(|source| source.text == "incomplete") {
            return Ok(ComponentAnalysis::failed(
                "memory:/project",
                "Incomplete text".into(),
            ));
        }
        Ok(ComponentAnalysis {
            model: Some(MountedModel::new(self.context.clone(), usages.to_vec())?),
            diagnostics: vec![],
            references: vec![],
        })
    }
}
fn saved() -> Vec<Source> {
    vec![Source {
        uri: "memory:/project".into(),
        text: "saved\r\n".into(),
    }]
}

#[test]
fn component_sessions_keep_unsaved_bytes_and_saved_baseline_on_adoption() -> Result<(), String> {
    let language = BufferLanguage::new()?;
    let mut project = ComponentSession::open(&language, &saved())?;
    let unsaved = "// α unsaved\r\nnew text\r\n";
    project.edit("memory:/project", unsaved.into())?;
    let proposal = project.prepare_usages(vec![language.image()?])?;
    assert_eq!(project.usages().len(), 0);
    project.adopt_usages(proposal)?;
    assert_eq!(project.usages().len(), 1);
    assert_eq!(project.documents()[0].text(), unsaved);
    assert_eq!(project.documents()[0].saved_text(), "saved\r\n");
    assert!(project.documents()[0].is_dirty());
    Ok(())
}

#[test]
fn component_sessions_reject_stale_and_foreign_proposals_without_losing_edits() -> Result<(), String>
{
    let language = BufferLanguage::new()?;
    let mut project = ComponentSession::open(&language, &saved())?;
    let proposal = project.prepare_usages(vec![language.image()?])?;
    project.edit("memory:/project", "newer unsaved text".into())?;
    assert!(project.adopt_usages(proposal).is_err());
    assert_eq!(project.documents()[0].text(), "newer unsaved text");
    assert!(project.usages().is_empty());
    let foreign = project.prepare_usages(vec![language.image()?])?;
    let mut other = ComponentSession::open(&language, &saved())?;
    assert!(other.adopt_usages(foreign).is_err());
    assert!(other.usages().is_empty());
    assert_eq!(other.documents()[0].text(), "saved\r\n");
    Ok(())
}

#[test]
fn component_sessions_keep_invalid_or_failed_edits_and_invalidate_other_candidates()
-> Result<(), String> {
    let language = BufferLanguage::new()?;
    let mut project = ComponentSession::open(&language, &saved())?;
    project.edit("memory:/project", "incomplete".into())?;
    let first = project.prepare_usages(vec![language.image()?])?;
    let second = project.prepare_usages(vec![])?;
    project.adopt_usages(first)?;
    assert!(project.adopt_usages(second).is_err());
    assert!(project.model().is_none());
    assert!(!project.analysis().diagnostics.is_empty());
    assert_eq!(project.documents()[0].text(), "incomplete");
    assert!(
        project
            .edit("memory:/project", "operational failure".into())
            .is_err()
    );
    assert_eq!(project.documents()[0].text(), "operational failure");
    assert!(project.documents()[0].is_dirty());
    assert!(project.model().is_none());
    assert_eq!(project.usages().len(), 1);
    Ok(())
}

#[test]
fn component_sessions_failed_admission_and_unknown_document_leave_state_intact()
-> Result<(), String> {
    let language = BufferLanguage::new()?;
    let mut project = ComponentSession::open(&language, &saved())?;
    project.edit("memory:/project", "unsaved".into())?;
    let revision = project.revision();
    let image = language.image()?;
    assert!(project.prepare_usages(vec![image.clone(), image]).is_err());
    assert!(project.edit("missing", "lost".into()).is_err());
    assert_eq!(project.revision(), revision);
    assert_eq!(project.documents()[0].text(), "unsaved");
    assert!(project.usages().is_empty());
    let duplicate = vec![saved().remove(0), saved().remove(0)];
    assert!(ComponentSession::open(&language, &duplicate).is_err());
    Ok(())
}

#[test]
fn component_sessions_adopt_exact_lock_atomically_and_refuse_unlocked_replacement()
-> Result<(), String> {
    use intersect_model::{
        definitions::bundle::SourceId,
        project_usages::{DependencyLock, PackageCatalog, PackageManifest, Usage, UsageResolver},
    };
    struct Catalog(PackageManifest);
    impl PackageCatalog for Catalog {
        fn candidates(&self, uri: &str, _: Option<&str>) -> Vec<PackageManifest> {
            if uri == self.0.uri {
                vec![self.0.clone()]
            } else {
                vec![]
            }
        }
    }
    let language = BufferLanguage::new()?;
    let image = language.image()?;
    let catalog = Catalog(PackageManifest {
        uri: image.uri().into(),
        version: "1.0.0".into(),
        provenance: SourceId {
            uri: "https://example.test/library.kpar".into(),
            sha256: "test".into(),
        },
        identity: image.identity().clone(),
        usages: vec![],
        dependency_lock: DependencyLock::empty(),
    });
    let lock = UsageResolver::resolve(
        &[Usage {
            resource_uri: image.uri().into(),
            version_constraint: None,
        }],
        &catalog,
    )
    .map_err(|e| e.to_string())?;
    let mut project = ComponentSession::open(&language, &saved())?;
    project.edit("memory:/project", "unsaved".into())?;
    assert!(
        project
            .prepare_locked_usages(vec![image.clone()], DependencyLock::empty())
            .is_err()
    );
    assert!(project.dependency_lock().is_none());
    let proposal = project.prepare_locked_usages(vec![image.clone()], lock.clone())?;
    project.adopt_usages(proposal)?;
    assert_eq!(project.dependency_lock(), Some(&lock));
    assert!(project.prepare_usages(vec![]).is_err());
    assert_eq!(project.usages().len(), 1);
    assert_eq!(project.documents()[0].text(), "unsaved");
    let stale = project.prepare_locked_usages(vec![], DependencyLock::empty())?;
    project.edit("memory:/project", "more edits".into())?;
    assert!(project.adopt_usages(stale).is_err());
    assert_eq!(project.dependency_lock(), Some(&lock));
    assert_eq!(project.documents()[0].text(), "more edits");
    Ok(())
}
