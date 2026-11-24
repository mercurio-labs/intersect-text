//! Handwritten distribution manifest for the bounded language compiler.
//! Files are UTF-8; hosts own archive encoding and I/O.
use crate::package::Package;
use crate::{compiler, grammar::Profile};
use intersect_model::definitions::PackageId;
use serde::{
    Deserialize, Deserializer, Serialize,
    de::{
        self, DeserializeSeed, IntoDeserializer, MapAccess, Visitor, value::MapAccessDeserializer,
    },
};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

pub const PROFILE: &str = "intersect.workbench.v1";
const SOURCE_ARCHIVE_FORMAT: u32 = 2;
const EXECUTION_ARCHIVE_FORMAT: u32 = 3;
const MAX_ARCHIVE_FILES: usize = 64;
const MAX_ARCHIVE_BYTES: usize = 2_000_000;
/// Frozen compatibility descriptor of format-2 legacy ordinary source archives.
/// This names the historical wire contract, not an installed package release.
pub const LEGACY_ORDINARY_COMPILER_VERSION: &str = "0.1.0";
/// Identity used by legacy ordinary archives, independent of package releases.
pub fn model_compiler_identity() -> CompilerIdentity {
    CompilerIdentity {
        name: "intersect-model".into(),
        version: LEGACY_ORDINARY_COMPILER_VERSION.into(),
    }
}
/// Latest supported generated Python asset version for the script.v1 host API.
pub const PYTHON_BINDING_VERSION: u32 = 7;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PackageMetadata {
    pub package_id: String,
    pub release: String,
    pub display_name: String,
    pub description: String,
}

impl PackageMetadata {
    pub fn validate(&self) -> Result<(), String> {
        for (name, value, limit) in [
            ("Package ID", self.package_id.as_str(), 160),
            ("Release", self.release.as_str(), 80),
        ] {
            if value.is_empty()
                || value.len() > limit
                || !value
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b"._-".contains(&b))
                || !value.as_bytes()[0].is_ascii_alphanumeric()
            {
                return Err(format!(
                    "{name} must start with a letter or digit and contain only ASCII letters, digits, dots, underscores or hyphens (maximum {limit} characters)"
                ));
            }
        }
        if self.display_name.trim().is_empty()
            || self.display_name != self.display_name.trim()
            || self.display_name.chars().count() > 120
            || self.display_name.chars().any(char::is_control)
        {
            return Err("Display name must be 1–120 characters without surrounding whitespace or control characters".into());
        }
        if self.description.chars().count() > 2000
            || self
                .description
                .chars()
                .any(|c| c.is_control() && c != '\n' && c != '\r' && c != '\t')
        {
            return Err(
                "Description must be at most 2000 characters without control characters".into(),
            );
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CompilerIdentity {
    pub name: String,
    pub version: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeRequirements {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub computed_settings: Vec<intersect_model::definitions::setting::ComputedSettingContract>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_policy: Option<crate::package::CompilationSourcePolicy>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub conversions: Vec<intersect_model::definitions::conversion::ConversionContract>,
    pub profile: String,
    pub package_format: u32,
    pub rcore_behavior_format: u32,
}

/// Shared with the language-pack container, which is not feature-gated.
pub use intersect_model::definitions::pack::FileDigest;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PythonBinding {
    pub generator: String,
    pub version: u32,
    pub revision: String,
    pub host_api: String,
    pub source: String,
    pub stub: String,
}

/// A language's named execution profiles and its optional default selection.
/// Engine contracts are prepared by the Run-capable host.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ExecutionManifest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default: Option<String>,
    #[serde(deserialize_with = "deserialize_execution_profiles")]
    pub profiles: BTreeMap<String, ExecutionProfile>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ExecutionProfile {
    /// A versioned host engine ID, such as `intersect.state-machine@1`.
    pub engine: String,
    /// A relative archive path to the Rcore bindings and behavior.
    pub source: String,
}

fn deserialize_execution_profiles<'de, D>(
    deserializer: D,
) -> Result<BTreeMap<String, ExecutionProfile>, D::Error>
where
    D: Deserializer<'de>,
{
    struct Profiles;

    impl<'de> Visitor<'de> for Profiles {
        type Value = BTreeMap<String, ExecutionProfile>;

        fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            formatter.write_str("a map of uniquely named execution profiles")
        }

        fn visit_map<M: MapAccess<'de>>(self, mut map: M) -> Result<Self::Value, M::Error> {
            let mut profiles = BTreeMap::new();
            while let Some((name, profile)) = map.next_entry::<String, ExecutionProfile>()? {
                match profiles.entry(name) {
                    std::collections::btree_map::Entry::Vacant(entry) => entry.insert(profile),
                    std::collections::btree_map::Entry::Occupied(entry) => {
                        return Err(de::Error::custom(format!(
                            "Duplicate execution profile name {:?}",
                            entry.key()
                        )));
                    }
                };
            }
            Ok(profiles)
        }
    }

