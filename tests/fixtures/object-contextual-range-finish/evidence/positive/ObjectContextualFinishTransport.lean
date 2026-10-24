module
public import Init
public import LexLeanR4.ObjectContextualFinishContext
public import LexLeanR4.ObjectManifestVersion
set_option autoImplicit false
set_option maxRecDepth 100000
set_option maxHeartbeats 1000000000
set_option linter.constructorNameAsVariable false
namespace LexLeanR4.ObjectContextualFinishTransport

public theorem objFinishLifted (count : Nat) : LexLeanR4.ObjectContextualFinishContext.ObjFinishGoal (count) := by
  induction count with
  | zero =>
    exact LexLeanR4.ObjectManifestVersion.objOriginalManifestVersion
  | succ prior ih =>
    cases prior with
    | zero =>
      congr 1
    | succ earlier =>
      congr 1

end LexLeanR4.ObjectContextualFinishTransport
