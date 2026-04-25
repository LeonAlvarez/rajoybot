use deunicode::deunicode;

use crate::persistence::Sound;

/// Preprocess a query: transliterate unicode to ASCII, strip punctuation, lowercase.
pub fn preprocess_query(query: &str) -> String {
    deunicode(query)
        .chars()
        .filter(|c| !c.is_ascii_punctuation())
        .collect::<String>()
        .to_lowercase()
}

/// Search sounds whose tags match all query words (substring matching).
///
/// Returns references to matching sounds. The caller controls truncation via `.take()`.
pub fn search_sounds<'a>(query: &str, sounds: &'a [Sound]) -> Vec<&'a Sound> {
    let query_words: Vec<&str> = query.split_whitespace().collect();
    if query_words.is_empty() {
        return Vec::new();
    }

    sounds
        .iter()
        .filter(|sound| {
            let tag_words: Vec<&str> = sound.tags.split_whitespace().collect();
            query_words
                .iter()
                .all(|qw| tag_words.iter().any(|tw| tw.contains(qw)))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sound(id: i64, tags: &str, text: &str) -> Sound {
        Sound {
            id,
            filename: format!("{id}.ogg"),
            text: text.into(),
            tags: tags.into(),
            disabled: false,
        }
    }

    #[test]
    fn preprocess_strips_accents_and_punctuation() {
        assert_eq!(preprocess_query("Cataluña"), "cataluna");
        assert_eq!(preprocess_query("¿Hola?"), "hola");
        assert_eq!(preprocess_query("It's"), "its");
    }

    #[test]
    fn search_basic_match() {
        let sounds = [
            sound(1, "viva el vino", "Viva el vino"),
            sound(2, "cuanto peor mejor", "Cuanto peor mejor"),
        ];
        let results = search_sounds("vino", &sounds);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, 1);
    }

    #[test]
    fn search_substring_match() {
        let sounds = [sound(1, "divino chocolate", "Divino")];
        let results = search_sounds("vino", &sounds);
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn search_multiple_words_all_must_match() {
        let sounds = [
            sound(1, "viva el vino", "Viva el vino"),
            sound(2, "viva la vida", "Viva la vida"),
        ];
        let results = search_sounds("viva vino", &sounds);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, 1);
    }

    #[test]
    fn search_empty_query_returns_nothing() {
        let sounds = [sound(1, "viva el vino", "Viva el vino")];
        assert!(search_sounds("", &sounds).is_empty());
    }
}
