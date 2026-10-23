module
public import Init
public import LexLeanR4.Object257LeafCertificate
public import LexLeanR4.Object257ParentContext
public import LexLeanR4.R4.ObjectStore
public import LexLeanR4.R4.ObjectTypes
set_option autoImplicit false
set_option maxRecDepth 100000
set_option maxHeartbeats 1000000000
set_option linter.constructorNameAsVariable false
namespace LexLeanR4.Object257ParentTransport

public theorem obj257UnrelatedZero : (0 = 0) := by
  rfl

public theorem obj257LeafParentLifted (firstChild : LexLeanR4.R4.ObjectTypes.ObjNodeId) (secondChild : LexLeanR4.R4.ObjectTypes.ObjNodeId) (count : Nat) : (LexLeanR4.Object257ParentContext.ObjParentResultIterate (firstChild) (secondChild) (count) (LexLeanR4.R4.ObjectStore.ObjMakeLeaf ((0 :: ([] : List (Nat))))) = LexLeanR4.Object257ParentContext.ObjParentResultIterate (firstChild) (secondChild) (count) (Except.ok (({ id := ({ bytes := (128 :: (154 :: (164 :: (144 :: (109 :: (181 :: (157 :: (164 :: (74 :: (198 :: (157 :: (124 :: ([] : List (Nat)))))))))))))) } : LexLeanR4.R4.ObjectTypes.ObjNodeId), scale := 0, payload := (0 :: (0 :: (0 :: (0 :: (0 :: (0 :: (0 :: (0 :: (0 :: (0 :: (0 :: (0 :: (0 :: (0 :: (0 :: (0 :: (0 :: (0 :: (0 :: (0 :: (0 :: (0 :: (0 :: (0 :: (0 :: (0 :: (0 :: (0 :: (0 :: (0 :: (0 :: (0 :: ([] : List (Nat)))))))))))))))))))))))))))))))))) } : LexLeanR4.R4.ObjectTypes.ObjRecord)))) := by
  induction count with
  | zero =>
    exact LexLeanR4.Object257LeafCertificate.objTrace257Record10
  | succ prior ih =>
    congr 1

public theorem obj257LeafParentConsumer (firstChild : LexLeanR4.R4.ObjectTypes.ObjNodeId) (secondChild : LexLeanR4.R4.ObjectTypes.ObjNodeId) : ((match LexLeanR4.R4.ObjectStore.ObjMakeLeaf ((0 :: ([] : List (Nat)))) with | Except.error failure => Except.error (failure) | Except.ok leaf => LexLeanR4.R4.ObjectStore.ObjMakeParent (1) (firstChild) (secondChild) ((leaf).id)) = LexLeanR4.R4.ObjectStore.ObjMakeParent (1) (firstChild) (secondChild) (({ bytes := (128 :: (154 :: (164 :: (144 :: (109 :: (181 :: (157 :: (164 :: (74 :: (198 :: (157 :: (124 :: ([] : List (Nat)))))))))))))) } : LexLeanR4.R4.ObjectTypes.ObjNodeId))) := by
  exact obj257LeafParentLifted (firstChild) (secondChild) (1)

end LexLeanR4.Object257ParentTransport
