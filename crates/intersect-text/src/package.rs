//! Handwritten text package ownership and composition over borrowed model metadata.
//! Retained package DTOs and views grant no compiler or runtime admission.
use intersect_model::definitions::schema::{Schema, Semantics};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
/// Original source inputs for a language-owned Xtext inference compiler.
/// This DTO is retained data only; definitions/core never run inference.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct XtextInferenceInputs {
    /// Explicit native source capability selection. Omitted recipes retain the
    /// historical inference profile. This is admission policy, never cached IR.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub profile: Option<crate::grammar::Profile>,

    pub version: u32,
    pub bundle: intersect_model::definitions::bundle::DefinitionBundle,
    pub grammar: intersect_model::definitions::bundle::SourceId,
    /// Optional exact authored behavior selection; only the language frontend
    /// interprets it. Old strict readers reject new selections rather than lose them.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub behavior: Option<intersect_model::definitions::bundle::SourceId>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Package {
    /// Mutually exclusive with ordinary retained compilation. Replay requires
    /// the language frontend; generated schema/grammar are never authority.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub xtext_inference: Option<XtextInferenceInputs>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_policy: Option<CompilationSourcePolicy>,
    pub version: u32,
    pub schema: Schema,
    /// None means a structure-only Rcore package: no text syntax is available.
    pub grammar: Option<crate::grammar::Grammar>,
    pub semantics: Semantics,
    /// Exact submitted definitions retained for audit and persistence.
    pub sources: BTreeMap<String, String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub compilation: Option<BundleCompilation>,
}

impl Package {
    /// Borrow the modeling portion without copying its schema or behavior data.
    /// The returned view grants no source verification or installed providers.
    pub fn model_definition(
        &self,
    ) -> intersect_model::definitions::model_definition::ModelDefinitionView<'_> {
        intersect_model::definitions::model_definition::ModelDefinitionView::new(
            &self.schema,
            &self.semantics,
        )
    }
}

/// Requested admission limits are data, not host authorization.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CompilationSourcePolicy {
    pub structure_bytes: usize,
    pub xtext_bytes: usize,
    pub xtext_declarations: usize,
}
impl Default for CompilationSourcePolicy {
    fn default() -> Self {
        Self {
            structure_bytes: 256_000,
            xtext_bytes: 256_000,
            xtext_declarations: 128,
        }
    }
}
impl CompilationSourcePolicy {
    pub fn admits(self, requested: Self) -> bool {
        requested.structure_bytes <= self.structure_bytes
            && requested.xtext_bytes <= self.xtext_bytes
            && requested.xtext_declarations <= self.xtext_declarations
    }
}

/// Complete reproducible inputs for a multi-resource package; serialized IR is not trusted.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BundleCompilation {
    /// Explicit closed applications for generic schema compilation; retained for exact replay.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub materialized_types: Vec<intersect_model::definitions::generic_types::RuntimeType>,
    pub bundle: intersect_model::definitions::bundle::DefinitionBundle,
    pub ecore: intersect_model::definitions::bundle::SourceId,
    pub namespace: String,
    pub xtext: Option<intersect_model::definitions::bundle::SourceId>,
    pub rcore_behavior: intersect_model::definitions::bundle::SourceId,
}

use crate::grammar::Grammar;
use intersect_model::definitions::model_definition::ModelDefinitionView;

#[derive(Clone, Copy)]
pub struct TextDefinitionView<'a> {
    model: ModelDefinitionView<'a>,
    grammar: &'a Grammar,
    compilation: Option<&'a BundleCompilation>,
    inference: Option<&'a XtextInferenceInputs>,
    source_policy: Option<&'a CompilationSourcePolicy>,
}
impl<'a> TextDefinitionView<'a> {
    pub fn new(model: ModelDefinitionView<'a>, grammar: &'a Grammar) -> Self {
        Self {
            model,
            grammar,
            compilation: None,
            inference: None,
            source_policy: None,
        }
    }
    pub fn from_package(package: &'a Package) -> Result<Self, String> {
        Ok(Self {
            model: package.model_definition(),
            grammar: package.grammar.as_ref().ok_or("Missing text grammar")?,
            compilation: package.compilation.as_ref(),
            inference: package.xtext_inference.as_ref(),
            source_policy: package.source_policy.as_ref(),
        })
    }
    pub fn model(self) -> ModelDefinitionView<'a> {
        self.model
    }
    pub fn grammar(self) -> &'a Grammar {
        self.grammar
    }
    pub fn compilation(self) -> Option<&'a BundleCompilation> {
        self.compilation
    }
    pub fn inference(self) -> Option<&'a XtextInferenceInputs> {
        self.inference
    }
    pub fn source_policy(self) -> Option<&'a CompilationSourcePolicy> {
        self.source_policy
    }
}

/// Borrowed text recipe data. This view retains the source allocation and grants
/// no compilation, provider installation or runtime admission.
#[derive(Clone, Copy, serde::Serialize)]
pub struct TextCompilationView<'a> {
    #[serde(skip_serializing_if = "slice_empty")]
    pub materialized_types: &'a [intersect_model::definitions::generic_types::RuntimeType],
    pub bundle: &'a intersect_model::definitions::bundle::DefinitionBundle,
    pub ecore: &'a intersect_model::definitions::bundle::SourceId,
    pub namespace: &'a str,
    pub xtext: Option<&'a intersect_model::definitions::bundle::SourceId>,
    pub rcore_behavior: &'a intersect_model::definitions::bundle::SourceId,
}
fn slice_empty<T>(value: &&[T]) -> bool {
    value.is_empty()
}
impl<'a> From<&'a BundleCompilation> for TextCompilationView<'a> {
    fn from(value: &'a BundleCompilation) -> Self {
        Self {
            materialized_types: &value.materialized_types,
            bundle: &value.bundle,
            ecore: &value.ecore,
            namespace: &value.namespace,
            xtext: value.xtext.as_ref(),
            rcore_behavior: &value.rcore_behavior,
        }
    }
}
impl TextCompilationView<'_> {
    /// Explicit owned export. Ordinary borrowed preparation never calls this.
    pub fn to_owned(self) -> BundleCompilation {
        BundleCompilation {
            materialized_types: self.materialized_types.to_vec(),
            bundle: self.bundle.clone(),
            ecore: self.ecore.clone(),
            namespace: self.namespace.to_owned(),
            xtext: self.xtext.cloned(),
            rcore_behavior: self.rcore_behavior.clone(),
        }
    }
}

