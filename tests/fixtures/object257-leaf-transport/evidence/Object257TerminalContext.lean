module
public import Init
public import LexLeanR4.R4.ObjectStore
public import LexLeanR4.R4.ObjectTypes
set_option autoImplicit false
set_option maxRecDepth 100000
set_option maxHeartbeats 1000000000
set_option linter.constructorNameAsVariable false
namespace LexLeanR4.Object257TerminalContext

@[expose] public def Obj257TerminalContext (frontier : List (LexLeanR4.R4.ObjectTypes.ObjRoot)) (inventory : List (LexLeanR4.R4.ObjectTypes.ObjRecord)) (limits : LexLeanR4.R4.ObjectTypes.ObjLimits) (item : Except (LexLeanR4.R4.ObjectTypes.ObjError) (LexLeanR4.R4.ObjectTypes.ObjRecord)) : Except (LexLeanR4.R4.ObjectTypes.ObjError) (LexLeanR4.R4.ObjectStore.ObjCarryState) := (match item with | Except.error failure => Except.error (failure) | Except.ok leaf => (match LexLeanR4.R4.ObjectStore.ObjStageRecord (inventory) (leaf) (limits) with | Except.error failure => Except.error (failure) | Except.ok staged => (match LexLeanR4.R4.ObjectStore.ObjCarryTriples (38) ((({ id := (leaf).id, scale := 0 } : LexLeanR4.R4.ObjectTypes.ObjRoot) :: frontier)) (staged) (limits) with | Except.error failure => Except.error (failure) | Except.ok carried => LexLeanR4.R4.ObjectStore.ObjFoldAtoms (([] : List (List (Nat)))) ((carried).frontier) ((carried).inventory) (limits))))

@[expose, reducible] public def Obj257TerminalGoal (frontier : List (LexLeanR4.R4.ObjectTypes.ObjRoot)) (inventory : List (LexLeanR4.R4.ObjectTypes.ObjRecord)) (limits : LexLeanR4.R4.ObjectTypes.ObjLimits) (count : Nat) : Prop := (match count with | Nat.zero => (LexLeanR4.R4.ObjectStore.ObjMakeLeaf ((0 :: ([] : List (Nat)))) = Except.ok (({ id := ({ bytes := (128 :: (154 :: (164 :: (144 :: (109 :: (181 :: (157 :: (164 :: (74 :: (198 :: (157 :: (124 :: ([] : List (Nat)))))))))))))) } : LexLeanR4.R4.ObjectTypes.ObjNodeId), scale := 0, payload := (0 :: (0 :: (0 :: (0 :: (0 :: (0 :: (0 :: (0 :: (0 :: (0 :: (0 :: (0 :: (0 :: (0 :: (0 :: (0 :: (0 :: (0 :: (0 :: (0 :: (0 :: (0 :: (0 :: (0 :: (0 :: (0 :: (0 :: (0 :: (0 :: (0 :: (0 :: (0 :: ([] : List (Nat)))))))))))))))))))))))))))))))))) } : LexLeanR4.R4.ObjectTypes.ObjRecord))) | Nat.succ _ => (Obj257TerminalContext (frontier) (inventory) (limits) (LexLeanR4.R4.ObjectStore.ObjMakeLeaf ((0 :: ([] : List (Nat))))) = Obj257TerminalContext (frontier) (inventory) (limits) (Except.ok (({ id := ({ bytes := (128 :: (154 :: (164 :: (144 :: (109 :: (181 :: (157 :: (164 :: (74 :: (198 :: (157 :: (124 :: ([] : List (Nat)))))))))))))) } : LexLeanR4.R4.ObjectTypes.ObjNodeId), scale := 0, payload := (0 :: (0 :: (0 :: (0 :: (0 :: (0 :: (0 :: (0 :: (0 :: (0 :: (0 :: (0 :: (0 :: (0 :: (0 :: (0 :: (0 :: (0 :: (0 :: (0 :: (0 :: (0 :: (0 :: (0 :: (0 :: (0 :: (0 :: (0 :: (0 :: (0 :: (0 :: (0 :: ([] : List (Nat)))))))))))))))))))))))))))))))))) } : LexLeanR4.R4.ObjectTypes.ObjRecord)))))

end LexLeanR4.Object257TerminalContext
