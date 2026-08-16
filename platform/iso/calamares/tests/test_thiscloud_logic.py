#!/usr/bin/env python3
"""Unit tests for the pure logic of the thiscloud job module."""
import os
import sys
import tempfile
import unittest

HERE = os.path.dirname(os.path.abspath(__file__))
MOD = os.path.normpath(os.path.join(HERE, os.pardir, "modules", "thiscloud"))
sys.path.insert(0, MOD)

import thiscloud_logic  # noqa: E402


class TestConfigToml(unittest.TestCase):
    def test_valid_ip(self):
        self.assertTrue(thiscloud_logic.is_valid_ip("192.168.1.10"))
        self.assertTrue(thiscloud_logic.is_valid_ip("10.0.0.1"))

    def test_invalid_ip(self):
        self.assertFalse(thiscloud_logic.is_valid_ip("999.1.1.1"))
        self.assertFalse(thiscloud_logic.is_valid_ip("not-an-ip"))
        self.assertFalse(thiscloud_logic.is_valid_ip(""))

    def test_valid_role(self):
        self.assertTrue(thiscloud_logic.is_valid_role("master"))
        self.assertTrue(thiscloud_logic.is_valid_role("worker"))
        self.assertFalse(thiscloud_logic.is_valid_role("admin"))

    def test_sanitize_cluster(self):
        self.assertEqual(thiscloud_logic.sanitize_cluster_name("  My Cluster  "), "my-cluster")
        self.assertEqual(thiscloud_logic.sanitize_cluster_name("a.b"), "ab")

    def test_render_config_toml(self):
        toml = thiscloud_logic.render_config_toml(
            cluster="my-cluster", ip="10.0.0.5", role="master", iface="eth0")
        self.assertIn('cluster_name = "my-cluster"', toml)
        self.assertIn('node_ip = "10.0.0.5"', toml)
        self.assertIn('role = "master"', toml)
        self.assertIn('interface = "eth0"', toml)

    def test_write_config(self):
        with tempfile.TemporaryDirectory() as tmp:
            path = thiscloud_logic.write_config(
                tmp, cluster="c", ip="127.0.0.1", role="worker", iface="eth0")
            self.assertEqual(path, os.path.join(tmp, "etc", "thiscloud", "config.toml"))
            self.assertTrue(os.path.isfile(path))
            self.assertIn('cluster_name = "c"', open(path).read())

    def test_services_for_role(self):
        self.assertIn("thiscloudd.service", thiscloud_logic.services_for_role("master"))
        self.assertIn("thiscloud-api.service", thiscloud_logic.services_for_role("master"))
        self.assertIn("thiscloudd.service", thiscloud_logic.services_for_role("worker"))
        self.assertNotIn("thiscloud-api.service", thiscloud_logic.services_for_role("worker"))

    def test_build_init_args(self):
        self.assertEqual(
            thiscloud_logic.build_init_args("10.0.0.5", "master"),
            ["/usr/bin/thiscloud", "init", "--ip", "10.0.0.5", "--role", "master"])


if __name__ == "__main__":
    unittest.main()