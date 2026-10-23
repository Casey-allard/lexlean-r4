module
public import Init
public import LexLeanR4.Object257FirstAtomGoal
public import LexLeanR4.Object257FirstAtomLeaf
public import LexLeanR4.R4.ObjectStore
public import LexLeanR4.R4.ObjectTypes
set_option autoImplicit false
set_option maxRecDepth 100000
set_option maxHeartbeats 1000000000
set_option linter.constructorNameAsVariable false
namespace LexLeanR4.Object257FirstAtomState

public theorem obj257FirstAtomLifted (count : Nat) : LexLeanR4.Object257FirstAtomGoal.Obj257FirstAtomGoal (count) := by
  induction count with
  | zero =>
    exact LexLeanR4.Object257FirstAtomLeaf.objTrace257Record0
  | succ prior ih =>
    cases prior with
    | zero =>
      congr 1
    | succ earlier =>
      congr 1

public theorem obj257FirstAtomState : (LexLeanR4.R4.ObjectStore.ObjFoldAtoms (((0 :: (1 :: (2 :: (3 :: (4 :: (5 :: (6 :: (7 :: (8 :: (9 :: (10 :: (11 :: (12 :: (13 :: (14 :: (15 :: (16 :: (17 :: (18 :: (19 :: (20 :: (21 :: (22 :: (23 :: (24 :: (25 :: (26 :: (27 :: (28 :: (29 :: (30 :: (31 :: ([] : List (Nat)))))))))))))))))))))))))))))))))) :: ([] : List (List (Nat))))) (([] : List (LexLeanR4.R4.ObjectTypes.ObjRoot))) (([] : List (LexLeanR4.R4.ObjectTypes.ObjRecord))) (({ objectBytes := 16777216, records := 1000000, recordBytes := 67108864, returnedBytes := 16777216, visits := 1000000 } : LexLeanR4.R4.ObjectTypes.ObjLimits)) = Except.ok (({ inventory := (({ id := ({ bytes := (63 :: (164 :: (31 :: (162 :: (68 :: (35 :: (68 :: (195 :: (38 :: (116 :: (35 :: (81 :: ([] : List (Nat)))))))))))))) } : LexLeanR4.R4.ObjectTypes.ObjNodeId), scale := 0, payload := (0 :: (1 :: (2 :: (3 :: (4 :: (5 :: (6 :: (7 :: (8 :: (9 :: (10 :: (11 :: (12 :: (13 :: (14 :: (15 :: (16 :: (17 :: (18 :: (19 :: (20 :: (21 :: (22 :: (23 :: (24 :: (25 :: (26 :: (27 :: (28 :: (29 :: (30 :: (31 :: ([] : List (Nat)))))))))))))))))))))))))))))))))) } : LexLeanR4.R4.ObjectTypes.ObjRecord) :: ([] : List (LexLeanR4.R4.ObjectTypes.ObjRecord))), frontier := (({ id := ({ bytes := (63 :: (164 :: (31 :: (162 :: (68 :: (35 :: (68 :: (195 :: (38 :: (116 :: (35 :: (81 :: ([] : List (Nat)))))))))))))) } : LexLeanR4.R4.ObjectTypes.ObjNodeId), scale := 0 } : LexLeanR4.R4.ObjectTypes.ObjRoot) :: ([] : List (LexLeanR4.R4.ObjectTypes.ObjRoot))) } : LexLeanR4.R4.ObjectStore.ObjCarryState))) := by
  exact obj257FirstAtomLifted (1)

end LexLeanR4.Object257FirstAtomState