    deserializer.deserialize_map(Profiles)
}

impl ExecutionManifest {
    /// Validate profile declarations and return their distinct source paths.
    pub fn source_paths(&self) -> Result<BTreeSet<&str>, String> {
        if self.profiles.is_empty() {
            return Err("Execution metadata must declare at least one profile".into());
        }
        if let Some(default) = &self.default
            && !self.profiles.contains_key(default)
        {
            return Err(format!("Unknown default execution profile {default:?}"));
        }
        let mut paths = BTreeSet::new();
        for (name, profile) in &self.profiles {
            if !is_archive_identifier(name) {
                return Err(format!("Invalid execution profile name {name:?}"));
            }
            if !profile.engine.split_once('@').is_some_and(|(id, version)| {
                is_archive_identifier(id)
                    && !version.starts_with('0')
                    && version.bytes().all(|b| b.is_ascii_digit())
                    && version.parse::<u32>().is_ok()
            }) {
                return Err(format!(
                    "Execution profile {name:?} requires a versioned engine ID"
                ));
            }
            if !safe_archive_path(&profile.source)
                || !profile.source.ends_with(".rcore")
                || profile.source.chars().any(char::is_control)
            {
                return Err(format!(
                    "Execution profile {name:?} requires a relative .rcore source path"
                ));
            }
            paths.insert(profile.source.as_str());
        }
        Ok(paths)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LanguageManifest {
    pub format_version: u32,
    pub metadata: PackageMetadata,
    /// SHA-256 of the compact JSON tuple (revision domain, retained sources).
    pub revision: String,
    pub ecore_namespace: String,
    pub schema_revision: String,
    pub compiler: CompilerIdentity,
    pub runtime: RuntimeRequirements,
    pub definitions: BTreeMap<String, String>,
    pub compiled_package: String,
    /// Every payload file, excluding this manifest to avoid a circular digest.
    pub files: BTreeMap<String, FileDigest>,
    pub python: Option<PythonBinding>,
}

impl LanguageManifest {
    pub fn identity(&self) -> Result<PackageId, intersect_model::definitions::InvalidPackageId> {
        PackageId::new(&self.metadata.package_id, &self.revision)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Distribution {
    pub manifest: LanguageManifest,
    pub package: Package,
    pub files: BTreeMap<String, String>,
}

/// Format-3 archive metadata. The common language fields and `execution` share
/// one object on the wire. The language revision includes the execution inputs.
#[derive(Clone, Debug, Serialize)]
pub struct ExecutionLanguageManifest {
    #[serde(flatten)]
    pub language: LanguageManifest,
    pub execution: ExecutionManifest,
}

impl<'de> Deserialize<'de> for ExecutionLanguageManifest {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct Manifest;

        impl<'de> Visitor<'de> for Manifest {
            type Value = ExecutionLanguageManifest;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("a format-3 language manifest")
            }

            fn visit_map<M: MapAccess<'de>>(self, map: M) -> Result<Self::Value, M::Error> {
                let mut execution = None;
                let language =
                    LanguageManifest::deserialize(MapAccessDeserializer::new(LanguageFields {
                        map,
                        execution: &mut execution,
                    }))?;
                Ok(ExecutionLanguageManifest {
                    language,
                    execution: execution.ok_or_else(|| de::Error::missing_field("execution"))?,
                })
            }
        }

        deserializer.deserialize_map(Manifest)
    }
}

/// Stream common fields to the original strict manifest decoder while reading
/// the execution declaration separately, preserving duplicate checks in both.
struct LanguageFields<'a, M> {
    map: M,
    execution: &'a mut Option<ExecutionManifest>,
}

impl<'de, M: MapAccess<'de>> MapAccess<'de> for LanguageFields<'_, M> {
    type Error = M::Error;

    fn next_key_seed<K: DeserializeSeed<'de>>(
        &mut self,
        seed: K,
    ) -> Result<Option<K::Value>, Self::Error> {
        while let Some(key) = self.map.next_key::<String>()? {
            if key != "execution" {
                return seed.deserialize(key.into_deserializer()).map(Some);
            }
            if self.execution.is_some() {
                return Err(de::Error::duplicate_field("execution"));
            }
            *self.execution = Some(self.map.next_value()?);
        }
        Ok(None)
    }

    fn next_value_seed<V: DeserializeSeed<'de>>(
        &mut self,
        seed: V,
    ) -> Result<V::Value, Self::Error> {
        self.map.next_value_seed(seed)
    }
}

/// A checked format-3 archive with its retained definition and execution files.
#[derive(Clone, Debug, Serialize)]
pub struct ExecutionDistribution {
    pub manifest: ExecutionLanguageManifest,
    pub package: Package,
    pub files: BTreeMap<String, String>,
}

/// Result of importing an archive through an execution-aware consumer.
#[derive(Clone, Debug, Serialize)]
#[serde(untagged)]
pub enum LanguageArchive {
    Source(Distribution),
    Execution(ExecutionDistribution),
}

