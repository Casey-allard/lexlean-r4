module
public import Init
public import LexLeanR4.ObjectReadConnectedRangeContext
public import LexLeanR4.ObjectReadRootTraversal
set_option autoImplicit false
set_option maxRecDepth 100000
set_option maxHeartbeats 1000000000
set_option linter.constructorNameAsVariable false
namespace LexLeanR4.ObjectReadConnectedRangeTransport

public theorem objConnectedRangeUnrelatedZero : (0 = 0) := by
  rfl

public theorem objConnectedRangeLifted (count : Nat) : LexLeanR4.ObjectReadConnectedRangeContext.ObjConnectedRangeGoal (count) := by
  induction count with
  | zero =>
    exact LexLeanR4.ObjectReadRootTraversal.objOriginalRootTraversal
  | succ prior ih =>
    cases prior with
    | zero =>
      congr 1
    | succ earlier =>
      congr 1

end LexLeanR4.ObjectReadConnectedRangeTransport
