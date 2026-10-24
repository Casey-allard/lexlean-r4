module
public import Init
public import LexLeanR4.R4.ObjectCodec
public import LexLeanR4.R4.ObjectTypes
set_option autoImplicit false
set_option maxRecDepth 100000
set_option maxHeartbeats 1000000000
set_option linter.constructorNameAsVariable false
namespace LexLeanR4.ObjectManifestVersion

public theorem objOriginalManifestVersion : (LexLeanR4.R4.ObjectCodec.ObjManifestVersion (({ length := 96, roots := (({ id := ({ bytes := (128 :: (174 :: (112 :: (205 :: (237 :: (99 :: (32 :: (90 :: (110 :: (198 :: (109 :: (200 :: ([] : List (Nat)))))))))))))) } : LexLeanR4.R4.ObjectTypes.ObjNodeId), scale := 1 } : LexLeanR4.R4.ObjectTypes.ObjRoot) :: ([] : List (LexLeanR4.R4.ObjectTypes.ObjRoot))), digest := ({ bytes := (201 :: (241 :: (165 :: (247 :: (157 :: (123 :: (234 :: (1 :: (165 :: (79 :: (78 :: (219 :: (65 :: (103 :: (55 :: (34 :: (246 :: (39 :: (238 :: (46 :: (130 :: (221 :: (163 :: (36 :: (148 :: (107 :: (99 :: (207 :: (75 :: (155 :: (22 :: (175 :: ([] : List (Nat)))))))))))))))))))))))))))))))))) } : LexLeanR4.R4.ObjectTypes.ObjWholeDigest) } : LexLeanR4.R4.ObjectTypes.ObjManifest)) (({ objectBytes := 16777216, records := 1000000, recordBytes := 67108864, returnedBytes := 16777216, visits := 1000000 } : LexLeanR4.R4.ObjectTypes.ObjLimits)) = Except.ok (({ bytes := (146 :: (74 :: (190 :: (232 :: (62 :: (91 :: (175 :: (190 :: (237 :: (157 :: (113 :: (38 :: (4 :: (8 :: (17 :: (5 :: (253 :: (233 :: (231 :: (187 :: (31 :: (113 :: (235 :: (100 :: (5 :: (194 :: (189 :: (181 :: (34 :: (231 :: (150 :: (142 :: ([] : List (Nat)))))))))))))))))))))))))))))))))) } : LexLeanR4.R4.ObjectTypes.ObjVersion))) := by
  rfl

end LexLeanR4.ObjectManifestVersion