impl Distribution {
    /// Attach authoritative execution metadata and Rcore sources to a source
    /// archive before generating revision-bound assets such as Python bindings.
    pub fn with_execution(
        mut self,
        execution: ExecutionManifest,
        sources: BTreeMap<String, String>,
    ) -> Result<ExecutionDistribution, String> {
        if self.manifest.format_version != SOURCE_ARCHIVE_FORMAT {
            return Err("Execution profiles can only be attached to a source archive once".into());
        }
        if self.manifest.python.is_some() {
            return Err("Attach execution profiles before generating Python bindings".into());
        }
        let paths = execution.source_paths()?;
        if !paths.iter().copied().eq(sources.keys().map(String::as_str)) {
            return Err("Execution sources must match the declared profile paths".into());
        }
        check_execution_archive_paths(
            self.files
                .keys()
                .chain(sources.keys())
                .map(String::as_str)
                .chain(["manifest.json"]),
        )?;
        let digests = sources
            .iter()
            .map(|(path, source)| {
                if self.files.contains_key(path) || self.manifest.files.contains_key(path) {
                    return Err(format!("Execution source path collides with {path:?}"));
                }
                Ok((path.as_str(), FileDigest::of(source.as_bytes())))
            })
            .collect::<Result<BTreeMap<_, _>, _>>()?;
        self.manifest.revision = execution_revision(&self.manifest.revision, &execution, &digests)?;
        self.manifest.format_version = EXECUTION_ARCHIVE_FORMAT;
        self.manifest.files.extend(
            digests
                .into_iter()
                .map(|(path, digest)| (path.to_owned(), digest)),
        );
        self.files.extend(sources);
        let manifest = ExecutionLanguageManifest {
            language: self.manifest,
            execution,
        };
        let manifest_bytes = serde_json::to_string_pretty(&manifest)
            .map_err(|error| error.to_string())?
            .len();
        check_archive_bounds(
            self.files.len() + 1,
            self.files.values().map(String::len).sum::<usize>() + manifest_bytes,
        )?;
        Ok(ExecutionDistribution {
            manifest,
            package: self.package,
            files: self.files,
        })
    }
}

fn is_archive_identifier(value: &str) -> bool {
    value
        .bytes()
        .next()
        .is_some_and(|b| b.is_ascii_alphanumeric())
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"._-".contains(&b))
}

fn safe_archive_path(path: &str) -> bool {
    !path.is_empty()
        && !path.contains(['\\', ':'])
        && path
            .split('/')
            .all(|part| !part.is_empty() && part != "." && part != "..")
}

fn check_execution_archive_paths<'a>(
    paths: impl IntoIterator<Item = &'a str>,
) -> Result<(), String> {
    let mut inventory = BTreeSet::new();
    for path in paths {
        if !inventory.insert(path) {
            return Err(format!("Language archive path collides with {path:?}"));
        }
    }
    for path in &inventory {
        for (separator, _) in path.match_indices('/') {
            let parent = &path[..separator];
            if inventory.contains(parent) {
                return Err(format!(
                    "Language archive path collides with a parent file: {path:?} and {parent:?}"
                ));
            }
        }
    }
    Ok(())
}

fn check_archive_bounds(files: usize, bytes: usize) -> Result<(), String> {
    if files > MAX_ARCHIVE_FILES || bytes > MAX_ARCHIVE_BYTES {
        return Err("Language archive exceeds 64 files or 2 MB expanded text".into());
    }
    Ok(())
}

fn execution_revision(
    definitions_revision: &str,
    execution: &ExecutionManifest,
    sources: &BTreeMap<&str, FileDigest>,
) -> Result<String, String> {
    let input = serde_json::to_vec(&(
        "intersect.language.execution-inputs.v1",
        definitions_revision,
        execution,
        sources,
    ))
    .map_err(|error| error.to_string())?;
    Ok(format!("sha256:{:x}", Sha256::digest(input)))
}

