"""Actual synthetic sampler records and sample-local ancestry negatives."""

import dataclasses
import importlib.util
import json
import unittest

import root_linkage
import test_root_linkage
from test_identity import stat

if importlib.util.find_spec('ancestry'):
    import ancestry
else:
    ancestry = None


class AncestryTests(unittest.TestCase):
    def setUp(self):
        self.assertIsNotNone(ancestry, 'sample-local ancestry is not implemented')
        self.root = test_root_linkage.RootLinkageTests()
        self.root.setUp()
        self.addCleanup(self.root.doCleanups)
        self.fixture = self.root.fixture

    def add(self, pid, ppid, start, inner):
        self.fixture.add_pid(pid)
        (self.fixture.proc/pid/'stat').write_bytes(stat(pid, ppid, start))
        (self.fixture.proc/pid/'status').write_bytes(
            f'Pid:\t{pid}\nPPid:\t{ppid}\nNSpid:\t{pid} {inner}\n'.encode())

    def capture(self, members=b'41\n42\n43\n', argv=None):
        self.fixture.cg.joinpath('cgroup.procs').write_bytes(members)
        return self.root.capture(argv)

    def chain(self):
        self.add('42', '41', '901', '3')
        self.add('43', '42', '902', '4')

    def roles(self, args):
        rows = list(ancestry.iter_invocation_families(*args))
        self.assertEqual(len(rows), 1)
        self.assertEqual(rows[0]['sample'], 8)
        self.assertFalse(rows[0]['live_authority'])
        return {r['pid']: r for r in rows[0]['processes']}

    def test_actual_nested_parent_records_link_only_same_sample_chain(self):
        self.chain()
        roles = self.roles(self.capture())
        self.assertEqual(roles['41']['role'], 'invocation_root')
        self.assertEqual(roles['42']['role'], 'observed_descendant')
        self.assertEqual(roles['43']['role'], 'observed_descendant')
        self.assertEqual(roles['43']['chain'], (('43', '902'), ('42', '901'), ('41', '900')))

    def test_outside_census_parent_is_not_read_or_inferred(self):
        self.chain()
        roles = self.roles(self.capture(b'41\n43\n'))
        self.assertEqual(roles['43']['role'], 'owned_unattributed')
        self.assertEqual(roles['43']['reason'], 'parent_not_observed')

    def test_cycles_never_confer_root_ancestry(self):
        self.add('42', '43', '901', '3')
        self.add('43', '42', '902', '4')
        roles = self.roles(self.capture())
        for pid in ('42', '43'):
            self.assertEqual(roles[pid]['role'], 'owned_unattributed')
            self.assertEqual(roles[pid]['reason'], 'cycle')

    def test_child_namespace_absence_stays_unknown(self):
        self.chain()
        (self.fixture.proc/'42'/'status').write_bytes(b'Pid:\t42\nPPid:\t41\n')
        roles = self.roles(self.capture())
        self.assertEqual(roles['42']['reason'], 'namespace_unknown')
        self.assertEqual(roles['43']['role'], 'owned_unattributed')

    def test_ambiguous_inner_namespace_pid_cannot_confer_ancestry(self):
        self.chain()
        (self.fixture.proc/'42'/'status').write_bytes(b'Pid:\t42\nPPid:\t41\nNSpid:\t42 2\n')
        roles = self.roles(self.capture())
        self.assertTrue(all(r['role'] == 'owned_unattributed' for r in roles.values()))

    def test_root_without_exec_evidence_cannot_seed_descendants(self):
        self.chain()
        roles = self.roles(self.capture(argv=b'shell\0'))
        self.assertTrue(all(r['role'] == 'owned_unattributed' for r in roles.values()))
        self.assertTrue(all(r['reason'] == 'exec_not_observed' for r in roles.values()))

    def test_malformed_nested_parent_bytes_or_shape_reject(self):
        self.chain(); args = self.capture()
        row = next(r for r in map(json.loads, args[0].splitlines()) if r[0] == 'i' and r[2] == '43')
        for index in range(6):
            altered = json.loads(json.dumps(row))
            altered[13][1][index][1] = '#'
            with self.subTest(index=index), self.assertRaises(ancestry.ObservationError):
                ancestry._identity(altered, self.root.context)
        altered = json.loads(json.dumps(row)); altered[13][1].reverse()
        with self.assertRaises(ancestry.ObservationError):
            ancestry._identity(altered, self.root.context)

    def test_parent_incarnation_reuse_or_reparenting_is_unknown(self):
        self.chain(); args = self.capture()
        identities = {r[2]: ancestry._identity(r, self.root.context)
                      for r in map(json.loads, args[0].splitlines()) if r[0] == 'i'}
        for field, value in (('key', ('42', '990')), ('ppid', '7'), ('namespace', ('42', '9'))):
            altered = dict(identities)
            altered['42'] = dataclasses.replace(altered['42'], **{field: value})
            roles = ancestry._sample_roles(tuple(altered.values()), ('41', '900'), 2, 1)
            child = next(r for r in roles if r['pid'] == '43')
            with self.subTest(field=field):
                self.assertEqual(child['role'], 'owned_unattributed')

    def test_historical_dictionary_parent_is_not_a_same_sample_parent(self):
        self.chain(); args = self.capture()
        identities = {r[2]: ancestry._identity(r, self.root.context)
                      for r in map(json.loads, args[0].splitlines()) if r[0] == 'i'}
        roles = ancestry._sample_roles((identities['41'], identities['43']), ('41', '900'), 2, 1)
        self.assertEqual(roles[1]['reason'], 'parent_not_in_sample')

    def test_result_is_lazy_without_retain_all_samples(self):
        self.chain(); args = self.capture()
        result = ancestry.iter_invocation_families(*args)
        self.assertIs(iter(result), result)
        self.assertEqual(next(result)['sample'], 8)
        with self.assertRaises(StopIteration): next(result)

    def test_bad_root_custody_rejects_before_iterator_is_returned(self):
        self.chain(); args = self.capture(); args[1] = b'bad'
        with self.assertRaises(ancestry.ObservationError):
            ancestry.iter_invocation_families(*args)

    def test_nonroot_collision_preserves_unrelated_root(self):
        self.chain()
        (self.fixture.proc/'43'/'status').write_bytes(b'Pid:\t43\nPPid:\t42\nNSpid:\t43 3\n')
        roles = self.roles(self.capture())
        self.assertEqual(roles['41']['role'], 'invocation_root')
        self.assertEqual(roles['42']['reason'], 'namespace_ambiguous')
        self.assertEqual(roles['43']['reason'], 'namespace_ambiguous')

    def test_collision_uses_configured_coordinate_not_last(self):
        self.chain(); args = self.capture()
        identities = {r[2]: ancestry._identity(r, self.root.context)
                      for r in map(json.loads, args[0].splitlines()) if r[0] == 'i'}
        namespaces = {'41': ('41', '2', '50'), '42': ('42', '3', '51'), '43': ('43', '3', '52')}
        items = tuple(dataclasses.replace(item, namespace=namespaces[pid],
                      parent_namespace=namespaces.get(item.ppid)) for pid, item in identities.items())
        roles = ancestry._sample_roles(items, ('41', '900'), 3, 1)
        self.assertEqual([r['role'] for r in roles],
                         ['invocation_root', 'owned_unattributed', 'owned_unattributed'])
