"""Retrospective sample-local parent chains; no IO or allocation-cause inference."""

from dataclasses import dataclass

import observe
import root_linkage as root
from observe import ObservationError


@dataclass(frozen=True)
class _Identity:
    key: tuple
    ppid: str
    namespace: tuple | None
    parent: tuple | None
    parent_ppid: str | None
    parent_namespace: tuple | None
    reason: str | None


def _parent(row, target):
    evidence = row[13]
    if type(evidence) is not list or not evidence or type(evidence[0]) is not str:
        raise ObservationError('ancestry_parent_shape')
    if evidence[0] != 'observed':
        if row[5] is not None:
            raise ObservationError('ancestry_unobserved_start')
        if evidence == ['not_observed', []]:
            return None, None, None, 'parent_not_observed'
        if (len(evidence) != 4 or evidence[0] != 'unavailable'
                or evidence[1] not in ('vanished', 'denied', 'io_failure', 'parse_error')
                or type(evidence[3]) is not list or len(evidence[3]) > 6
                or (evidence[2] is not None and (type(evidence[2]) is not int
                                                or not 0 <= evidence[2] <= 65535))):
            raise ObservationError('ancestry_unavailable_shape')
        return None, None, None, 'parent_unavailable'
    names = ('stat', 'cgroup', 'status', 'cmdline', 'stat', 'cgroup')
    if len(evidence) != 2 or type(evidence[1]) is not list or len(evidence[1]) != 6:
        raise ObservationError('ancestry_parent_reads')
    raw = []
    for item, name in zip(evidence[1], names):
        if type(item) is not list or len(item) != 2 or item[0] != name:
            raise ObservationError('ancestry_parent_order')
        raw.append(root._bytes(item[1], observe._OBSERVATION_CAPS[name]))
    return _parent_identity(raw, row, target)


def _parent_identity(raw, row, target):
    before, after = observe.parse_stat(raw[0]), observe.parse_stat(raw[4])
    pid, ppid, ns, _, _ = observe.parse_status(raw[2])
    if (before != after or (pid, ppid) != before[:2]
            or before[0] != row[4] or before[2] != row[5]
            or observe.parse_membership(raw[1]) != target
            or observe.parse_membership(raw[5]) != target):
        raise ObservationError('ancestry_parent_identity')
    return (pid, before[2]), ppid, ns, None


def _identity(row, context):
    key, ns, _ = root._identity(row, context.identity[4][0].encode('utf-8'))
    parent, ppid, parent_ns, reason = _parent(row, context.identity[4][0].encode('utf-8'))
    return _Identity(key, row[4], ns, parent, ppid, parent_ns, reason)


def _edge(item, current, depth):
    if item.namespace is None or len(item.namespace) != depth:
        return None, 'namespace_unknown'
    if item.parent is None:
        return None, item.reason
    parent = current.get(item.parent)
    if parent is None:
        return None, 'parent_not_in_sample'
    if (parent.namespace is None or item.parent_namespace is None
            or len(parent.namespace) != depth or len(item.parent_namespace) != depth):
        return None, 'parent_namespace_unknown'
    if parent.ppid != item.parent_ppid or parent.namespace != item.parent_namespace:
        return None, 'parent_changed'
    return parent.key, None


def _classify(item, current, invocation, depth, ambiguous):
    key, chain, seen = item.key, [], set()
    while key not in seen and len(seen) < 64:
        seen.add(key); chain.append(key)
        if key in ambiguous:
            return 'owned_unattributed', 'namespace_ambiguous', tuple(chain)
        if key == invocation:
            return ('invocation_root' if item.key == invocation else 'observed_descendant',
                    None, tuple(chain))
        next_key, reason = _edge(current[key], current, depth)
        if next_key is None:
            return 'owned_unattributed', reason, tuple(chain)
        key = next_key
    return 'owned_unattributed', 'cycle', tuple(chain)


def _ambiguous(items, depth, coordinate):
    if (type(depth) is not int or type(coordinate) is not int
            or not 0 <= coordinate < depth <= 64):
        raise ObservationError('ancestry_namespace_coordinate')
    groups = {}
    for item in items:
        if item.namespace is not None and len(item.namespace) == depth:
            groups.setdefault(item.namespace[coordinate], set()).add(item.key)
    return {key for keys in groups.values() if len(keys) > 1 for key in keys}


def _sample_roles(items, invocation, depth, coordinate):
    if len(items) > 64 or len({item.key[0] for item in items}) != len(items):
        raise ObservationError('ancestry_sample_count_or_duplicate')
    current = {item.key: item for item in items}
    ambiguous = _ambiguous(items, depth, coordinate)
    result = []
    for item in items:
        role, reason, chain = _classify(item, current, invocation, depth, ambiguous)
        result.append({'pid': item.key[0], 'start': item.key[1], 'role': role,
                       'reason': reason, 'chain': chain})
    return tuple(result)


def _families(journal, controls, context, linked):
    identities = {}
    first, last = (root.boundary.decode_ack(controls[i])[3] for i in (1, 3))
    for _, _, row in root._rows(journal):
        if row[0] == 'i':
            identities[row[1]] = _identity(row, context)
        elif row[0] == 's' and first < row[1] <= last:
            items = tuple(identities[item[0]] for item in row[9])
            if linked['root'] is not None:
                roles = _sample_roles(items, linked['root'], context.namespace_depth,
                                      context.namespace_coordinate)
            else:
                roles = tuple({'pid': item.key[0], 'start': item.key[1],
                               'role': 'owned_unattributed', 'reason': linked['reason'],
                               'chain': ()} for item in items)
            yield {'sample': row[1], 'processes': roles, 'live_authority': False}


def iter_invocation_families(journal: bytes, marker: bytes, retrieval: bytes,
                             controls: tuple, context: root.RootContext):
    """Validate root content first, then yield at most64 current roles per sample.

    Nested reads prove only discrete same-sample consistency. Neither an exited
    ancestor nor an identity found only in another sample supplies an edge.
    The input capture, namespace mapping and live custody remain conditional.
    """
    linked = root.join_invocation_root(journal, marker, retrieval, controls, context)
    return _families(journal, controls, context, linked)
