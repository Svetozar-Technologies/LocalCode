use crate::memory::types::*;
use compact_str::CompactString;
use std::collections::HashMap;

/// Entity types recognized by the extractor
#[derive(Clone, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum EntityType {
    Person,
    Location,
    Organization,
    Date,
    Product,
    Custom(CompactString),
}

/// A recognized entity with its normalized form
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Entity {
    /// Original text as found in content
    pub surface_form: CompactString,
    /// Normalized canonical form
    pub canonical: CompactString,
    pub entity_type: EntityType,
}

/// Index of entities across memories for resolution and linking
pub struct EntityIndex {
    /// Canonical entity -> set of memory IDs containing it
    entity_memories: HashMap<CompactString, Vec<MemoryId>>,
    /// Memory ID -> entities found in it
    memory_entities: HashMap<MemoryId, Vec<Entity>>,
    /// Alias resolution: surface form -> canonical form
    aliases: HashMap<CompactString, CompactString>,
}

impl EntityIndex {
    pub fn new() -> Self {
        Self {
            entity_memories: HashMap::new(),
            memory_entities: HashMap::new(),
            aliases: HashMap::new(),
        }
    }

    /// Extract entities from text content and index them
    pub fn index_memory(&mut self, id: MemoryId, content: &str) -> Vec<Entity> {
        let entities = self.extract_entities(content);

        for entity in &entities {
            self.entity_memories
                .entry(entity.canonical.clone())
                .or_default()
                .push(id);
        }

        self.memory_entities.insert(id, entities.clone());
        entities
    }

    /// Remove a memory from the entity index
    pub fn remove_memory(&mut self, id: &MemoryId) {
        if let Some(entities) = self.memory_entities.remove(id) {
            for entity in &entities {
                if let Some(ids) = self.entity_memories.get_mut(&entity.canonical) {
                    ids.retain(|i| i != id);
                }
            }
        }
    }

    /// Find memories sharing entities with the given memory
    pub fn find_related(&self, id: &MemoryId) -> Vec<(MemoryId, Vec<CompactString>)> {
        let entities = match self.memory_entities.get(id) {
            Some(e) => e,
            None => return Vec::new(),
        };

        let mut related: HashMap<MemoryId, Vec<CompactString>> = HashMap::new();

        for entity in entities {
            if let Some(ids) = self.entity_memories.get(&entity.canonical) {
                for related_id in ids {
                    if related_id != id {
                        related
                            .entry(*related_id)
                            .or_default()
                            .push(entity.canonical.clone());
                    }
                }
            }
        }

        let mut results: Vec<_> = related.into_iter().collect();
        results.sort_by(|a, b| b.1.len().cmp(&a.1.len()));
        results
    }

    /// Register an alias for entity normalization
    pub fn add_alias(&mut self, alias: impl Into<CompactString>, canonical: impl Into<CompactString>) {
        self.aliases.insert(alias.into(), canonical.into());
    }

    /// Get entities for a memory
    pub fn get_entities(&self, id: &MemoryId) -> Option<&Vec<Entity>> {
        self.memory_entities.get(id)
    }

    /// Extract entities from text using pattern matching
    fn extract_entities(&self, text: &str) -> Vec<Entity> {
        let mut entities = Vec::new();

        // Person detection: capitalized words not at sentence start
        let words: Vec<&str> = text.split_whitespace().collect();
        let mut i = 0;
        while i < words.len() {
            let word = words[i].trim_matches(|c: char| !c.is_alphanumeric());

            if !word.is_empty() && word.chars().next().map(|c| c.is_uppercase()).unwrap_or(false) {
                // Check for multi-word names (e.g., "New York", "Alice Smith")
                let mut name_parts = vec![word.to_string()];
                let mut j = i + 1;
                while j < words.len() {
                    let next = words[j].trim_matches(|c: char| !c.is_alphanumeric());
                    if !next.is_empty() && next.chars().next().map(|c| c.is_uppercase()).unwrap_or(false) {
                        name_parts.push(next.to_string());
                        j += 1;
                    } else {
                        break;
                    }
                }

                let surface = name_parts.join(" ");
                let surface_cs: CompactString = surface.as_str().into();
                let canonical = self.resolve_alias(&surface_cs);

                // Skip common sentence starters and short words
                if surface.len() > 1 && !is_common_word(&surface.to_lowercase()) {
                    let entity_type = classify_entity(&surface, text);
                    entities.push(Entity {
                        surface_form: surface_cs,
                        canonical,
                        entity_type,
                    });
                }

                i = j;
                continue;
            }

            i += 1;
        }

        // Date detection: common date patterns
        for pattern in extract_date_patterns(text) {
            entities.push(Entity {
                surface_form: pattern.clone(),
                canonical: pattern,
                entity_type: EntityType::Date,
            });
        }

        entities
    }