/// A host-installed language compiler, never selected as executable code by
/// archive metadata. Definitions own source envelopes and archive verification;
/// the language package owns inference and supplies fresh compiled results.
pub trait XtextInferenceFrontend {
    fn identity(&self) -> CompilerIdentity;
    fn profile(&self) -> Profile;
    /// Installed compiler capabilities; source recompilation selects the actual profile.
    fn supports_profile(&self, profile: Profile) -> bool {
        profile == self.profile()
    }
    fn compile(
        &self,
        inputs: &crate::package::XtextInferenceInputs,
        requested: crate::package::CompilationSourcePolicy,
        host: crate::package::CompilationSourcePolicy,
    ) -> Result<Package, String>;
}
/// Fresh ordinary compilation supplied by an explicitly installed host frontend.
/// The original bundle stays replay authority; this is never deserialized code.
pub struct CompiledBundle {
    pub package: Package,
    pub compiler: CompilerIdentity,
}
/// Language-owned ordinary source preparation over the shared model compiler.
/// Registries/providers belong to the installed implementation, never the archive.
pub trait BundleCompilationFrontend {
    fn compile(
        &self,
        inputs: &crate::package::BundleCompilation,
        profile: Profile,
        settings: &[intersect_model::definitions::setting::ComputedSettingContract],
        requested: Option<crate::package::CompilationSourcePolicy>,
        host: crate::package::CompilationSourcePolicy,
    ) -> Result<CompiledBundle, String>;
}
/// Assemble the same ordinary archive after source-authoritative language preparation.
/// Optional policy preserves legacy ordinary payloads and revision domains exactly.
pub fn build_bundle_with_frontend(
    inputs: crate::package::BundleCompilation,
    metadata: PackageMetadata,
    profile: Profile,
    settings: &[intersect_model::definitions::setting::ComputedSettingContract],
    requested: Option<crate::package::CompilationSourcePolicy>,
    host: crate::package::CompilationSourcePolicy,
    frontend: &dyn BundleCompilationFrontend,
) -> Result<Distribution, String> {
    metadata.validate()?;
    if !host.admits(requested.unwrap_or_default()) {
        return Err("Requested source policy exceeds host admission limits".into());
    }
    if !settings.is_empty() && requested.is_none() {
        return Err(
            "Computed settings require retained bundle compilation and source policy".into(),
        );
    }
    inputs.bundle.validate()?;
    let compiled = frontend.compile(&inputs, profile, settings, requested, host)?;
    let package = compiled.package;
    if package.version != 2
        || package.semantics.version != 1
        || serde_json::to_value(&package.compilation).map_err(|e| e.to_string())?
            != serde_json::to_value(Some(&inputs)).map_err(|e| e.to_string())?
        || inputs.xtext.is_some() != package.grammar.is_some()
        || package.xtext_inference.is_some()
        || package.source_policy != requested
        || package
            .grammar
            .as_ref()
            .is_some_and(|g| g.profile != profile)
        || package
            .grammar
            .as_ref()
            .map_or(&[][..], |g| g.computed_settings.as_slice())
            != settings
        || !matches!(
            compiled.compiler.name.as_str(),
            "intersect-model" | "intersect-text"
        )
    {
        return Err(
            "Ordinary frontend returned inconsistent source inputs or runtime requirements".into(),
        );
    }
    assemble_with_compiler(package, metadata, profile, compiled.compiler)
}

/// Compile original inference inputs with a host-selected frontend, then reuse
/// the ordinary archive assembler. No generated compilation is replay authority.
pub fn build_inferred_with_frontend(
    inputs: crate::package::XtextInferenceInputs,
    metadata: PackageMetadata,
    requested: crate::package::CompilationSourcePolicy,
    host: crate::package::CompilationSourcePolicy,
    frontend: &dyn XtextInferenceFrontend,
) -> Result<Distribution, String> {
    metadata.validate()?;
    if !host.admits(requested) {
        return Err("Requested source policy exceeds host admission limits".into());
    }
    inputs.bundle.validate()?;
    if inputs.bundle.source(&inputs.grammar)?.kind
        != intersect_model::definitions::bundle::SourceKind::Xtext
    {
        return Err("Inference entry must select an Xtext source".into());
    }
    let identity = frontend.identity();
    if identity.name != "intersect-text" {
        return Err("Inference archives require the intersect-text compiler identity".into());
    }
    let package = frontend.compile(&inputs, requested, host)?;
    let profile = package
        .grammar
        .as_ref()
        .map(|grammar| grammar.profile)
        .filter(|profile| frontend.supports_profile(*profile))
        .ok_or("Inference frontend returned inconsistent source inputs or runtime requirements")?;
    if package.version != 2
        || package.semantics.version != 1
        || package.compilation.is_some()
        || package.xtext_inference.as_ref() != Some(&inputs)
        || package.source_policy != Some(requested)
    {
        return Err(
            "Inference frontend returned inconsistent source inputs or runtime requirements".into(),
        );
    }
    assemble_with_compiler(package, metadata, profile, identity)
}

