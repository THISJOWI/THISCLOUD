#!/usr/bin/env python3
"""Structural checks for the thiscloudqml view module sources."""
import os
import unittest

HERE = os.path.dirname(os.path.abspath(__file__))
MOD = os.path.normpath(os.path.join(HERE, os.pardir, "modules", "thiscloudqml"))


class TestThisCloudQml(unittest.TestCase):
    def test_files_exist(self):
        for name in ("CMakeLists.txt", "ThisCloudViewStep.h", "ThisCloudViewStep.cpp",
                     "thiscloudqml.qml", "thiscloudqml.conf", "thiscloudqml.qrc"):
            self.assertTrue(os.path.isfile(os.path.join(MOD, name)), name)

    def test_cpp_extends_qmlviewstep(self):
        cpp = open(os.path.join(MOD, "ThisCloudViewStep.h")).read()
        self.assertIn("QmlViewStep", cpp)
        self.assertIn("CALAMARES_PLUGIN_FACTORY_DECLARATION", cpp)

    def test_cpp_writes_globalstorage(self):
        cpp = open(os.path.join(MOD, "ThisCloudViewStep.cpp")).read()
        for key in ("thiscloudRole", "thiscloudClusterName",
                    "thiscloudNodeIp", "thiscloudInterface"):
            self.assertIn(key, cpp)

    def test_qml_has_form_fields(self):
        qml = open(os.path.join(MOD, "thiscloudqml.qml")).read()
        for tok in ("ComboBox", "TextField", "nodeRole", "clusterName",
                    "nodeIp", "interface", "config"):
            self.assertIn(tok, qml)

    def test_qml_has_lifecycle_hooks(self):
        # Calamares QML view steps expose onActivate()/onLeave(); the Next
        # button is owned by ViewManager, so no onNextRequested is needed.
        qml = open(os.path.join(MOD, "thiscloudqml.qml")).read()
        self.assertIn("onActivate", qml)
        self.assertIn("onLeave", qml)

    def test_conf_maps_module(self):
        conf = open(os.path.join(MOD, "thiscloudqml.conf")).read()
        self.assertIn("qmlFilename", conf)


if __name__ == "__main__":
    unittest.main()