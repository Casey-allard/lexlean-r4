module
public import Init
public import LexLeanR4.HashWireDirect09Controls
public import LexLeanR4.ObjectWholeDigestContext
set_option autoImplicit false
set_option maxRecDepth 100000
set_option maxHeartbeats 1000000000
set_option linter.constructorNameAsVariable false
namespace LexLeanR4.ObjectWholeDigestTransport

public theorem objWholeDigestLifted (count : Nat) : LexLeanR4.ObjectWholeDigestContext.ObjWholeDigestGoal (count) := by
  induction count with
  | zero =>
    exact LexLeanR4.HashWireDirect09Controls.wireDirect09
  | succ prior ih =>
    cases prior with
    | zero =>
      congr 1
    | succ earlier =>
      congr 1

end LexLeanR4.ObjectWholeDigestTransport
