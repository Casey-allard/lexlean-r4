module
public import Init
public import LexLeanR4.ObjectReadParent49
public import LexLeanR4.ObjectReadParent49FetchContext
set_option autoImplicit false
set_option maxRecDepth 100000
set_option maxHeartbeats 1000000000
set_option linter.constructorNameAsVariable false
namespace LexLeanR4.ObjectReadParent49FetchTransport

public theorem objParent49FetchUnrelatedZero : (0 = 0) := by
  rfl

public theorem objParent49FetchLifted (count : Nat) : LexLeanR4.ObjectReadParent49FetchContext.ObjParent49FetchGoal (count) := by
  induction count with
  | zero =>
    exact LexLeanR4.ObjectReadParent49.objParent49Decode
  | succ prior ih =>
    cases prior with
    | zero =>
      congr 1
    | succ earlier =>
      congr 1

end LexLeanR4.ObjectReadParent49FetchTransport
