from pathlib import Path
import sys
import tempfile
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[2] / 'tools'))
from checks.doc_commands import missing_entrypoints


class DocumentCommands(unittest.TestCase):
    def test_deleted_entrypoints_in_command_fences_fail(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            doc = root / 'docs/authoring/bridge.md'
            text = '```text\nblender source.blend --background --python export_reference.py -- options\n```'
            self.assertEqual(missing_entrypoints(text, root, doc), ['export_reference.py'])
            (root / 'tools').mkdir()
            tool = root / 'tools/build.py'
            tool.write_text('')
            text = '```sh\npython3 -u "tools/build.py"\n```'
            self.assertEqual(missing_entrypoints(text, root, doc), [])
            tool.unlink()
            self.assertEqual(missing_entrypoints(text, root, doc), ['tools/build.py'])

    def test_skill_relative_tools_and_explicit_external_inputs(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            skill = root / '.agents/skills/current'
            (skill / 'scripts').mkdir(parents=True)
            (skill / 'scripts/inspect.py').write_text('')
            text = "python scripts/inspect.py\npython3 $TOOL/script.py\nblender --python /external/script.py"
            self.assertEqual(missing_entrypoints(text, root, skill / 'SKILL.md'), [])


if __name__ == '__main__':
    unittest.main()
