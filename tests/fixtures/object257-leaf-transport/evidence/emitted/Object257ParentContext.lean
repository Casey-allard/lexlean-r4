module
public import Init
public import LexLeanR4.R4.ObjectStore
public import LexLeanR4.R4.ObjectTypes
set_option autoImplicit false
set_option maxRecDepth 100000
set_option maxHeartbeats 1000000000
set_option linter.constructorNameAsVariable false
namespace LexLeanR4.Object257ParentContext

@[expose] public def ObjParentResultContext (firstChild : LexLeanR4.R4.ObjectTypes.ObjNodeId) (secondChild : LexLeanR4.R4.ObjectTypes.ObjNodeId) (item : Except (LexLeanR4.R4.ObjectTypes.ObjError) (LexLeanR4.R4.ObjectTypes.ObjRecord)) : Except (LexLeanR4.R4.ObjectTypes.ObjError) (LexLeanR4.R4.ObjectTypes.ObjRecord) := (match item with | Except.error failure => Except.error (failure) | Except.ok leaf => LexLeanR4.R4.ObjectStore.ObjMakeParent (1) (firstChild) (secondChild) ((leaf).id))

@[expose] public def ObjParentResultIterate : (firstChild : LexLeanR4.R4.ObjectTypes.ObjNodeId) -> (secondChild : LexLeanR4.R4.ObjectTypes.ObjNodeId) -> (count : Nat) -> (item : Except (LexLeanR4.R4.ObjectTypes.ObjError) (LexLeanR4.R4.ObjectTypes.ObjRecord)) -> Except (LexLeanR4.R4.ObjectTypes.ObjError) (LexLeanR4.R4.ObjectTypes.ObjRecord)
  | _firstChild, _secondChild, Nat.zero, item => item
  | firstChild, secondChild, Nat.succ prior, item => ObjParentResultContext (firstChild) (secondChild) (ObjParentResultIterate (firstChild) (secondChild) (prior) (item))

end LexLeanR4.Object257ParentContext
