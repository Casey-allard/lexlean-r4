module
public import Init
public import LexLeanR4.Object257LeafCertificate
public import LexLeanR4.Object257StagingContext
public import LexLeanR4.R4.ObjectStore
public import LexLeanR4.R4.ObjectTypes
set_option autoImplicit false
set_option maxRecDepth 100000
set_option maxHeartbeats 1000000000
set_option linter.constructorNameAsVariable false
namespace LexLeanR4.Object257StagingTransport

public theorem obj257StagingUnrelatedZero : (0 = 0) := by
  rfl

public theorem obj257LeafStagingLifted (inventory : List (LexLeanR4.R4.ObjectTypes.ObjRecord)) (limits : LexLeanR4.R4.ObjectTypes.ObjLimits) (count : Nat) : LexLeanR4.Object257StagingContext.ObjLeafStagingGoal (inventory) (limits) (count) := by
  induction count with
  | zero =>
    exact LexLeanR4.Object257LeafCertificate.objTrace257Record10
  | succ prior ih =>
    cases prior with
    | zero =>
      congr 1
    | succ earlier =>
      congr 1

public theorem obj257LeafStagingConsumer (inventory : List (LexLeanR4.R4.ObjectTypes.ObjRecord)) (limits : LexLeanR4.R4.ObjectTypes.ObjLimits) : ((match LexLeanR4.R4.ObjectStore.ObjMakeLeaf ((0 :: ([] : List (Nat)))) with | Except.error failure => Except.error (failure) | Except.ok leaf => LexLeanR4.R4.ObjectStore.ObjStageRecord (inventory) (leaf) (limits)) = LexLeanR4.R4.ObjectStore.ObjStageRecord (inventory) (({ id := ({ bytes := (128 :: (154 :: (164 :: (144 :: (109 :: (181 :: (157 :: (164 :: (74 :: (198 :: (157 :: (124 :: ([] : List (Nat)))))))))))))) } : LexLeanR4.R4.ObjectTypes.ObjNodeId), scale := 0, payload := (0 :: (0 :: (0 :: (0 :: (0 :: (0 :: (0 :: (0 :: (0 :: (0 :: (0 :: (0 :: (0 :: (0 :: (0 :: (0 :: (0 :: (0 :: (0 :: (0 :: (0 :: (0 :: (0 :: (0 :: (0 :: (0 :: (0 :: (0 :: (0 :: (0 :: (0 :: (0 :: ([] : List (Nat)))))))))))))))))))))))))))))))))) } : LexLeanR4.R4.ObjectTypes.ObjRecord)) (limits)) := by
  exact obj257LeafStagingLifted (inventory) (limits) (1)

end LexLeanR4.Object257StagingTransport