/// Recompile retained definitions so exported IR always matches the source.
/// This is an export constructor, not a loader for untrusted manifests or ZIPs.
pub fn build(
    rcore: &str,
    xtext: &str,
    rcore_behavior: &str,
    metadata: PackageMetadata,
) -> Result<Distribution, String> {
    build_context(
        rcore,
        xtext,
        rcore_behavior,
        metadata,
        None,
        Profile::default(),
    )
}
pub fn build_with_conversions(
    rcore: &str,
    xtext: &str,
    rcore_behavior: &str,
    metadata: PackageMetadata,
    conversions: &intersect_model::definitions::conversion::ConversionRegistry<'_>,
) -> Result<Distribution, String> {
    build_context(
        rcore,
        xtext,
        rcore_behavior,
        metadata,
        Some(conversions),
        Profile::default(),
    )
}
pub fn build_with_profile(
    rcore: &str,
    xtext: &str,
    rcore_behavior: &str,
    metadata: PackageMetadata,
    profile: Profile,
) -> Result<Distribution, String> {
    build_context(rcore, xtext, rcore_behavior, metadata, None, profile)
}
pub fn build_with_conversions_and_profile(
    rcore: &str,
    xtext: &str,
    rcore_behavior: &str,
    metadata: PackageMetadata,
    conversions: &intersect_model::definitions::conversion::ConversionRegistry<'_>,
    profile: Profile,
) -> Result<Distribution, String> {
    build_context(
        rcore,
        xtext,
        rcore_behavior,
        metadata,
        Some(conversions),
        profile,
    )
}
fn build_context(
    rcore: &str,
    xtext: &str,
    rcore_behavior: &str,
    metadata: PackageMetadata,
    conversions: Option<&intersect_model::definitions::conversion::ConversionRegistry<'_>>,
    profile: Profile,
) -> Result<Distribution, String> {
    metadata.validate()?;
    let package = match conversions {
        Some(registry) => compiler::compile_rcore_with_conversions_and_profile(
            rcore,
            Some(xtext),
            Some(rcore_behavior),
            registry,
            profile,
        ),
        None => {
            compiler::compile_rcore_with_profile(rcore, Some(xtext), Some(rcore_behavior), profile)
        }
    }
    .map_err(|d| format!("Cannot package invalid definitions: {d:?}"))?;
    assemble(package, metadata, profile)
}

/// Package the exact multi-source compiler input; executable IR is rebuilt on import.
pub fn build_bundle_with_profile(
    compilation: crate::package::BundleCompilation,
    metadata: PackageMetadata,
    profile: Profile,
) -> Result<Distribution, String> {
    metadata.validate()?;
    let package = compiler::compile_bundle_with_profile(compilation, profile)
        .map_err(|diagnostics| format!("Cannot package invalid definitions: {diagnostics:?}"))?;
    assemble(package, metadata, profile)
}

/// Package retained Rcore definitions with explicit native conversion providers.
pub fn build_bundle_with_conversions_and_profile(
    compilation: crate::package::BundleCompilation,
    metadata: PackageMetadata,
    conversions: &intersect_model::definitions::conversion::ConversionRegistry<'_>,
    profile: Profile,
) -> Result<Distribution, String> {
    metadata.validate()?;
    let package =
        compiler::compile_bundle_with_conversions_and_profile(compilation, conversions, profile)
            .map_err(|diagnostics| {
                format!("Cannot package invalid definitions: {diagnostics:?}")
            })?;
    assemble(package, metadata, profile)
}
/// Retain requested policy separately from immutable compilation source inputs.
pub fn build_bundle_with_source_policy(
    compilation: crate::package::BundleCompilation,
    metadata: PackageMetadata,
    profile: Profile,
    requested: crate::package::CompilationSourcePolicy,
    host: crate::package::CompilationSourcePolicy,
) -> Result<Distribution, String> {
    metadata.validate()?;
    let package =
        compiler::compile_bundle_with_source_policy(compilation, profile, requested, host)
            .map_err(|diagnostics| {
                format!("Cannot package invalid definitions: {diagnostics:?}")
            })?;
    assemble(package, metadata, profile)
}

/// Build a retained-source archive with explicitly selected setting requirements.
/// Requirements are revision-bound metadata; no executable provider is packaged.
pub fn build_bundle_with_settings_and_source_policy(
    compilation: crate::package::BundleCompilation,
    metadata: PackageMetadata,
    profile: Profile,
    settings: &[intersect_model::definitions::setting::ComputedSettingContract],
    requested: crate::package::CompilationSourcePolicy,
    host: crate::package::CompilationSourcePolicy,
) -> Result<Distribution, String> {
    metadata.validate()?;
    let package = compiler::compile_bundle_with_settings_and_source_policy(
        compilation,
        profile,
        settings,
        requested,
        host,
    )
    .map_err(|diagnostics| format!("Cannot package invalid definitions: {diagnostics:?}"))?;
    assemble(package, metadata, profile)
}

