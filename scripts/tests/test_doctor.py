"""Pin the project bootstrap's SDK location without installing tools."""
from pathlib import Path
import sys
import tempfile
import unittest
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import doctor


class AndroidSDKDiscovery(unittest.TestCase):
    def test_default_project_sdk_is_found_without_environment(self):
        with tempfile.TemporaryDirectory() as directory:
            home = Path(directory)
            sdk = home / '.local/share/holocron/android-sdk'
            adb = sdk / 'platform-tools/adb'
            adb.parent.mkdir(parents=True)
            adb.touch(mode=0o755)
            with patch.dict('os.environ', {}, clear=True), patch.object(Path, 'home', return_value=home):
                result = doctor.android_sdk()
            self.assertIn(str(sdk), [entry['root'] for entry in result['found']])


if __name__ == '__main__':
    unittest.main()
