"""Exercise the documentation checker through its CLI in isolated repositories."""
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import unittest


CHECKER = Path(__file__).resolve().parents[1] / 'check_docs.py'


class DocumentationLinks(unittest.TestCase):
    def setUp(self):
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        self.root = Path(temporary.name)
        (self.root / 'scripts').mkdir()
        (self.root / 'docs').mkdir()
        shutil.copyfile(CHECKER, self.root / 'scripts/check_docs.py')
        for name in ('README.md', 'AGENTS.md', 'ARCHITECTURE.md'):
            (self.root / name).write_text('# Project\n')

    def check(self, readme):
        (self.root / 'README.md').write_text(readme)
        return subprocess.run([sys.executable, 'scripts/check_docs.py'], cwd=self.root,
                              capture_output=True, text=True, timeout=10)

    def test_missing_heading_fails(self):
        (self.root / 'docs/guide.md').write_text('# Current behavior\n')
        result = self.check('[Guide](docs/guide.md#old-heading)\n')
        self.assertNotEqual(result.returncode, 0, result.stdout)
        self.assertIn('old-heading', result.stdout)

    def test_missing_html_image_fails(self):
        result = self.check('<img src="docs/missing.png" alt="Preview">\n')
        self.assertNotEqual(result.returncode, 0, result.stdout)
        self.assertIn('missing.png', result.stdout)

    def test_duplicate_heading_anchor_is_valid(self):
        result = self.check('# Topic\n# Topic\n[Second](#topic-1)\n')
        self.assertEqual(result.returncode, 0, result.stdout)

    def test_external_link_is_not_fetched(self):
        result = self.check('[External](https://example.invalid/missing#section)\n')
        self.assertEqual(result.returncode, 0, result.stdout)

    def test_fenced_example_is_not_a_link(self):
        result = self.check('# Real\n```md\n# Example\n[Missing](absent.md)\n```\n[Here](#real)\n')
        self.assertEqual(result.returncode, 0, result.stdout)

    def test_percent_encoded_image_path_is_valid(self):
        (self.root / 'docs/icon preview.png').write_bytes(b'preview')
        result = self.check('![Preview](docs/icon%20preview.png)\n')
        self.assertEqual(result.returncode, 0, result.stdout)


if __name__ == '__main__':
    unittest.main()
