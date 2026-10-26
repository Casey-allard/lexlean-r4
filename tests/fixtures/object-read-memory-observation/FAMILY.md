# Owned parent observation in a forced sample

`family.owned_family_evidence` connects the existing nested consistency check to
bounded, pinned filesystem reads. It opens at most the child and one parent from
the same fresh owned-cgroup census. It reads the parent between the child's
initial and final identity checks, then independently closes its PID directory
descriptors. The caller's root descriptors stay open. No proc discovery, recursive
grandparent walk or unrelated process read is added.

The returned transcript contains the exact successful reads in order. A complete
observation has child stat/cgroup/status/cmdline, parent
stat/cgroup/status/cmdline/stat/cgroup, then child stat/cgroup. An ordinary parent
open/read/parse failure can leave the child valid with `parent_start=null`; its
original exception and partial parent transcript are retained. Child, outer-root,
topology and cleanup failures remain fatal. Parent file reads opt into the bounded
reader's `cleanup_fatal` mode: actual cleanup failures form an exception group,
even for one close failure, and cannot be mistaken for ordinary parent loss.
Other callers retain the reader's original default error behavior.

The sample header is now version 3. Existing identity fields retain their order;
one final field records the nested parent:

- `["observed", reads]` after the complete nested check;
- `["not_observed", []]` for an outside-census or self parent;
- `["unavailable", reason, errnoOrNull, partialReads]` after ordinary parent loss.

Each read is `[fixedFilename, base64ActualBytes]`. The parent PID/start/PPID,
namespace and command can be reconstructed from those exact records. Failure
reasons are finite, and arbitrary exception text is never written. The entire
parent field participates in the identity fingerprint. Nested parent reads do
not add another memory row: the ordinary sample still has one row per census PID.

All identity and journal quotas stay unchanged. Additional evidence can exhaust
the existing 8 KiB identity row limit or identity pool; that stops observation
before an ACK rather than truncating evidence. This does not guarantee complete
coverage of the earlier run or bound Python RSS to encoded payload size.

This is discrete consistency, not atomic ancestry. Unobserved ABA races remain
possible. Role stays `owned_unattributed` and root binding stays null. Actual
marker/exec identity, namespace mapping, live preflight, periodic scheduling,
control publication and final invocation remain required before the one native
diagnostic. No compiler, VM restart or resource acceptance occurs in these tests.
