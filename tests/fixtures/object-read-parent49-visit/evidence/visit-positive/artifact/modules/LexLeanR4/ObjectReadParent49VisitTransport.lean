module
public import Init
public import LexLeanR4.ObjectReadParent49AuthenticatedFetch
public import LexLeanR4.ObjectReadParent49VisitContext
set_option autoImplicit false
set_option maxRecDepth 100000
set_option maxHeartbeats 1000000000
set_option linter.constructorNameAsVariable false
namespace LexLeanR4.ObjectReadParent49VisitTransport

public theorem objParent49VisitUnrelatedZero : (0 = 0) := by
  rfl

public theorem objParent49VisitLifted (count : Nat) : LexLeanR4.ObjectReadParent49VisitContext.ObjParent49VisitGoal (count) := by
  induction count with
  | zero =>
    exact LexLeanR4.ObjectReadParent49AuthenticatedFetch.objParent49AuthenticatedFetch
  | succ prior ih =>
    cases prior with
    | zero =>
      congr 1
    | succ earlier =>
      congr 1

end LexLeanR4.ObjectReadParent49VisitTransport
