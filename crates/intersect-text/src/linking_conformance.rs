//! Fixed-schema predicates for one eligible linking call, never graph evidence.
use intersect_model::definitions::schema::Schema;
use std::collections::BTreeMap;

type Members = BTreeMap<String, BTreeMap<String, bool>>;

pub(super) struct ConformanceCache<'a> {
    schema: &'a Schema,
    classes: BTreeMap<String, BTreeMap<String, bool>>,
    references: BTreeMap<String, Members>,
    entries: usize,
}
impl<'a> ConformanceCache<'a> {
    const CAPACITY: usize = 256;

    pub(super) fn new(schema: &'a Schema) -> Self {
        Self {
            schema,
            classes: BTreeMap::new(),
            references: BTreeMap::new(),
            entries: 0,
        }
    }

    pub(super) fn class_matches(&mut self, actual: &str, expected: &str) -> bool {
        if let Some(result) = self.classes.get(actual).and_then(|row| row.get(expected)) {
            return *result;
        }
        let result = self.schema.conforms(actual, expected);
        if self.entries < Self::CAPACITY {
            self.classes
                .entry(actual.into())
                .or_default()
                .insert(expected.into(), result);
            self.entries += 1;
        }
        result
    }

    pub(super) fn reference_matches(
        &mut self,
        owner: &str,
        member: &str,
        actual: &str,
    ) -> Result<bool, String> {
        if let Some(result) = self
            .references
            .get(owner)
            .and_then(|members| members.get(member))
            .and_then(|targets| targets.get(actual))
        {
            return Ok(*result);
        }
        // Errors remain canonical on every attempt; only completed predicates
        // enter this cache. False is a type mismatch, not a graph-validation award.
        let result = self.schema.reference_conforms(owner, member, actual)?;
        if self.entries < Self::CAPACITY {
            self.references
                .entry(owner.into())
                .or_default()
                .entry(member.into())
                .or_default()
                .insert(actual.into(), result);
            self.entries += 1;
        }
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use intersect_model::definitions::dynamic::SchemaBuilder;

    #[test]
    fn predicates_remain_canonical_at_capacity_and_failures_do_not_seed_reuse() {
        let mut builder = SchemaBuilder::new("link-predicates", "urn:link-predicates");
        let base = builder.class("Base").unwrap();
        let owner = builder.class("Owner").unwrap();
        owner.reference("target", &base).optional().unwrap();
        for i in 0..ConformanceCache::CAPACITY + 2 {
            builder
                .class(&format!("C{i}"))
                .unwrap()
                .extends(&base)
                .unwrap();
        }
        let schema = builder.build().unwrap();
        let schema = schema.as_schema();
        let mut cache = ConformanceCache::new(schema);
        assert_eq!(
            cache.reference_matches("Owner", "missing", "Base"),
            schema.reference_conforms("Owner", "missing", "Base")
        );
        assert_eq!(cache.entries, 0);
        assert!(cache.class_matches("Base", "Base"));
        assert!(cache.reference_matches("Owner", "target", "Base").unwrap());
        assert!(!cache.reference_matches("Owner", "target", "Owner").unwrap());
        for i in 0..ConformanceCache::CAPACITY + 2 {
            let class = format!("C{i}");
            assert_eq!(
                cache.class_matches(&class, "Base"),
                schema.conforms(&class, "Base")
            );
            assert_eq!(
                cache.reference_matches("Owner", "target", &class),
                schema.reference_conforms("Owner", "target", &class)
            );
        }
        assert_eq!(cache.entries, ConformanceCache::CAPACITY);
        assert!(!cache.classes.contains_key("C257"));
        assert!(!cache.references["Owner"]["target"].contains_key("C257"));
        assert_eq!(
            cache.reference_matches("Owner", "missing", "Base"),
            schema.reference_conforms("Owner", "missing", "Base")
        );
        assert_eq!(cache.entries, ConformanceCache::CAPACITY);
    }
}