    fn resolve_alias(&self, surface: &CompactString) -> CompactString {
        self.aliases.get(surface).cloned().unwrap_or_else(|| surface.clone())
    }
}

impl Default for EntityIndex {
    fn default() -> Self {
        Self::new()
    }
}

fn is_common_word(word: &str) -> bool {
    const COMMON: &[&str] = &[
        "the", "this", "that", "these", "those", "my", "your", "his", "her", "its",
        "our", "their", "user", "they", "was", "were", "has", "had", "been", "being",
        "have", "does", "did", "will", "would", "could", "should", "may", "might",
        "can", "shall", "must", "also", "just", "now", "then", "here", "there",
        "when", "where", "why", "how", "what", "which", "who", "whom", "not", "all",
        "each", "every", "both", "few", "more", "most", "other", "some", "such",
    ];
    COMMON.contains(&word)
}

fn classify_entity(surface: &str, context: &str) -> EntityType {
    let lower = surface.to_lowercase();
    let ctx_lower = context.to_lowercase();

    // Location heuristics
    let location_indicators = ["in ", "from ", "to ", "at ", "near ", "city", "country", "airport"];
    for indicator in &location_indicators {
        if ctx_lower.contains(&format!("{}{}", indicator, lower)) {
            return EntityType::Location;
        }
    }

    // Organization heuristics
    let org_suffixes = ["Inc", "Corp", "LLC", "Ltd", "Co", "Foundation", "Institute"];
    if org_suffixes.iter().any(|s| surface.ends_with(s)) {
        return EntityType::Organization;
    }

    // Default to Person for capitalized names
    EntityType::Person
}

fn extract_date_patterns(text: &str) -> Vec<CompactString> {
    let mut dates = Vec::new();

    // Simple date patterns: "January 15", "March 2024", "2024-01-15"
    let months = [
        "january", "february", "march", "april", "may", "june",
        "july", "august", "september", "october", "november", "december",
    ];

    let lower = text.to_lowercase();
    for month in &months {
        if let Some(pos) = lower.find(month) {
            let end = (pos + month.len() + 10).min(text.len());
            let fragment = &text[pos..end];
            // Capture "Month DD" or "Month YYYY"
            let date_str: String = fragment
                .chars()
                .take_while(|c| c.is_alphanumeric() || *c == ' ' || *c == ',')
                .collect();
            if date_str.len() > month.len() + 1 {
                dates.push(date_str.trim().into());
            }
        }
    }

    // ISO date pattern: YYYY-MM-DD
    for word in text.split_whitespace() {
        let clean = word.trim_matches(|c: char| !c.is_alphanumeric() && c != '-');
        if clean.len() == 10 && clean.chars().filter(|c| *c == '-').count() == 2
            && clean[..4].chars().all(|c| c.is_ascii_digit())
        {
            dates.push(clean.into());
        }
    }

    dates
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[test]
    fn test_entity_extraction() {
        let mut index = EntityIndex::new();
        let id = Uuid::now_v7();

        let entities = index.index_memory(id, "Alice went to Paris with Bob on January 15");
        let names: Vec<&str> = entities.iter().map(|e| e.surface_form.as_str()).collect();

        assert!(names.contains(&"Alice"), "Should find Alice, got {:?}", names);
        assert!(names.contains(&"Paris"), "Should find Paris, got {:?}", names);
        assert!(names.contains(&"Bob"), "Should find Bob, got {:?}", names);
    }

    #[test]
    fn test_entity_alias_resolution() {
        let mut index = EntityIndex::new();
        index.add_alias("Bob", "Robert Smith");

        let id = Uuid::now_v7();
        let entities = index.index_memory(id, "Bob likes coffee");

        let bob_entity = entities.iter().find(|e| e.surface_form.as_str() == "Bob");
        assert!(bob_entity.is_some());
        assert_eq!(bob_entity.unwrap().canonical.as_str(), "Robert Smith");
    }

    #[test]
    fn test_find_related_memories() {
        let mut index = EntityIndex::new();

        let id1 = Uuid::now_v7();
        let id2 = Uuid::now_v7();
        let id3 = Uuid::now_v7();

        index.index_memory(id1, "Alice likes Italian food");
        index.index_memory(id2, "Alice went to Paris");
        index.index_memory(id3, "Bob went to Paris");

        // id1 and id2 share "Alice"
        let related = index.find_related(&id1);
        assert!(related.iter().any(|(id, _)| *id == id2));

        // id2 and id3 share "Paris"
        let related2 = index.find_related(&id2);
        assert!(related2.iter().any(|(id, _)| *id == id3));
    }

    #[test]
    fn test_date_extraction() {
        let dates = extract_date_patterns("Meeting on January 15, departure 2024-03-20");
        assert!(!dates.is_empty(), "Should extract dates, got {:?}", dates);
    }
}