/// Read-only package inspection over checked owners or ordinary DTOs. The view
/// itself is data and cannot publish an executable or inspection owner.
#[derive(Clone, Copy, serde::Serialize)]
pub struct TextPackageView<'a> {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub xtext_inference: Option<&'a XtextInferenceInputs>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_policy: Option<&'a CompilationSourcePolicy>,
    pub version: u32,
    pub schema: &'a intersect_model::definitions::schema::Schema,
    pub grammar: Option<&'a Grammar>,
    pub semantics: &'a intersect_model::definitions::schema::Semantics,
    pub sources: &'a std::collections::BTreeMap<String, String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub compilation: Option<TextCompilationView<'a>>,
}
impl<'a> TextPackageView<'a> {
    pub fn from_package(package: &'a Package) -> Self {
        Self {
            xtext_inference: package.xtext_inference.as_ref(),
            source_policy: package.source_policy.as_ref(),
            version: package.version,
            schema: &package.schema,
            grammar: package.grammar.as_ref(),
            semantics: &package.semantics,
            sources: &package.sources,
            compilation: package.compilation.as_ref().map(Into::into),
        }
    }
    pub(crate) fn same_world(self, other: Self) -> bool {
        fn same<T>(a: Option<&T>, b: Option<&T>) -> bool {
            match (a, b) {
                (Some(a), Some(b)) => std::ptr::eq(a, b),
                (None, None) => true,
                _ => false,
            }
        }
        self.version == other.version
            && std::ptr::eq(self.schema, other.schema)
            && std::ptr::eq(self.semantics, other.semantics)
            && std::ptr::eq(self.sources, other.sources)
            && same(self.grammar, other.grammar)
            && same(self.xtext_inference, other.xtext_inference)
            && same(self.source_policy, other.source_policy)
            && match (self.compilation, other.compilation) {
                (Some(a), Some(b)) => {
                    std::ptr::eq(a.bundle, b.bundle)
                        && a.ecore == b.ecore
                        && a.xtext == b.xtext
                        && a.rcore_behavior == b.rcore_behavior
                        && a.namespace == b.namespace
                        && a.materialized_types == b.materialized_types
                }
                (None, None) => true,
                _ => false,
            }
    }
    pub fn model_definition(self) -> ModelDefinitionView<'a> {
        ModelDefinitionView::new(self.schema, self.semantics)
    }
    /// Explicit ordinary DTO export. No checked owner or admission is returned.
    pub fn to_owned(self) -> Package {
        Package {
            xtext_inference: self.xtext_inference.cloned(),
            source_policy: self.source_policy.copied(),
            version: if self.version == 2 { 2 } else { 0 },
            schema: self.schema.clone(),
            grammar: self.grammar.cloned(),
            semantics: self.semantics.clone(),
            sources: self.sources.clone(),
            compilation: self.compilation.map(TextCompilationView::to_owned),
        }
    }
}

impl<'a> From<&'a Package> for TextPackageView<'a> {
    fn from(value: &'a Package) -> Self {
        Self::from_package(value)
    }
}
impl<'a> From<&TextPackageView<'a>> for TextPackageView<'a> {
    fn from(value: &TextPackageView<'a>) -> Self {
        *value
    }
}

impl Package {
    /// Borrow model selectors without copying metadata or interpreting retained syntax.
    pub fn model_package(
        &self,
    ) -> intersect_model::definitions::model_definition::ModelPackageView<'_> {
        let (structure, behavior, bundle) = match (&self.compilation, &self.xtext_inference) {
            (Some(recipe), None) => (
                Some(&recipe.ecore),
                Some(&recipe.rcore_behavior),
                Some(&recipe.bundle),
            ),
            (None, Some(recipe)) => (None, recipe.behavior.as_ref(), None),
            _ => (None, None, None),
        };
        intersect_model::definitions::model_definition::ModelPackageView {
            definition: self.model_definition(),
            version: if self.version == 2 { 2 } else { 0 },
            structure_text: self.sources.get("ecore").map(String::as_str),
            behavior_text: self.sources.get("rcore_behavior").map(String::as_str),
            structure,
            behavior,
            bundle,
        }
    }
}
impl<'a> From<&'a Package> for ModelDefinitionView<'a> {
    fn from(package: &'a Package) -> Self {
        package.model_definition()
    }
}
impl<'a> From<&'a Package>
    for intersect_model::definitions::model_definition::ModelPackageView<'a>
{
    fn from(package: &'a Package) -> Self {
        package.model_package()
    }
}
// Preserve the observed legacy owner footprint on the two supported pointer widths.
// The physical layout remains pinned after relocation. Logical accounting uses
// the model's portable 64-bit charge on both native and WASM hosts.
const _: () = {
    let expected = if cfg!(target_pointer_width = "64") {
        1696
    } else {
        852
    };
    assert!(std::mem::size_of::<Package>() + std::mem::size_of::<Box<Package>>() == expected);
};
