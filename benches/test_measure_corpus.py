"""Guard units and missing measurements in the published benchmark JSON."""
import importlib.util
from pathlib import Path
import tempfile
import unittest

spec = importlib.util.spec_from_file_location("measure_corpus", Path(__file__).with_name("measure-corpus.py"))
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)


class MetricsTest(unittest.TestCase):
    def estimate(self, value=1000):
        return {"mean": {"point_estimate": value, "confidence_interval": {
            "confidence_level": .95, "lower_bound": 900, "upper_bound": 1100}}}

    def test_corpus_latency_and_interval_are_per_frame(self):
        benchmark = {"full_id": "comparison/decode/rust", "throughput": {"Elements": 10}}
        metric = module.metric(benchmark, self.estimate(), "compiler=test", False)
        self.assertEqual(metric["value"], 100)
        self.assertEqual(metric["range"], "95% CI 90.00–110.00")
        self.assertEqual(metric["unit"], "ns/frame")
        self.assertIn("frames/iteration=10", metric["extra"])

    def test_missing_element_count_is_rejected(self):
        benchmark = {"full_id": "comparison/decode/rust", "throughput": {"Bytes": 30}}
        with self.assertRaises(ValueError):
            module.metric(benchmark, self.estimate(), "test", True)

    def test_comparison_implementations_have_same_chart_name(self):
        for implementation in ("rust", "libmbus"):
            benchmark = {"full_id": f"comparison/decode/{implementation}", "throughput": {"Elements": 2}}
            metric = module.metric(benchmark, self.estimate(), "test", True)
            self.assertEqual(metric["value"], 500)
            self.assertEqual(metric["name"], f"Corpus decode latency [implementation={implementation}]")

    def test_missing_results_and_invalid_timings_fail(self):
        with tempfile.TemporaryDirectory() as directory:
            with self.assertRaises(ValueError):
                module.collect(Path(directory), "test", False)
        for value in (0, -1, float("nan"), float("inf")):
            with self.assertRaises(ValueError):
                module.metric({"full_id": "comparison/decode/rust", "throughput": {"Elements": 73}}, self.estimate(value), "test", False)


if __name__ == "__main__":
    unittest.main()
