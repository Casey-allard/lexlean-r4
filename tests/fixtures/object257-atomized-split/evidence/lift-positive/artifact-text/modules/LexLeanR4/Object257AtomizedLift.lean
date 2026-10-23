module
public import Init
public import LexLeanR4.Object257AtomizedContext
public import LexLeanR4.Object257FirstAtomState
public import LexLeanR4.R4.ObjectStore
public import LexLeanR4.R4.ObjectTypes
set_option autoImplicit false
set_option maxRecDepth 100000
set_option maxHeartbeats 1000000000
set_option linter.constructorNameAsVariable false
namespace LexLeanR4.Object257AtomizedLift

public theorem obj257AtomizedUnrelatedZero : (0 = 0) := by
  rfl

public theorem obj257AtomizedLifted (count : Nat) : LexLeanR4.Object257AtomizedContext.Obj257AtomizedGoal (count) := by
  induction count with
  | zero =>
    exact LexLeanR4.Object257FirstAtomState.obj257FirstAtomState
  | succ prior ih =>
    cases prior with
    | zero =>
      congr 1
    | succ earlier =>
      congr 1

end LexLeanR4.Object257AtomizedLift
