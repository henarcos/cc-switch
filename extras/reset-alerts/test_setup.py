import json
import subprocess
import tempfile
import unittest
from pathlib import Path

SCRIPT = Path(__file__).with_name('resolve-data-dir.sh')

class ConfigurationLocationTests(unittest.TestCase):
    def resolve(self, home):
        return subprocess.check_output(['/bin/bash',str(SCRIPT),str(home)],text=True).strip()

    def save_override(self, home, value):
        store=home/'Library/Application Support/com.ccswitch.desktop/app_paths.json'
        store.parent.mkdir(parents=True)
        store.write_text(json.dumps({'app_config_dir_override':value}))

    def test_default_and_existing_override_match_the_host_app(self):
        with tempfile.TemporaryDirectory() as tmp:
            home=Path(tmp)
            self.assertEqual(self.resolve(home),str(home/'.cc-switch'))
            custom=home/'Custom data with spaces'; custom.mkdir()
            self.save_override(home,str(custom))
            self.assertEqual(self.resolve(home),str(custom))

    def test_override_whitespace_matches_cc_switch_normalization(self):
        with tempfile.TemporaryDirectory() as tmp:
            home=Path(tmp); custom=home/'custom'; custom.mkdir()
            self.save_override(home, '  '+str(custom)+'  ')
            self.assertEqual(self.resolve(home),str(custom))

    def test_tilde_override_is_resolved_against_the_destination_user(self):
        with tempfile.TemporaryDirectory() as tmp:
            home=Path(tmp); (home/'custom').mkdir()
            self.save_override(home,'~/custom')
            self.assertEqual(self.resolve(home),str(home/'custom'))

    def test_missing_override_uses_the_same_default_as_cc_switch(self):
        with tempfile.TemporaryDirectory() as tmp:
            home=Path(tmp); self.save_override(home,str(home/'gone'))
            self.assertEqual(self.resolve(home),str(home/'.cc-switch'))

class ActivationConfigurationTests(unittest.TestCase):
    def test_native_configuration_writer_produces_loadable_json(self):
        with tempfile.TemporaryDirectory() as tmp:
            target=Path(tmp)/'config.json'
            subprocess.run(['/bin/bash','-ec', 'source "$1"; write_configuration "$2" "$3"', 'test', str(SCRIPT.with_name('configuration.sh')),str(target),'/Applications/CC Switch Reset Alerts.app/Contents/MacOS/ResetAlerts'],check=True)
            self.assertEqual(json.loads(target.read_text()), {'enabled':True,'notifier_path':'/Applications/CC Switch Reset Alerts.app/Contents/MacOS/ResetAlerts'})

if __name__ == '__main__': unittest.main()
