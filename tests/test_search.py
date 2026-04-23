import sys
import os

sys.path.insert(0, os.path.join(os.path.dirname(__file__), '..', 'app'))

from bot import search_sounds


SAMPLE_SOUNDS = [
    {"id": 1, "filename": "cuanto_peor.ogg", "text": "Cuanto peor mejor", "tags": "cuanto peor mejor todos"},
    {"id": 2, "filename": "es_el_vecino.ogg", "text": "Es el vecino", "tags": "vecino alcalde"},
    {"id": 3, "filename": "somos.ogg", "text": "Somos sentimientos", "tags": "somos sentimientos humanos"},
    {"id": 4, "filename": "vino.ogg", "text": "Vino aqui", "tags": "vino aqui tinto"},
    {"id": 5, "filename": "divino.ogg", "text": "Divino", "tags": "divino santo"},
]


class TestSearchSounds:
    def test_exact_word_match(self):
        results = search_sounds("vino", SAMPLE_SOUNDS)
        filenames = [r["filename"] for r in results]
        assert "vino.ogg" in filenames

    def test_partial_word_match(self):
        """'vino' should match 'divino' since 'vino' is a substring of 'divino'."""
        results = search_sounds("vino", SAMPLE_SOUNDS)
        filenames = [r["filename"] for r in results]
        assert "divino.ogg" in filenames

    def test_no_match(self):
        results = search_sounds("xyz", SAMPLE_SOUNDS)
        assert len(results) == 0

    def test_multi_word_query(self):
        results = search_sounds("cuanto peor", SAMPLE_SOUNDS)
        assert len(results) == 1
        assert results[0]["filename"] == "cuanto_peor.ogg"

    def test_empty_query(self):
        results = search_sounds("", SAMPLE_SOUNDS)
        # Empty string should match everything (empty query_words list -> all() returns True)
        assert len(results) > 0

    def test_result_limit(self):
        """Ensure results are capped at TELEGRAM_INLINE_MAX_RESULTS."""
        many_sounds = [{"id": i, "filename": f"s{i}.ogg", "text": f"Sound {i}", "tags": "common tag"}
                       for i in range(100)]
        results = search_sounds("common", many_sounds)
        assert len(results) <= 49  # TELEGRAM_INLINE_MAX_RESULTS + 1
