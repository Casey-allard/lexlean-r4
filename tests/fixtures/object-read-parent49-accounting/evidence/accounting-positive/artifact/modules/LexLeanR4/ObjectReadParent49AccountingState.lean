module
public import Init
public import LexLeanR4.R4.ObjectRead
public import LexLeanR4.R4.ObjectTypes
set_option autoImplicit false
set_option maxRecDepth 100000
set_option maxHeartbeats 1000000000
set_option linter.constructorNameAsVariable false
namespace LexLeanR4.ObjectReadParent49AccountingState

public theorem objParent49AccountingState : ((match LexLeanR4.R4.ObjectRead.ObjMarkVisit (({ output := ([] : List (Nat)), visits := 0, wireBytes := 0, seen := ([] : List (LexLeanR4.R4.ObjectTypes.ObjNodeId)) } : LexLeanR4.R4.ObjectRead.ObjWalkState)) (({ objectBytes := 16777216, records := 1000000, recordBytes := 67108864, returnedBytes := 16777216, visits := 1000000 } : LexLeanR4.R4.ObjectTypes.ObjLimits)) with | Except.error failure => Except.error (failure) | Except.ok counted => LexLeanR4.R4.ObjectRead.ObjAccountRecord (counted) (({ id := ({ bytes := (128 :: (174 :: (112 :: (205 :: (237 :: (99 :: (32 :: (90 :: (110 :: (198 :: (109 :: (200 :: ([] : List (Nat)))))))))))))) } : LexLeanR4.R4.ObjectTypes.ObjNodeId), scale := 1, payload := (204 :: (224 :: (130 :: (100 :: (35 :: (209 :: (207 :: (137 :: (18 :: (167 :: (134 :: (61 :: (229 :: (238 :: (245 :: (54 :: (216 :: (221 :: (107 :: (40 :: (188 :: (50 :: (60 :: (161 :: (125 :: (221 :: (203 :: (109 :: (110 :: (224 :: (251 :: (215 :: (105 :: (78 :: (221 :: (7 :: ([] : List (Nat)))))))))))))))))))))))))))))))))))))) } : LexLeanR4.R4.ObjectTypes.ObjRecord))) = Except.ok (({ output := ([] : List (Nat)), visits := 1, wireBytes := 49, seen := (({ bytes := (128 :: (174 :: (112 :: (205 :: (237 :: (99 :: (32 :: (90 :: (110 :: (198 :: (109 :: (200 :: ([] : List (Nat)))))))))))))) } : LexLeanR4.R4.ObjectTypes.ObjNodeId) :: ([] : List (LexLeanR4.R4.ObjectTypes.ObjNodeId))) } : LexLeanR4.R4.ObjectRead.ObjWalkState))) := by
  rfl

end LexLeanR4.ObjectReadParent49AccountingState
