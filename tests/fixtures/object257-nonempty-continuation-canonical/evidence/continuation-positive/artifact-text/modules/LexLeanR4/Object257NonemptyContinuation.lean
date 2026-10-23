module
public import Init
public import LexLeanR4.Object257FirstAtomState
public import LexLeanR4.Object257NonemptyContext
public import LexLeanR4.R4.ObjectStore
public import LexLeanR4.R4.ObjectTypes
set_option autoImplicit false
set_option maxRecDepth 100000
set_option maxHeartbeats 1000000000
set_option linter.constructorNameAsVariable false
namespace LexLeanR4.Object257NonemptyContinuation

public theorem obj257ContinuationUnrelatedZero : (0 = 0) := by
  rfl

public theorem obj257FirstContinuationLifted (count : Nat) : LexLeanR4.Object257NonemptyContext.Obj257FirstContinuationGoal (count) := by
  induction count with
  | zero =>
    exact LexLeanR4.Object257FirstAtomState.obj257FirstAtomState
  | succ prior ih =>
    cases prior with
    | zero =>
      congr 1
    | succ earlier =>
      congr 1

public theorem obj257FirstTwoAtomsContinuation : (LexLeanR4.R4.ObjectStore.ObjFoldAtoms (((0 :: (1 :: (2 :: (3 :: (4 :: (5 :: (6 :: (7 :: (8 :: (9 :: (10 :: (11 :: (12 :: (13 :: (14 :: (15 :: (16 :: (17 :: (18 :: (19 :: (20 :: (21 :: (22 :: (23 :: (24 :: (25 :: (26 :: (27 :: (28 :: (29 :: (30 :: (31 :: ([] : List (Nat)))))))))))))))))))))))))))))))))) :: ((32 :: (33 :: (34 :: (35 :: (36 :: (37 :: (38 :: (39 :: (40 :: (41 :: (42 :: (43 :: (44 :: (45 :: (46 :: (47 :: (48 :: (49 :: (50 :: (51 :: (52 :: (53 :: (54 :: (55 :: (56 :: (57 :: (58 :: (59 :: (60 :: (61 :: (62 :: (63 :: ([] : List (Nat)))))))))))))))))))))))))))))))))) :: ([] : List (List (Nat)))))) (([] : List (LexLeanR4.R4.ObjectTypes.ObjRoot))) (([] : List (LexLeanR4.R4.ObjectTypes.ObjRecord))) (({ objectBytes := 16777216, records := 1000000, recordBytes := 67108864, returnedBytes := 16777216, visits := 1000000 } : LexLeanR4.R4.ObjectTypes.ObjLimits)) = LexLeanR4.R4.ObjectStore.ObjFoldAtoms (((32 :: (33 :: (34 :: (35 :: (36 :: (37 :: (38 :: (39 :: (40 :: (41 :: (42 :: (43 :: (44 :: (45 :: (46 :: (47 :: (48 :: (49 :: (50 :: (51 :: (52 :: (53 :: (54 :: (55 :: (56 :: (57 :: (58 :: (59 :: (60 :: (61 :: (62 :: (63 :: ([] : List (Nat)))))))))))))))))))))))))))))))))) :: ([] : List (List (Nat))))) ((({ id := ({ bytes := (63 :: (164 :: (31 :: (162 :: (68 :: (35 :: (68 :: (195 :: (38 :: (116 :: (35 :: (81 :: ([] : List (Nat)))))))))))))) } : LexLeanR4.R4.ObjectTypes.ObjNodeId), scale := 0 } : LexLeanR4.R4.ObjectTypes.ObjRoot) :: ([] : List (LexLeanR4.R4.ObjectTypes.ObjRoot)))) ((({ id := ({ bytes := (63 :: (164 :: (31 :: (162 :: (68 :: (35 :: (68 :: (195 :: (38 :: (116 :: (35 :: (81 :: ([] : List (Nat)))))))))))))) } : LexLeanR4.R4.ObjectTypes.ObjNodeId), scale := 0, payload := (0 :: (1 :: (2 :: (3 :: (4 :: (5 :: (6 :: (7 :: (8 :: (9 :: (10 :: (11 :: (12 :: (13 :: (14 :: (15 :: (16 :: (17 :: (18 :: (19 :: (20 :: (21 :: (22 :: (23 :: (24 :: (25 :: (26 :: (27 :: (28 :: (29 :: (30 :: (31 :: ([] : List (Nat)))))))))))))))))))))))))))))))))) } : LexLeanR4.R4.ObjectTypes.ObjRecord) :: ([] : List (LexLeanR4.R4.ObjectTypes.ObjRecord)))) (({ objectBytes := 16777216, records := 1000000, recordBytes := 67108864, returnedBytes := 16777216, visits := 1000000 } : LexLeanR4.R4.ObjectTypes.ObjLimits))) := by
  exact obj257FirstContinuationLifted (1)

end LexLeanR4.Object257NonemptyContinuation