fn assemble(
    package: Package,
    metadata: PackageMetadata,
    profile: Profile,
) -> Result<Distribution, String> {
    assemble_with_compiler(package, metadata, profile, model_compiler_identity())
}
fn assemble_with_compiler(
    package: Package,
    metadata: PackageMetadata,
    profile: Profile,
    compiler_identity: CompilerIdentity,
) -> Result<Distribution, String> {
    let settings = package
        .grammar
        .as_ref()
        .map_or(&[][..], |g| g.computed_settings.as_slice());
    let (revision_input, definitions, mut files) =
        match (&package.compilation, &package.xtext_inference) {
            (Some(compilation), None) => (
                serde_json::to_vec(&(
                    "intersect.language.retained-inputs.v1",
                    profile,
                    compilation,
                    package.source_policy,
                    &package.schema.conversion_contracts,
                    settings,
                ))
                .map_err(|e| e.to_string())?,
                BTreeMap::from([("bundle".into(), "definitions/bundle.json".into())]),
                BTreeMap::from([(
                    "definitions/bundle.json".into(),
                    serde_json::to_string_pretty(compilation).map_err(|e| e.to_string())?,
                )]),
            ),
            (None, Some(inference)) => (
                serde_json::to_vec(&(
                    "intersect.language.inferred-inputs.v1",
                    profile,
                    inference,
                    package.source_policy,
                    &package.schema.conversion_contracts,
                    settings,
                ))
                .map_err(|e| e.to_string())?,
                BTreeMap::from([("inference".into(), "definitions/inference.json".into())]),
                BTreeMap::from([(
                    "definitions/inference.json".into(),
                    serde_json::to_string_pretty(inference).map_err(|e| e.to_string())?,
                )]),
            ),
            (Some(_), Some(_)) => {
                return Err("Package selects conflicting source compilation inputs".into());
            }
            (None, None) => return Err("Missing retained compilation inputs".into()),
        };
    let revision = format!("sha256:{:x}", Sha256::digest(&revision_input));
    let compiled_package = "package.json".to_owned();
    files.insert(
        compiled_package.clone(),
        serde_json::to_string_pretty(&package).map_err(|e| e.to_string())?,
    );
    if !package.schema.conversion_contracts.is_empty() {
        files.insert(
            "definitions/conversions.json".into(),
            serde_json::to_string_pretty(&package.schema.conversion_contracts)
                .map_err(|error| error.to_string())?,
        );
    }
    let digests = files
        .iter()
        .map(|(path, contents)| {
            (
                path.clone(),
                FileDigest {
                    sha256: format!("{:x}", Sha256::digest(contents.as_bytes())),
                    bytes: contents.len(),
                },
            )
        })
        .collect();
    Ok(Distribution {
        manifest: LanguageManifest {
            format_version: SOURCE_ARCHIVE_FORMAT,
            metadata,
            revision,
            ecore_namespace: package.schema.uri.clone(),
            schema_revision: package.schema.revision.clone(),
            compiler: compiler_identity,
            runtime: RuntimeRequirements {
                computed_settings: settings.to_vec(),
                source_policy: package.source_policy,
                conversions: package.schema.conversion_contracts.clone(),
                profile: profile.as_str().into(),
                package_format: package.version,
                rcore_behavior_format: package.semantics.version,
            },
            definitions,
            compiled_package,
            files: digests,
            python: None,
        },
        package,
        files,
    })
}

/// Verify an extracted language archive and recompile its exact retained sources.
/// Hosts own ZIP decoding, duplicate-entry detection and expanded-byte limits.
/// Serialized executable IR is compared with fresh compilation, never executed.
pub fn import_files(files: BTreeMap<String, String>) -> Result<Distribution, String> {
    import_files_with_source_policy(files, crate::package::CompilationSourcePolicy::default())
}
/// The host ceiling is not supplied by the archive. Expanded archive byte/file
/// limits and exact-source/compiled-IR verification remain unchanged.
pub fn import_files_with_source_policy(
    files: BTreeMap<String, String>,
    host: crate::package::CompilationSourcePolicy,
) -> Result<Distribution, String> {
    import_files_with_settings_and_source_policy(files, &[], host)
}
/// The host selects expected contracts independently of archive metadata.
/// Exact payload, source, revision and freshly rebuilt IR checks are shared.
pub fn import_files_with_settings_and_source_policy(
    files: BTreeMap<String, String>,
    settings: &[intersect_model::definitions::setting::ComputedSettingContract],
    host: crate::package::CompilationSourcePolicy,
) -> Result<Distribution, String> {
    import_files_context(files, settings, host, None, None)
}
/// Use the same archive verifier with an explicitly installed inference frontend.
/// Ordinary source archives keep their model compiler and existing format.
pub fn import_files_with_inference_frontend(
    files: BTreeMap<String, String>,
    settings: &[intersect_model::definitions::setting::ComputedSettingContract],
    host: crate::package::CompilationSourcePolicy,
    frontend: &dyn XtextInferenceFrontend,
) -> Result<Distribution, String> {
    import_files_context(files, settings, host, Some(frontend), None)
}
/// Verify both producer forms with independently installed source frontends.
/// No manifest or payload installs an executable frontend or provider.
pub fn import_files_with_frontends(
    files: BTreeMap<String, String>,
    settings: &[intersect_model::definitions::setting::ComputedSettingContract],
    host: crate::package::CompilationSourcePolicy,
    inference: Option<&dyn XtextInferenceFrontend>,
    ordinary: Option<&dyn BundleCompilationFrontend>,
) -> Result<Distribution, String> {
    import_files_context(files, settings, host, inference, ordinary)
}
fn import_files_context(
    files: BTreeMap<String, String>,
    settings: &[intersect_model::definitions::setting::ComputedSettingContract],
    host: crate::package::CompilationSourcePolicy,
    inference_frontend: Option<&dyn XtextInferenceFrontend>,
    ordinary_frontend: Option<&dyn BundleCompilationFrontend>,
) -> Result<Distribution, String> {
    match import_archive_context(
        files,
        settings,
        host,
        inference_frontend,
        ordinary_frontend,
        false,
    )? {
        LanguageArchive::Source(distribution) => Ok(distribution),
        LanguageArchive::Execution(_) => {
            Err("Format-3 language archives require execution-aware import".into())
        }
    }
}

