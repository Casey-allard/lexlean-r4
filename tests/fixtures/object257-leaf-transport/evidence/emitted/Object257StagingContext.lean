module
public import Init
public import LexLeanR4.R4.ObjectStore
public import LexLeanR4.R4.ObjectTypes
set_option autoImplicit false
set_option maxRecDepth 100000
set_option maxHeartbeats 1000000000
set_option linter.constructorNameAsVariable false
namespace LexLeanR4.Object257StagingContext

@[expose] public def ObjLeafStagingContext (inventory : List (LexLeanR4.R4.ObjectTypes.ObjRecord)) (limits : LexLeanR4.R4.ObjectTypes.ObjLimits) (item : Except (LexLeanR4.R4.ObjectTypes.ObjError) (LexLeanR4.R4.ObjectTypes.ObjRecord)) : Except (LexLeanR4.R4.ObjectTypes.ObjError) (List (LexLeanR4.R4.ObjectTypes.ObjRecord)) := (match item with | Except.error failure => Except.error (failure) | Except.ok leaf => LexLeanR4.R4.ObjectStore.ObjStageRecord (inventory) (leaf) (limits))

@[expose, reducible] public def ObjLeafStagingGoal (inventory : List (LexLeanR4.R4.ObjectTypes.ObjRecord)) (limits : LexLeanR4.R4.ObjectTypes.ObjLimits) (count : Nat) : Prop := (match count with | Nat.zero => (LexLeanR4.R4.ObjectStore.ObjMakeLeaf ((0 :: ([] : List (Nat)))) = Except.ok (({ id := ({ bytes := (128 :: (154 :: (164 :: (144 :: (109 :: (181 :: (157 :: (164 :: (74 :: (198 :: (157 :: (124 :: ([] : List (Nat)))))))))))))) } : LexLeanR4.R4.ObjectTypes.ObjNodeId), scale := 0, payload := (0 :: (0 :: (0 :: (0 :: (0 :: (0 :: (0 :: (0 :: (0 :: (0 :: (0 :: (0 :: (0 :: (0 :: (0 :: (0 :: (0 :: (0 :: (0 :: (0 :: (0 :: (0 :: (0 :: (0 :: (0 :: (0 :: (0 :: (0 :: (0 :: (0 :: (0 :: (0 :: ([] : List (Nat)))))))))))))))))))))))))))))))))) } : LexLeanR4.R4.ObjectTypes.ObjRecord))) | Nat.succ _ => (ObjLeafStagingContext (inventory) (limits) (LexLeanR4.R4.ObjectStore.ObjMakeLeaf ((0 :: ([] : List (Nat))))) = ObjLeafStagingContext (inventory) (limits) (Except.ok (({ id := ({ bytes := (128 :: (154 :: (164 :: (144 :: (109 :: (181 :: (157 :: (164 :: (74 :: (198 :: (157 :: (124 :: ([] : List (Nat)))))))))))))) } : LexLeanR4.R4.ObjectTypes.ObjNodeId), scale := 0, payload := (0 :: (0 :: (0 :: (0 :: (0 :: (0 :: (0 :: (0 :: (0 :: (0 :: (0 :: (0 :: (0 :: (0 :: (0 :: (0 :: (0 :: (0 :: (0 :: (0 :: (0 :: (0 :: (0 :: (0 :: (0 :: (0 :: (0 :: (0 :: (0 :: (0 :: (0 :: (0 :: ([] : List (Nat)))))))))))))))))))))))))))))))))) } : LexLeanR4.R4.ObjectTypes.ObjRecord)))))

end LexLeanR4.Object257StagingContext
