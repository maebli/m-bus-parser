import unittest
from instructions import instruction_total, regressions


def metric(value, fingerprint="a"):
    return {"name": "Decoder instructions [implementation=rust]", "value": value,
            "extra": f"baseline={fingerprint * 64}; compiler=test"}


def history(value=100, fingerprint="a"):
    return {"entries": {"Parser instruction counts": [{"date": 1, "benches": [metric(value, fingerprint)]}]}}


class InstructionsTest(unittest.TestCase):
    def test_reads_instruction_column(self):
        self.assertEqual(instruction_total("events: Dr Ir Dw\nsummary: 50 1000 20\n"), 1000)

    def test_rejects_missing_empty_and_ambiguous_profiles(self):
        for profile in ("", "events: Dr\nsummary: 2\n", "events: Ir\nsummary: 0\n",
                        "events: Ir\nsummary: 1\nsummary: 2\n", "events: Ir Dr\nsummary: 1\n"):
            with self.subTest(profile=profile), self.assertRaises(ValueError):
                instruction_total(profile)

    def test_threshold_and_improvements(self):
        for value, fails in ((90, False), (105, False), (106, True)):
            self.assertEqual(bool(regressions([metric(value)], history())[0]), fails)

    def test_new_environment_and_first_run_seed_baselines(self):
        self.assertFalse(regressions([metric(1000)], {})[0])
        self.assertFalse(regressions([metric(1000, "b")], history())[0])

    def test_uses_latest_run_without_reaching_into_older_environments(self):
        runs = history()
        runs["entries"]["Parser instruction counts"].append({"date": 2, "benches": [metric(100, "b")]})
        self.assertFalse(regressions([metric(1000)], runs)[0])

    def test_rejects_invalid_baseline(self):
        with self.assertRaises(ValueError):
            regressions([metric(10)], history(0))
