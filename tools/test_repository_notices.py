import hashlib
import unittest

from check_repository_notices import check


class RepositoryNoticeTests(unittest.TestCase):
    def test_missing_or_changed_notice_is_refused(self):
        policy = {'components': [{'id': 'upstream', 'scope': 'Imported source',
                  'inputs': ['imports/'], 'notices': [{'path': 'LICENSE-upstream',
                  'sha256': hashlib.sha256(b'original').hexdigest()}]}]}
        with self.assertRaisesRegex(ValueError, 'Missing notice'):
            check(policy, {'imports/source': {}}, lambda p: b'original')
        files = {'imports/source': {}, 'LICENSE-upstream': {}}
        with self.assertRaisesRegex(ValueError, 'Notice changed'):
            check(policy, files, lambda p: b'replacement')
        self.assertEqual(check(policy, files, lambda p: b'original')[0]['input_files'], 1)

    def test_stale_input_family_cannot_claim_coverage(self):
        policy = {'components': [{'id': 'removed', 'inputs': ['missing/'], 'notices': []}]}
        with self.assertRaisesRegex(ValueError, 'No inputs'):
            check(policy, {}, lambda p: b'')


if __name__ == '__main__':
    unittest.main()
