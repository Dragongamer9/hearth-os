import unittest
from unittest.mock import patch
import metrics

class TelemetryTests(unittest.TestCase):
    def tearDown(self): metrics._LAST_CPU=None
    def test_first_sample(self):
        metrics._LAST_CPU=None
        with patch.object(metrics,"_cpu_times",return_value=(100,40)):
            self.assertIsNone(metrics._cpu_percent())
    def test_delta(self):
        metrics._LAST_CPU=(100,40)
        with patch.object(metrics,"_cpu_times",return_value=(200,60)):
            self.assertEqual(metrics._cpu_percent(),80.0)
    def test_zero_delta(self):
        metrics._LAST_CPU=(100,40)
        with patch.object(metrics,"_cpu_times",return_value=(100,40)):
            self.assertIsNone(metrics._cpu_percent())
    def test_memory_missing(self):
        with patch.object(metrics,"_text",return_value=""):
            self.assertIsNone(metrics._memory()["percent"])
    def test_memory_values(self):
        with patch.object(metrics,"_text",return_value="MemTotal: 1048576 kB\nMemAvailable: 524288 kB\n"):
            self.assertEqual(metrics._memory()["percent"],50.0)
            self.assertEqual(metrics._memory()["usedGiB"],0.5)
    def test_snapshot_contract(self):
        output=metrics.snapshot()
        for key in ("cpuPercent","memory","thermalSensors","gpuDrivers","fps","gpuPercent"):
            self.assertIn(key,output)
