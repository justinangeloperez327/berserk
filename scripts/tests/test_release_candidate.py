"""Fail-closed checks for candidate identity, soak bounds, and registry lookups."""
import io
from pathlib import Path
import subprocess
import sys
import unittest
import urllib.error
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import verify_release_candidate as candidate


class CandidateChecks(unittest.TestCase):
    def test_soak_cannot_be_disabled_or_exceed_job_budget(self):
        for value in ("0", "20", "299", "901", "-300", "300.0", "inf", ""):
            with self.subTest(value=value), self.assertRaises(ValueError):
                candidate.validate_soak_seconds(value)
        self.assertEqual(candidate.validate_soak_seconds("300"), 300)
        self.assertEqual(candidate.validate_soak_seconds("900"), 900)

    def test_source_requires_full_matching_commit(self):
        with self.assertRaises(ValueError):
            candidate.verify_source("main")
        with patch.object(candidate.subprocess, "check_output", return_value="a" * 40):
            with self.assertRaisesRegex(ValueError, "expected"):
                candidate.verify_source("b" * 40)

    def test_dirty_and_untracked_candidate_is_rejected(self):
        sha = "a" * 40
        with patch.object(candidate.subprocess, "check_output", return_value=sha), patch.object(
            candidate.subprocess, "run", side_effect=subprocess.CalledProcessError(1, "git")
        ):
            with self.assertRaises(subprocess.CalledProcessError):
                candidate.verify_source(sha)
        with patch.object(candidate.subprocess, "check_output", side_effect=[sha, "untracked.rs"]), patch.object(
            candidate.subprocess, "run"
        ):
            with self.assertRaisesRegex(ValueError, "untracked"):
                candidate.verify_source(sha)

    def test_clean_exact_candidate_is_accepted(self):
        sha = "a" * 40
        with patch.object(candidate.subprocess, "check_output", side_effect=[sha, ""]), patch.object(
            candidate.subprocess, "run"
        ):
            self.assertEqual(candidate.verify_source(sha), sha)

    def test_all_normalization_equivalent_spellings_are_observed(self):
        self.assertEqual(candidate.spelling_variants("a-b_c"), ["a-b-c", "a-b_c", "a_b-c", "a_b_c"])
        with patch.object(candidate, "observe_registry_name", side_effect=lambda name: {"spelling": name, "status": "not_registered"}) as lookup:
            result = candidate.check_registry_names(["claw-orm", "berserk"])
        self.assertTrue(result["checked"])
        self.assertEqual({call.args[0] for call in lookup.call_args_list}, {"claw-orm", "claw_orm", "berserk"})

    def test_existing_alias_blocks_first_publication(self):
        def lookup(name):
            return {"spelling": name, "status": "registered", "crate": "claw_orm"} if name == "claw_orm" else {"spelling": name, "status": "not_registered"}
        with patch.object(candidate, "observe_registry_name", side_effect=lookup):
            with self.assertRaisesRegex(ValueError, "claw_orm"):
                candidate.check_registry_names(["claw-orm"])

    def test_only_a_404_means_unregistered(self):
        for status in (401, 403, 429, 500):
            error = urllib.error.HTTPError("https://crates.io", status, "error", {}, None)
            with self.subTest(status=status), patch.object(candidate.urllib.request, "urlopen", side_effect=error):
                with self.assertRaises(ValueError):
                    candidate.observe_registry_name("berserk")
        error = urllib.error.HTTPError("https://crates.io", 404, "not found", {}, None)
        with patch.object(candidate.urllib.request, "urlopen", side_effect=error):
            self.assertEqual(candidate.observe_registry_name("berserk")["status"], "not_registered")

    def test_network_and_malformed_response_do_not_mean_available(self):
        with patch.object(candidate.urllib.request, "urlopen", side_effect=urllib.error.URLError("offline")):
            with self.assertRaises(ValueError):
                candidate.observe_registry_name("berserk")
        for payload in (b"not-json", b"[]", b"{}", b'{"crate":{"id":null}}'):
            response = io.BytesIO(payload)
            response.status = 200
            with self.subTest(payload=payload), patch.object(candidate.urllib.request, "urlopen", return_value=response):
                with self.assertRaises(ValueError):
                    candidate.observe_registry_name("berserk")

    def test_valid_registered_response_is_preserved(self):
        response = io.BytesIO(b'{"crate":{"id":"berserk","repository":"https://example.com"}}')
        response.status = 200
        with patch.object(candidate.urllib.request, "urlopen", return_value=response):
            result = candidate.observe_registry_name("berserk")
        self.assertEqual(result["status"], "registered")
        self.assertEqual(result["crate"], "berserk")


if __name__ == "__main__":
    unittest.main()
