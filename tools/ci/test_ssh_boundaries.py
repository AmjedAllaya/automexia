#!/usr/bin/env python3
"""Mutation tests for the actual canonical SSH boundary validator."""
from __future__ import annotations
import importlib.util
import json
from pathlib import Path
import shutil
import sys
import tempfile
import unittest

HERE = Path(__file__).resolve().parent
spec = importlib.util.spec_from_file_location('ssh_boundaries', HERE/'check_ssh_boundaries.py')
BOUNDARY = importlib.util.module_from_spec(spec)
spec.loader.exec_module(BOUNDARY)

class Boundaries(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        package = HERE.parents[1]/'automexia-ssh-integration'
        shutil.copytree(package, self.root/'automexia-ssh-integration', ignore=shutil.ignore_patterns('__pycache__','target'))
        self.write('apps/automexia-terminal/src/context/launch_broker.rs',
            'pub const MANAGED_SESSION_LAUNCH_ENABLED: bool = false;\nconst _: () = assert!(!MANAGED_SESSION_LAUNCH_ENABLED);')
        self.write('apps/automexia-terminal/src/automexia/ssh_integration.rs', '"enhanced_execution_enabled": false')
        self.write('apps/automexia-terminal/src/automexia/connections/direct_openssh.rs', 'native reviewer unchanged')
        self.write('tools/xtask/src/main.rs', '"automexia-ssh-integration"\nrun_python("tools/ci/check_ssh_boundaries.py")?;')
        self.write('tools/ci/validate_repository.py', 'validate_ssh_boundaries()')
        self.write('tests/assurance/feature-test-reinforcement-v1.json', json.dumps({'features': [
            {'id':'extension-contract-runtime','needed_tests':['SSH planning contracts']}]}))
    def write(self, name, value):
        p=self.root/name;p.parent.mkdir(parents=True,exist_ok=True);p.write_text(value,encoding='utf-8')
    def append(self, name, value):
        p=self.root/name;p.write_text(p.read_text(encoding='utf-8')+value,encoding='utf-8')
    def fail(self):
        with self.assertRaises(ValueError): BOUNDARY.validate_repository(self.root)
    def test_canonical_inputs_are_accepted(self):
        self.assertEqual(BOUNDARY.validate_repository(self.root)['runtime_dependencies'],2)
    def test_windows_network_dependency_is_rejected(self):
        self.append('automexia-ssh-integration/Cargo.toml','\n[target.\'cfg(windows)\'.dependencies]\nreqwest="1"\n');self.fail()
    def test_build_dependency_is_rejected(self):
        self.append('automexia-ssh-integration/Cargo.toml','\n[build-dependencies]\ncc="1"\n');self.fail()
    def test_target_build_dependency_is_rejected(self):
        self.append('automexia-ssh-integration/Cargo.toml','\n[target.\'cfg(unix)\'.build-dependencies]\ncc="1"\n');self.fail()
    def test_target_dev_dependency_is_rejected(self):
        self.append('automexia-ssh-integration/Cargo.toml','\n[target.\'cfg(windows)\'.dev-dependencies]\nreqwest="1"\n');self.fail()
    def test_nested_module_filesystem_access_is_rejected(self):
        self.append('automexia-ssh-integration/src/lib.rs','\nmod nested;\n')
        self.write('automexia-ssh-integration/src/nested/mod.rs','use std::fs;');self.fail()
    def test_grouped_std_import_requires_review(self):
        self.write('automexia-ssh-integration/src/nested.rs','use std::{fs, fmt};');self.fail()
    def test_renamed_dependency_is_rejected(self):
        self.append('automexia-ssh-integration/Cargo.toml','\n[dependencies.hidden]\npackage="base64"\nversion="0.23"\n');self.fail()
    def test_optional_feature_expansion_is_rejected(self):
        self.append('automexia-ssh-integration/Cargo.toml','\n[features]\nnetwork=[]\n');self.fail()
    def test_build_script_is_rejected_even_when_disabled(self):
        self.write('automexia-ssh-integration/build.rs','fn main() {}');self.fail()
    def test_publication_is_rejected(self):
        p=self.root/'automexia-ssh-integration/Cargo.toml';p.write_text(p.read_text(encoding='utf-8').replace('publish = false','publish = true'),encoding='utf-8');self.fail()
    def test_unregistered_custom_lib_root_is_rejected(self):
        self.append('automexia-ssh-integration/Cargo.toml','\n[lib]\npath="other.rs"\n');self.fail()
    def test_production_binary_is_rejected(self):
        self.write('automexia-ssh-integration/src/main.rs','fn main() {}');self.fail()
    def test_macro_source_inclusion_is_rejected(self):
        self.append('automexia-ssh-integration/src/lib.rs','\ninclude!("hidden.rs");');self.fail()
    def test_source_path_escape_is_rejected(self):
        self.append('automexia-ssh-integration/src/lib.rs','\n#[path="../../other.rs"] mod escape;');self.fail()
    def test_second_embedded_resource_is_rejected(self):
        self.append('automexia-ssh-integration/src/lib.rs','\nconst X:&str=include_str!("../other");');self.fail()
    def test_enabling_broker_is_rejected(self):
        p=self.root/'apps/automexia-terminal/src/context/launch_broker.rs';p.write_text(p.read_text(encoding='utf-8').replace('= false','= true'),encoding='utf-8');self.fail()
    def test_removing_compile_time_denial_is_rejected(self):
        self.write('apps/automexia-terminal/src/context/launch_broker.rs','pub const MANAGED_SESSION_LAUNCH_ENABLED: bool = false;');self.fail()
    def test_false_live_claim_is_rejected(self):
        self.write('apps/automexia-terminal/src/automexia/ssh_integration.rs','"enhanced_execution_enabled": true');self.fail()
    def test_unscoped_remote_osc7_is_rejected(self):
        self.append('automexia-ssh-integration/resources/bash-core.bash',"\nprintf '\\e]7;file://remote/x\\a'\n");self.fail()
    def test_reintroduced_review_forwarder_is_rejected(self):
        self.write('automexia-ssh-integration/src/reviewed.rs','// duplicate');self.fail()
    def test_changed_native_review_path_is_rejected(self):
        self.append('apps/automexia-terminal/src/automexia/connections/direct_openssh.rs','ssh_integration::assess_reviewed');self.fail()
    def test_missing_xtask_registration_is_rejected(self):
        self.write('tools/xtask/src/main.rs','');self.fail()
    def test_missing_repository_registration_is_rejected(self):
        self.write('tools/ci/validate_repository.py','');self.fail()
    def test_missing_scenario_evidence_is_rejected(self):
        self.write('tests/assurance/feature-test-reinforcement-v1.json','{"features":[]}');self.fail()
    def test_source_file_count_is_bounded(self):
        for n in range(65): self.write(f'automexia-ssh-integration/src/generated/n{n}.rs','// fixture')
        self.fail()
    def test_linked_source_guard_rejects_native_or_injected_metadata(self):
        p=self.root/'automexia-ssh-integration/src/link.rs'
        try: p.symlink_to('lib.rs')
        except OSError:
            # Mandatory policy test still runs without Windows symlink rights.
            # This branch proves the guard, NOT native link behavior.
            from unittest.mock import patch
            original = Path.is_symlink
            p.write_text('// injected link metadata',encoding='utf-8')
            with patch.object(Path,'is_symlink',lambda value: value == p or original(value)):
                self.fail()
            return
        self.fail()
    def test_metadata_validator_is_static_not_a_build_runner(self):
        source=(HERE/'check_ssh_boundaries.py').read_text(encoding='utf-8')
        self.assertNotIn('import subprocess',source)
        self.assertNotIn('cargo ready',source)

if __name__=='__main__': unittest.main()
