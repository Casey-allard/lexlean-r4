module
public import Init
public import LexLeanR4.Object257LeafCertificate
public import LexLeanR4.Object257TerminalContext
public import LexLeanR4.R4.ObjectStore
public import LexLeanR4.R4.ObjectTypes
set_option autoImplicit false
set_option maxRecDepth 100000
set_option maxHeartbeats 1000000000
set_option linter.constructorNameAsVariable false
namespace LexLeanR4.Object257TerminalFoldTransition

public theorem obj257TerminalUnrelatedZero : (0 = 0) := by
  rfl

public theorem obj257TerminalLifted (frontier : List (LexLeanR4.R4.ObjectTypes.ObjRoot)) (inventory : List (LexLeanR4.R4.ObjectTypes.ObjRecord)) (limits : LexLeanR4.R4.ObjectTypes.ObjLimits) (count : Nat) : LexLeanR4.Object257TerminalContext.Obj257TerminalGoal (frontier) (inventory) (limits) (count) := by
  induction count with
  | zero =>
    exact LexLeanR4.Object257LeafCertificate.objTrace257Record10
  | succ prior ih =>
    cases prior with
    | zero =>
      congr 1
    | succ earlier =>
      congr 1

public theorem obj257TerminalFoldTransition (frontier : List (LexLeanR4.R4.ObjectTypes.ObjRoot)) (inventory : List (LexLeanR4.R4.ObjectTypes.ObjRecord)) (limits : LexLeanR4.R4.ObjectTypes.ObjLimits) : (LexLeanR4.R4.ObjectStore.ObjFoldAtoms (((0 :: ([] : List (Nat))) :: ([] : List (List (Nat))))) (frontier) (inventory) (limits) = (match LexLeanR4.R4.ObjectStore.ObjStageRecord (inventory) (({ id := ({ bytes := (128 :: (154 :: (164 :: (144 :: (109 :: (181 :: (157 :: (164 :: (74 :: (198 :: (157 :: (124 :: ([] : List (Nat)))))))))))))) } : LexLeanR4.R4.ObjectTypes.ObjNodeId), scale := 0, payload := (0 :: (0 :: (0 :: (0 :: (0 :: (0 :: (0 :: (0 :: (0 :: (0 :: (0 :: (0 :: (0 :: (0 :: (0 :: (0 :: (0 :: (0 :: (0 :: (0 :: (0 :: (0 :: (0 :: (0 :: (0 :: (0 :: (0 :: (0 :: (0 :: (0 :: (0 :: (0 :: ([] : List (Nat)))))))))))))))))))))))))))))))))) } : LexLeanR4.R4.ObjectTypes.ObjRecord)) (limits) with | Except.error failure => Except.error (failure) | Except.ok staged => (match LexLeanR4.R4.ObjectStore.ObjCarryTriples (38) ((({ id := (({ id := ({ bytes := (128 :: (154 :: (164 :: (144 :: (109 :: (181 :: (157 :: (164 :: (74 :: (198 :: (157 :: (124 :: ([] : List (Nat)))))))))))))) } : LexLeanR4.R4.ObjectTypes.ObjNodeId), scale := 0, payload := (0 :: (0 :: (0 :: (0 :: (0 :: (0 :: (0 :: (0 :: (0 :: (0 :: (0 :: (0 :: (0 :: (0 :: (0 :: (0 :: (0 :: (0 :: (0 :: (0 :: (0 :: (0 :: (0 :: (0 :: (0 :: (0 :: (0 :: (0 :: (0 :: (0 :: (0 :: (0 :: ([] : List (Nat)))))))))))))))))))))))))))))))))) } : LexLeanR4.R4.ObjectTypes.ObjRecord)).id, scale := 0 } : LexLeanR4.R4.ObjectTypes.ObjRoot) :: frontier)) (staged) (limits) with | Except.error failure => Except.error (failure) | Except.ok carried => LexLeanR4.R4.ObjectStore.ObjFoldAtoms (([] : List (List (Nat)))) ((carried).frontier) ((carried).inventory) (limits)))) := by
  exact obj257TerminalLifted (frontier) (inventory) (limits) (1)

end LexLeanR4.Object257TerminalFoldTransition