pub(crate) fn import_archive_with_frontends(
    files: BTreeMap<String, String>,
    settings: &[intersect_model::definitions::setting::ComputedSettingContract],
    host: crate::package::CompilationSourcePolicy,
    inference_frontend: Option<&dyn XtextInferenceFrontend>,
    ordinary_frontend: Option<&dyn BundleCompilationFrontend>,
) -> Result<LanguageArchive, String> {
    import_archive_context(
        files,
        settings,
        host,
        inference_frontend,
        ordinary_frontend,
        true,
    )
}

fn import_archive_context(
    files: BTreeMap<String, String>,
    settings: &[intersect_model::definitions::setting::ComputedSettingContract],
    host: crate::package::CompilationSourcePolicy,
    inference_frontend: Option<&dyn XtextInferenceFrontend>,
    ordinary_frontend: Option<&dyn BundleCompilationFrontend>,
    allow_execution: bool,
) -> Result<LanguageArchive, String> {
    check_archive_bounds(files.len(), files.values().map(String::len).sum())?;
    if files.keys().any(|p| !safe_archive_path(p)) {
        return Err("Invalid language archive path".into());
    }
    #[derive(Deserialize)]
    struct FormatVersion {
        format_version: u32,
    }
    let manifest_source = files.get("manifest.json").ok_or("Missing manifest.json")?;
    let FormatVersion { format_version } =
        serde_json::from_str(manifest_source).map_err(|e| format!("Invalid manifest: {e}"))?;
    let (manifest, execution) = match format_version {
        SOURCE_ARCHIVE_FORMAT => (
            serde_json::from_str(manifest_source).map_err(|e| format!("Invalid manifest: {e}"))?,
            None,
        ),
        EXECUTION_ARCHIVE_FORMAT if allow_execution => {
            check_execution_archive_paths(files.keys().map(String::as_str))?;
            let ExecutionLanguageManifest {
                language,
                execution,
            } = serde_json::from_str(manifest_source)
                .map_err(|e| format!("Invalid manifest: {e}"))?;
            (language, Some(execution))
        }
        EXECUTION_ARCHIVE_FORMAT => {
            return Err("Format-3 language archives require execution-aware import".into());
        }
        _ => return Err("Unsupported language archive format or runtime profile".into()),
    };
    manifest.metadata.validate()?;
    if manifest.runtime.package_format != 2 || manifest.runtime.rcore_behavior_format != 1 {
        return Err("Unsupported language archive format or runtime profile".into());
    }

    if manifest.runtime.computed_settings != settings {
        return Err("Explicit computed-setting contracts do not match archive requirements".into());
    }
    if !settings.is_empty() && manifest.runtime.source_policy.is_none() {
        return Err(
            "Computed settings require retained bundle compilation and source policy".into(),
        );
    }
    if !host.admits(manifest.runtime.source_policy.unwrap_or_default()) {
        return Err("Requested source policy exceeds host admission limits".into());
    }
    let profile: Profile =
        serde_json::from_value(serde_json::Value::String(manifest.runtime.profile.clone()))
            .map_err(|_| "Unsupported language archive runtime profile")?;
    let inference_frontend =
        if manifest.definitions.len() == 1 && manifest.definitions.contains_key("inference") {
            let frontend = inference_frontend
                .ok_or("Inferred Xtext archives require an installed language frontend")?;
            if !frontend.supports_profile(profile) {
                return Err("Language archive requires a different inference profile".into());
            }
            Some(frontend)
        } else {
            None
        };
    let expected_compiler = if let Some(frontend) = inference_frontend {
        Some(frontend.identity())
    } else {
        if manifest.compiler.name == "intersect-text" && ordinary_frontend.is_none() {
            return Err("Ordinary Xtext archives require an installed language frontend".into());
        }
        if !matches!(
            manifest.compiler.name.as_str(),
            "intersect-model" | "intersect-text"
        ) {
            return Err("Unsupported ordinary source compiler".into());
        }
        ordinary_frontend.is_none().then(model_compiler_identity)
    };
    if expected_compiler
        .as_ref()
        .is_some_and(|expected| &manifest.compiler != expected)
    {
        return Err("Language archive requires a different compiler version".into());
    }
    if manifest.files.len() + 1 != files.len() || manifest.files.contains_key("manifest.json") {
        return Err("Language archive inventory mismatch".into());
    }
    for (path, digest) in &manifest.files {
        let content = files
            .get(path)
            .ok_or_else(|| format!("Missing archive file {path}"))?;
        if content.len() != digest.bytes
            || format!("{:x}", Sha256::digest(content.as_bytes())) != digest.sha256
        {
            return Err(format!("Archive digest mismatch: {path}"));
        }
    }
    if manifest.definitions.len() != 1 {
        return Err("Expected one retained compilation bundle".into());
    }
    let source = |kind| {
        let path = manifest
            .definitions
            .get(kind)
            .ok_or("Missing definition path")?;
        if !manifest.files.contains_key(path) {
            return Err("Definition not in manifest inventory");
        }
        files
            .get(path)
            .map(String::as_str)
            .ok_or("Missing definition source")
    };
    let rebuilt = if let Some(frontend) = inference_frontend {
        let inputs = serde_json::from_str(source("inference")?)
            .map_err(|e| format!("Invalid retained inference inputs: {e}"))?;
        let requested = manifest
            .runtime
            .source_policy
            .ok_or("Missing inference source policy")?;
        build_inferred_with_frontend(inputs, manifest.metadata.clone(), requested, host, frontend)?
    } else {
        let compilation = serde_json::from_str(source("bundle")?)
            .map_err(|e| format!("Invalid retained bundle compilation: {e}"))?;
        let requested = manifest.runtime.source_policy.unwrap_or_default();
        if !host.admits(requested) {
            return Err("Requested source policy exceeds host admission limits".into());
        }
        if let Some(frontend) = ordinary_frontend {
            build_bundle_with_frontend(
                compilation,
                manifest.metadata.clone(),
                profile,
                settings,
                manifest.runtime.source_policy,
                host,
                frontend,
            )?
        } else if !settings.is_empty() {
            build_bundle_with_settings_and_source_policy(
                compilation,
                manifest.metadata.clone(),
                profile,
                settings,
                requested,
                host,
            )?
        } else {
            match manifest.runtime.source_policy {
                Some(policy) => build_bundle_with_source_policy(
                    compilation,
                    manifest.metadata.clone(),
                    profile,
                    policy,
                    host,
                )?,
                None => build_bundle_with_profile(compilation, manifest.metadata.clone(), profile)?,
            }
        }
    };
    if rebuilt.manifest.compiler != manifest.compiler {
        return Err("Language archive requires a different compiler version".into());
    }
    let expected_revision = if let Some(execution) = &execution {
        let digests = execution
            .source_paths()?
            .into_iter()
            .map(|path| {
                if rebuilt.files.contains_key(path)
                    || manifest.definitions.values().any(|source| source == path)
                    || manifest.compiled_package == path
                    || manifest
                        .python
                        .as_ref()
                        .is_some_and(|python| path == python.source || path == python.stub)
                {
                    return Err(format!("Execution source path collides with {path:?}"));
                }
                let digest = manifest
                    .files
                    .get(path)
                    .ok_or_else(|| format!("Missing execution profile source {path:?}"))?;
                Ok((path, digest.clone()))
            })
            .collect::<Result<BTreeMap<_, _>, _>>()?;
        execution_revision(&rebuilt.manifest.revision, execution, &digests)?
    } else {
        rebuilt.manifest.revision
    };
    if expected_revision != manifest.revision
        || rebuilt.manifest.schema_revision != manifest.schema_revision
        || rebuilt.manifest.ecore_namespace != manifest.ecore_namespace
    {
        return Err("Language archive definition identity mismatch".into());
    }
    if serde_json::to_value(&rebuilt.manifest.runtime).map_err(|e| e.to_string())?
        != serde_json::to_value(&manifest.runtime).map_err(|e| e.to_string())?
    {
        return Err(
            "Language archive runtime requirements differ from freshly compiled definitions".into(),
        );
    }
    if !manifest.files.contains_key(&manifest.compiled_package) {
        return Err("Compiled package missing from inventory".into());
    }
    let saved: serde_json::Value = serde_json::from_str(
        files
            .get(&manifest.compiled_package)
            .ok_or("Missing compiled package")?,
    )
    .map_err(|e| e.to_string())?;
    if saved != serde_json::to_value(&rebuilt.package).map_err(|e| e.to_string())? {
        return Err("Archived compiled package differs from freshly compiled definitions".into());
    }
    if let Some(python) = &manifest.python
        && (python.revision != manifest.revision
            || python.generator != "intersect-python-api"
            || python.version != PYTHON_BINDING_VERSION
            || python.host_api != "intersect.script.v1"
            || !manifest.files.contains_key(&python.source)
            || !manifest.files.contains_key(&python.stub)
            || python.source == python.stub)
    {
        return Err("Unsupported or inconsistent Python binding metadata".into());
    }
    let package = rebuilt.package;
    let files = files
        .into_iter()
        .filter(|(p, _)| p != "manifest.json")
        .collect();
    Ok(match execution {
        Some(execution) => LanguageArchive::Execution(ExecutionDistribution {
            manifest: ExecutionLanguageManifest {
                language: manifest,
                execution,
            },
            package,
            files,
        }),
        None => LanguageArchive::Source(Distribution {
            manifest,
            package,
            files,
        }),
    })
}
